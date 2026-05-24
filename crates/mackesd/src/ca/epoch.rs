//! CA epoch counter + rotation (NF-2.5).
//!
//! Called from the leader-election path (`leader.rs`) when the new
//! leader detects an expired lease. The flow:
//!
//! 1. Atomically `UPDATE nebula_ca SET retired_at = now() WHERE
//!    mesh_id = ?1 AND retired_at IS NULL` so every prior epoch is
//!    marked retired in the same transaction.
//! 2. Re-mint a fresh CA via [`super::mint::mint_ca`]'s subprocess
//!    path (writes the sealed key + cert to disk + inserts the
//!    new `nebula_ca` row at `max_epoch + 1`).
//! 3. Re-sign every active peer cert under the new CA epoch.
//! 4. Emit a hash-chained `Lifecycle` audit event so the chain
//!    captures the rotation (consumed by `mackes audit verify`).

use std::path::PathBuf;
use std::process::Command;

use rusqlite::{params, Connection};

use super::error::{CaError, CaResult};
use super::mint::{nebula_cert_bin, unix_now_ms};
use super::seal::ca_dir;
use super::sign::list_active_peer_certs;

/// Result of a single rotation: the new epoch + count of peer
/// certs re-signed.
#[derive(Debug, Clone)]
pub struct EpochBump {
    /// New active epoch (one greater than the previously-active
    /// epoch).
    pub new_epoch: u32,
    /// Number of peer-cert rows re-signed under `new_epoch`.
    pub peers_resigned: usize,
    /// PEM of the newly-minted CA cert (handy for the audit event
    /// payload + the operator CLI).
    pub new_ca_cert_pem: String,
}

/// Bump the CA epoch for `mesh_id`, re-mint the CA, and re-sign
/// every active peer cert under the new epoch.
///
/// This is the rotation path called from the leader-election win
/// transition. It is NOT idempotent on its own — every call bumps
/// the epoch by one. The caller (leader.rs) gates calls on
/// "this node is now the leader AND the previous lease expired
/// without renewal."
///
/// # Errors
///
/// - [`CaError::MeshNotFound`] if no prior `nebula_ca` row exists
///   for `mesh_id`. The caller must run [`super::mint::mint_ca`]
///   before the first rotation.
/// - [`CaError::NebulaCertMissing`] / [`CaError::NebulaCertFailed`]
///   when the subprocess path fails.
/// - [`CaError::Io`] on filesystem errors during seal.
/// - [`CaError::Sql`] on rusqlite failures.
pub fn bump_epoch(conn: &mut Connection, mesh_id: &str) -> CaResult<EpochBump> {
    if mesh_id.is_empty() {
        return Err(CaError::Sql("mesh_id must not be empty".into()));
    }
    let bin = nebula_cert_bin();
    if !bin.exists() {
        return Err(CaError::NebulaCertMissing(bin));
    }

    // 1+2 — retire prior epochs + mint the fresh CA against a
    // tempdir. The retire UPDATE + the new INSERT live in one SQL
    // transaction so a crash between them can never leave the
    // mesh without an active row.
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
    let new_ca_cert_pem = String::from_utf8(crt_bytes.clone())
        .map_err(|_| CaError::NebulaCertOutputMissing(crt_src.clone()))?;

    let now_ms = unix_now_ms();
    let new_epoch = retire_and_insert(conn, mesh_id, &new_ca_cert_pem, now_ms)?;

    // 1+2 (disk side) — seal the new CA over the old one. After
    // this point the next sign call will pick up the new key.
    super::seal::seal(&ca_dir().join("ca.key"), &key_bytes, 0o600)?;
    super::seal::seal(&ca_dir().join("ca.crt"), &crt_bytes, 0o644)?;

    // 3 — re-sign every active peer cert under the new epoch.
    let active = list_active_peer_certs(conn)?;
    let peers_resigned = resign_active_certs(conn, &active, new_epoch)?;

    // 4 — hash-chained audit event. The payload is JSON-shaped so
    // it slots into the existing events.payload_json column.
    let payload = serde_json::json!({
        "event":           "nebula_ca_rotation",
        "mesh_id":         mesh_id,
        "new_epoch":       new_epoch,
        "peers_resigned":  peers_resigned,
        "new_ca_cert_pem": new_ca_cert_pem,
    })
    .to_string();
    let actor = format!("ca/epoch:{mesh_id}");
    // Best-effort — if the audit insert fails the rotation has
    // already landed; we surface the error so the caller can log
    // it but don't unwind the rotation.
    if let Err(e) = crate::store::insert_event(conn, "lifecycle", &actor, &payload) {
        return Err(CaError::Sql(format!(
            "rotation succeeded but audit insert failed: {e}"
        )));
    }

    Ok(EpochBump {
        new_epoch,
        peers_resigned,
        new_ca_cert_pem,
    })
}

