//! Mint a fresh Nebula CA (NF-2.2).
//!
//! `mint_ca` shells out to `/usr/bin/nebula-cert ca` against a
//! tempdir, reads the resulting `ca.crt` + `ca.key` byte streams,
//! seals them to the persistent CA directory, and records the
//! epoch-0 row in the `nebula_ca` SQL table.
//!
//! Per the v2.5 Q3 lock the CA private key is sealed mode 0600
//! root-owned; the cert is mode 0644 so peers can read it without
//! root. The whole flow runs against a tempdir under
//! `MACKESD_NEBULA_CA_DIR` in tests — production resolves the dir
//! via [`super::seal::ca_dir`].

use std::path::PathBuf;
use std::process::Command;

use rusqlite::{params, Connection};

use super::error::{CaError, CaResult};
use super::seal::{ca_dir, seal};

/// On-disk path resolution + paired bytes produced by a successful
/// CA mint.
#[derive(Debug, Clone)]
pub struct CaArtifacts {
    /// Mesh id the CA was minted for.
    pub mesh_id: String,
    /// Epoch counter — `0` on initial mint, monotonically increasing
    /// on every leader-failover rotation.
    pub epoch: u32,
    /// Final on-disk path to the sealed public cert (mode 0644).
    pub ca_crt_path: PathBuf,
    /// Final on-disk path to the sealed private key (mode 0600).
    pub ca_key_path: PathBuf,
    /// PEM-encoded public cert bytes (also stored in `nebula_ca.ca_cert_pem`).
    pub ca_cert_pem: String,
}

/// Resolve the `nebula-cert` binary path. Reads
/// `MACKESD_NEBULA_CERT_BIN` first (test injection / operator
/// override), falls back to `/usr/bin/nebula-cert` (the Fedora
/// `nebula` package's canonical install path).
#[must_use]
pub fn nebula_cert_bin() -> PathBuf {
    if let Ok(p) = std::env::var("MACKESD_NEBULA_CERT_BIN") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    PathBuf::from("/usr/bin/nebula-cert")
}

/// Mint a fresh CA for `mesh_id` at `epoch=0`.
///
/// Reads the existing `nebula_ca` row first; if one exists for
/// `(mesh_id, epoch=0)`, the mint is idempotent — the returned
/// artifacts point at the already-sealed files and no subprocess
/// runs. Use [`super::epoch::bump_epoch`] for the rotation path
/// (NF-2.5).
///
/// # Errors
///
/// - [`CaError::NebulaCertMissing`] if the resolved binary path
///   doesn't exist.
/// - [`CaError::NebulaCertFailed`] if the subprocess exits non-zero.
/// - [`CaError::NebulaCertOutputMissing`] if `ca.crt` / `ca.key`
///   don't materialize in the tempdir.
/// - [`CaError::Io`] on any sealing / filesystem error.
/// - [`CaError::Sql`] on rusqlite failures.
pub fn mint_ca(conn: &Connection, mesh_id: &str) -> CaResult<CaArtifacts> {
    if mesh_id.is_empty() {
        return Err(CaError::Sql("mesh_id must not be empty".into()));
    }

    // NF-2.2 idempotency invariant: if epoch=0 already exists for
    // this mesh, return the on-disk artifact without re-shelling
    // to `nebula-cert`. Re-minting would generate a different key
    // pair and orphan every peer cert under the prior epoch.
    if let Some(existing) = load_existing_epoch_zero(conn, mesh_id)? {
        return Ok(existing);
    }

    let bin = nebula_cert_bin();
    if !bin.exists() {
        return Err(CaError::NebulaCertMissing(bin));
    }

    // Run `nebula-cert ca` inside a tempdir so we never collide
    // with a half-written sealed dir on failure.
    let tmp = tempfile::tempdir().map_err(CaError::Io)?;
    let output = Command::new(&bin)
        .arg("ca")
        .arg("-name")
        .arg(mesh_id)
        .arg("-duration")
        .arg("87600h0m0s")
        .current_dir(tmp.path())
        .output()
        .map_err(CaError::Io)?;

    if !output.status.success() {
        return Err(CaError::NebulaCertFailed {
            exit_status: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let crt_src = tmp.path().join("ca.crt");
    let key_src = tmp.path().join("ca.key");
    if !crt_src.exists() {
        return Err(CaError::NebulaCertOutputMissing(crt_src));
    }
    if !key_src.exists() {
        return Err(CaError::NebulaCertOutputMissing(key_src));
    }

    let crt_bytes = std::fs::read(&crt_src).map_err(CaError::Io)?;
    let key_bytes = std::fs::read(&key_src).map_err(CaError::Io)?;
    let ca_cert_pem = String::from_utf8(crt_bytes.clone())
        .map_err(|_| CaError::NebulaCertOutputMissing(crt_src))?;

    let dir = ca_dir();
    let ca_crt_path = dir.join("ca.crt");
    let ca_key_path = dir.join("ca.key");

    // Key first (more sensitive); cert second so an interrupted
    // mint leaves a coherent half-state (no cert without a key).
    seal(&ca_key_path, &key_bytes, 0o600)?;
    seal(&ca_crt_path, &crt_bytes, 0o644)?;

    let now_ms = unix_now_ms();
    conn.execute(
        "INSERT INTO nebula_ca (mesh_id, epoch, ca_cert_pem, created_at) \
         VALUES (?1, 0, ?2, ?3)",
        params![mesh_id, ca_cert_pem, now_ms],
    )?;

    Ok(CaArtifacts {
        mesh_id: mesh_id.to_owned(),
        epoch: 0,
        ca_crt_path,
        ca_key_path,
        ca_cert_pem,
    })
}

/// Look up an existing epoch-0 row for the named mesh and return its
/// `CaArtifacts` view, reading the sealed files from disk so the
/// caller always sees a coherent (row, file) pair.
fn load_existing_epoch_zero(conn: &Connection, mesh_id: &str) -> CaResult<Option<CaArtifacts>> {
    let pem: Option<String> = conn
        .query_row(
            "SELECT ca_cert_pem FROM nebula_ca \
             WHERE mesh_id = ?1 AND epoch = 0",
            params![mesh_id],
            |row| row.get::<_, String>(0),
        )
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(CaError::from(other)),
        })?;
    let Some(pem) = pem else { return Ok(None) };
    let dir = ca_dir();
    let ca_crt_path = dir.join("ca.crt");
    let ca_key_path = dir.join("ca.key");
    Ok(Some(CaArtifacts {
        mesh_id: mesh_id.to_owned(),
        epoch: 0,
        ca_crt_path,
        ca_key_path,
        ca_cert_pem: pem,
    }))
}

/// Unix epoch ms — wrapped so tests can stub without pulling in
/// `chrono::Utc::now`.
#[must_use]
pub fn unix_now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(0))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    /// Drop a fake `nebula-cert` shim into a tempdir that writes
    /// deterministic `ca.crt` + `ca.key` payloads, then return the
    /// path to it. Lets the mint flow exercise the full subprocess
    /// path without needing the real binary installed.
    fn install_fake_nebula_cert(dir: &TempDir, key_bytes: &[u8], crt_bytes: &[u8]) -> PathBuf {
        let shim_path = dir.path().join("nebula-cert");
        let script = format!(
            "#!/bin/sh\n\
             cat > ca.crt <<'__CA_CRT__'\n{crt}\n__CA_CRT__\n\
             cat > ca.key <<'__CA_KEY__'\n{key}\n__CA_KEY__\n",
            crt = std::str::from_utf8(crt_bytes).unwrap(),
            key = std::str::from_utf8(key_bytes).unwrap(),
        );
        std::fs::write(&shim_path, script).unwrap();
        let mut perms = std::fs::metadata(&shim_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&shim_path, perms).unwrap();
        shim_path
    }

    fn open_in_memory_with_migrations() -> Connection {
        crate::store::open_in_memory().expect("open in-memory")
    }

    /// Wraps a test in a section that owns the env vars
    /// `MACKESD_NEBULA_CA_DIR` + `MACKESD_NEBULA_CERT_BIN`. Each
    /// invocation generates fresh tempdirs + paths so the global
    /// process env doesn't leak across tests run by cargo test's
    /// thread pool.
    struct EnvGuard {
        _shim_dir: TempDir,
        _ca_dir: TempDir,
        prev_ca: Option<String>,
        prev_bin: Option<String>,
    }

    impl EnvGuard {
        fn install(key_bytes: &[u8], crt_bytes: &[u8]) -> Self {
            let shim_dir = tempfile::tempdir().unwrap();
            let ca_dir = tempfile::tempdir().unwrap();
            let bin = install_fake_nebula_cert(&shim_dir, key_bytes, crt_bytes);
            let prev_ca = std::env::var("MACKESD_NEBULA_CA_DIR").ok();
            let prev_bin = std::env::var("MACKESD_NEBULA_CERT_BIN").ok();
            std::env::set_var("MACKESD_NEBULA_CA_DIR", ca_dir.path());
            std::env::set_var("MACKESD_NEBULA_CERT_BIN", bin);
            Self {
                _shim_dir: shim_dir,
                _ca_dir: ca_dir,
                prev_ca,
                prev_bin,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev_ca {
                Some(v) => std::env::set_var("MACKESD_NEBULA_CA_DIR", v),
                None => std::env::remove_var("MACKESD_NEBULA_CA_DIR"),
            }
            match &self.prev_bin {
                Some(v) => std::env::set_var("MACKESD_NEBULA_CERT_BIN", v),
                None => std::env::remove_var("MACKESD_NEBULA_CERT_BIN"),
            }
        }
    }

    /// Serialize tests that touch the process-wide env. cargo
    /// test threads share one process so concurrent set/remove
    /// races without this lock.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn mint_ca_round_trips_through_disk_and_sql() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let _env = EnvGuard::install(b"FAKE-KEY-BYTES", b"FAKE-CRT-BYTES");
        let conn = open_in_memory_with_migrations();
        let art = mint_ca(&conn, "test-mesh-001").expect("mint");
        assert_eq!(art.epoch, 0);
        assert_eq!(art.mesh_id, "test-mesh-001");
        // CA cert PEM ends with the placeholder we wrote.
        assert!(art.ca_cert_pem.contains("FAKE-CRT-BYTES"));
        // Files exist on disk.
        assert!(art.ca_crt_path.exists());
        assert!(art.ca_key_path.exists());
    }

    #[test]
    fn mint_ca_seals_key_at_0600_and_cert_at_0644() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let _env = EnvGuard::install(b"K", b"C");
        let conn = open_in_memory_with_migrations();
        let art = mint_ca(&conn, "test-mesh-mode-bits").expect("mint");
        let key_mode = std::fs::metadata(&art.ca_key_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o7777;
        let crt_mode = std::fs::metadata(&art.ca_crt_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o7777;
        assert_eq!(key_mode, 0o600);
        assert_eq!(crt_mode, 0o644);
    }

    #[test]
    fn mint_ca_inserts_one_row_at_epoch_zero() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let _env = EnvGuard::install(b"K", b"C");
        let conn = open_in_memory_with_migrations();
        mint_ca(&conn, "test-mesh-row").expect("mint");
        let (epoch, count): (i64, i64) = conn
            .query_row(
                "SELECT epoch, COUNT(*) FROM nebula_ca WHERE mesh_id = ?1",
                params!["test-mesh-row"],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(count, 1, "exactly one row at epoch 0");
        assert_eq!(epoch, 0);
    }

    #[test]
    fn mint_ca_is_idempotent_on_re_mint() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let _env = EnvGuard::install(b"K", b"C");
        let conn = open_in_memory_with_migrations();
        let a = mint_ca(&conn, "test-mesh-idem").expect("first mint");
        // Re-running must not insert a second row or change the
        // sealed files.
        let b = mint_ca(&conn, "test-mesh-idem").expect("second mint");
        assert_eq!(a.ca_cert_pem, b.ca_cert_pem);
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM nebula_ca WHERE mesh_id = ?1",
                params!["test-mesh-idem"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "re-mint must not duplicate rows");
    }

    #[test]
    fn mint_ca_records_epoch_zero_invariant() {
        // The lock says epoch=0 is the initial mint. Anything else
        // is the rotation path (NF-2.5).
        let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let _env = EnvGuard::install(b"K", b"C");
        let conn = open_in_memory_with_migrations();
        let art = mint_ca(&conn, "test-mesh-epoch").expect("mint");
        assert_eq!(art.epoch, 0, "initial mint must record epoch=0");
        let stored_epoch: i64 = conn
            .query_row(
                "SELECT epoch FROM nebula_ca WHERE mesh_id = ?1 ORDER BY epoch ASC LIMIT 1",
                params!["test-mesh-epoch"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored_epoch, 0);
    }

    #[test]
    fn mint_ca_errors_when_nebula_cert_missing() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        // Point at a nonexistent binary.
        let ca_dir = tempfile::tempdir().unwrap();
        let prev_ca = std::env::var("MACKESD_NEBULA_CA_DIR").ok();
        let prev_bin = std::env::var("MACKESD_NEBULA_CERT_BIN").ok();
        std::env::set_var("MACKESD_NEBULA_CA_DIR", ca_dir.path());
        std::env::set_var(
            "MACKESD_NEBULA_CERT_BIN",
            "/nonexistent/nebula-cert-binary-zzz",
        );
        let conn = open_in_memory_with_migrations();
        let err = mint_ca(&conn, "test-mesh-no-bin").expect_err("must fail");
        assert!(
            matches!(err, CaError::NebulaCertMissing(_)),
            "expected NebulaCertMissing, got {err:?}"
        );
        match prev_ca {
            Some(v) => std::env::set_var("MACKESD_NEBULA_CA_DIR", v),
            None => std::env::remove_var("MACKESD_NEBULA_CA_DIR"),
        }
        match prev_bin {
            Some(v) => std::env::set_var("MACKESD_NEBULA_CERT_BIN", v),
            None => std::env::remove_var("MACKESD_NEBULA_CERT_BIN"),
        }
    }
}