/// Atomic: retire every prior un-retired row, then insert a new
/// row at `max_epoch + 1`. Returns the inserted epoch.
fn retire_and_insert(
    conn: &mut Connection,
    mesh_id: &str,
    new_ca_cert_pem: &str,
    now_ms: i64,
) -> CaResult<u32> {
    let tx = conn.transaction().map_err(CaError::from)?;
    // Compute next epoch under the same transaction. The
    // `nebula_ca` table's PK includes `epoch` so a second writer
    // trying to insert at the same value would trip UNIQUE; this
    // path runs from the singleton leader so concurrent rotation
    // is structurally impossible.
    let prior_max: Option<i64> = tx
        .query_row(
            "SELECT MAX(epoch) FROM nebula_ca WHERE mesh_id = ?1",
            params![mesh_id],
            |r| r.get::<_, Option<i64>>(0),
        )
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(CaError::from(other)),
        })?;
    let prior_max = prior_max.ok_or_else(|| CaError::MeshNotFound {
        mesh_id: mesh_id.to_owned(),
    })?;

    let new_epoch_i64 = prior_max
        .checked_add(1)
        .ok_or_else(|| CaError::Sql("epoch overflow".into()))?;
    let new_epoch = u32::try_from(new_epoch_i64)
        .map_err(|_| CaError::Sql(format!("epoch {new_epoch_i64} doesn't fit in u32")))?;

    tx.execute(
        "UPDATE nebula_ca SET retired_at = ?1 \
         WHERE mesh_id = ?2 AND retired_at IS NULL",
        params![now_ms, mesh_id],
    )
    .map_err(CaError::from)?;

    tx.execute(
        "INSERT INTO nebula_ca (mesh_id, epoch, ca_cert_pem, created_at) \
         VALUES (?1, ?2, ?3, ?4)",
        params![mesh_id, new_epoch_i64, new_ca_cert_pem, now_ms],
    )
    .map_err(CaError::from)?;

    tx.commit().map_err(CaError::from)?;
    Ok(new_epoch)
}

/// Re-sign every active peer cert under the new epoch. For each
/// active row the prior row is left in place (audit history) and
/// a fresh row is inserted at the new epoch — same overlay IP,
/// same node id, fresh cert PEM.
///
/// Per NF-2.3 the actual cryptographic re-sign happens via
/// `nebula-cert sign`. We mirror that subprocess shape here.
fn resign_active_certs(
    conn: &Connection,
    active: &[(String, u32, String, String)],
    new_epoch: u32,
) -> CaResult<usize> {
    if active.is_empty() {
        return Ok(0);
    }
    let bin = nebula_cert_bin();
    let dir = ca_dir();
    let ca_crt = dir.join("ca.crt");
    let ca_key = dir.join("ca.key");
    let now_ms = unix_now_ms();
    let expires_ms = now_ms.saturating_add(10 * 365 * 24 * 60 * 60 * 1000);
    // The v2.5 design lock fixes the mesh CIDR at 10.42.0.0/16
    // (Q3); we re-use the default for the rotation prefix so the
    // `<ip>/<prefix>` shape `nebula-cert sign` wants matches the
    // initial mint.
    let prefix = super::sign::default_mesh_cidr().prefix_len();

    let mut count = 0usize;
    for (node_id, _prior_epoch, _prior_pem, overlay_ip) in active {
        let tmp = tempfile::tempdir().map_err(CaError::Io)?;
        let output = Command::new(&bin)
            .arg("sign")
            .arg("-name")
            .arg(node_id)
            .arg("-ip")
            .arg(format!("{overlay_ip}/{prefix}"))
            .arg("-ca-crt")
            .arg(&ca_crt)
            .arg("-ca-key")
            .arg(&ca_key)
            .current_dir(tmp.path())
            .output()
            .map_err(CaError::Io)?;
        if !output.status.success() {
            return Err(CaError::NebulaCertFailed {
                exit_status: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        let crt_src = tmp.path().join(format!("{node_id}.crt"));
        if !crt_src.exists() {
            return Err(CaError::NebulaCertOutputMissing(crt_src));
        }
        let cert_pem = std::fs::read_to_string(&crt_src).map_err(CaError::Io)?;
        conn.execute(
            "INSERT INTO nebula_peer_certs \
             (node_id, epoch, cert_pem, overlay_ip, created_at, expires_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(node_id, epoch) DO UPDATE SET \
                cert_pem   = excluded.cert_pem, \
                overlay_ip = excluded.overlay_ip, \
                created_at = excluded.created_at, \
                expires_at = excluded.expires_at, \
                revoked_at = NULL",
            params![
                node_id,
                i64::from(new_epoch),
                cert_pem,
                overlay_ip,
                now_ms,
                expires_ms,
            ],
        )
        .map_err(CaError::from)?;
        count += 1;
    }
    Ok(count)
}

/// Read the current active epoch for `mesh_id`. Convenience helper
/// for the rotate-CLI subcommand.
///
/// # Errors
///
/// Returns [`CaError::MeshNotFound`] when no `nebula_ca` row exists.
pub fn current_epoch(conn: &Connection, mesh_id: &str) -> CaResult<u32> {
    let row: Option<i64> = conn
        .query_row(
            "SELECT epoch FROM nebula_ca \
             WHERE mesh_id = ?1 AND retired_at IS NULL \
             ORDER BY epoch DESC LIMIT 1",
            params![mesh_id],
            |r| r.get::<_, i64>(0),
        )
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(CaError::from(other)),
        })?;
    let row = row.ok_or_else(|| CaError::MeshNotFound {
        mesh_id: mesh_id.to_owned(),
    })?;
    u32::try_from(row).map_err(|_| CaError::Sql(format!("epoch {row} doesn't fit in u32")))
}

/// Path of the sealed CA cert on disk. Convenience re-export.
#[must_use]
pub fn sealed_cert_path() -> PathBuf {
    ca_dir().join("ca.crt")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_in_memory_with_migrations() -> Connection {
        crate::store::open_in_memory().expect("open in-memory")
    }

    #[test]
    fn retire_and_insert_advances_epoch_and_retires_prior() {
        let mut conn = open_in_memory_with_migrations();
        conn.execute(
            "INSERT INTO nebula_ca (mesh_id, epoch, ca_cert_pem, created_at) \
             VALUES ('m', 0, 'PEM-A', 100)",
            [],
        )
        .unwrap();
        let new = retire_and_insert(&mut conn, "m", "PEM-B", 200).unwrap();
        assert_eq!(new, 1);
        // Prior epoch is retired.
        let retired: i64 = conn
            .query_row(
                "SELECT retired_at FROM nebula_ca WHERE mesh_id = 'm' AND epoch = 0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(retired, 200);
        // New epoch row exists.
        let new_pem: String = conn
            .query_row(
                "SELECT ca_cert_pem FROM nebula_ca WHERE mesh_id = 'm' AND epoch = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(new_pem, "PEM-B");
    }

    #[test]
    fn retire_and_insert_errors_when_mesh_unknown() {
        let mut conn = open_in_memory_with_migrations();
        let err = retire_and_insert(&mut conn, "missing", "PEM", 100).expect_err("must fail");
        assert!(matches!(err, CaError::MeshNotFound { .. }));
    }

    #[test]
    fn current_epoch_returns_active_row() {
        let conn = open_in_memory_with_migrations();
        conn.execute(
            "INSERT INTO nebula_ca (mesh_id, epoch, ca_cert_pem, created_at, retired_at) \
             VALUES ('m', 0, 'A', 100, 200)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO nebula_ca (mesh_id, epoch, ca_cert_pem, created_at) \
             VALUES ('m', 1, 'B', 200)",
            [],
        )
        .unwrap();
        let e = current_epoch(&conn, "m").expect("active");
        assert_eq!(e, 1);
    }
}
