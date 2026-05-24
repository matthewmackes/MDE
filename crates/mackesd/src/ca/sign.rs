//! Sign peer host certs against the sealed CA (NF-2.3).
//!
//! Allocates an overlay IP from the mesh CIDR, shells out to
//! `nebula-cert sign` against the sealed CA key, and persists the
//! signed cert PEM + overlay IP in `nebula_peer_certs`.
//!
//! Ed25519 verification on the inbound enrollment payload is the
//! authoritative gate — a forged `node_id`/`public_key` pair never
//! reaches the signing step.

use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::process::Command;

use ed25519_dalek::{Signature, VerifyingKey};
use rusqlite::{params, Connection};

use super::error::{CaError, CaResult};
use super::ipv4_net::Ipv4Network;
use super::mint::{nebula_cert_bin, unix_now_ms};
use super::seal::ca_dir;

/// Outcome of a successful peer-cert sign.
#[derive(Debug, Clone)]
pub struct SignedCert {
    /// Stable peer node id (mirrors `nodes.node_id`).
    pub node_id: String,
    /// CA epoch the cert was signed under.
    pub epoch: u32,
    /// PEM-encoded peer host cert.
    pub cert_pem: String,
    /// PEM-encoded peer host key (kept on the leader so the peer
    /// can fetch its own keypair via the bundle writer).
    pub key_pem: String,
    /// Allocated overlay IP (host-only, without the /24 suffix).
    pub overlay_ip: Ipv4Addr,
    /// Mesh CIDR the IP was allocated from.
    pub mesh_cidr: Ipv4Network,
}

/// Default mesh CIDR per the v2.5 design lock (Q3).
///
/// # Panics
///
/// Cannot panic in practice — `10.42.0.0/16` is a compile-time
/// valid CIDR. The `.expect` is here only because the
/// `Ipv4Network::new` constructor returns `Result` for non-const
/// callers; the precondition `prefix_len <= 32` is statically
/// satisfied (`16 <= 32`).
#[must_use]
pub fn default_mesh_cidr() -> Ipv4Network {
    Ipv4Network::new(Ipv4Addr::new(10, 42, 0, 0), 16)
        .expect("10.42.0.0/16 is a valid CIDR (16 <= 32)")
}

/// Verify a peer's Ed25519 signature over an enrollment payload.
///
/// `public_key` is the 32-byte VerifyingKey bytes the peer
/// published; `payload` is whatever payload bytes were signed
/// (typically `EnrollmentRequest::public_key_hex || hw_fingerprint`
/// — the caller decides). `signature_bytes` is the raw 64-byte
/// Ed25519 signature.
///
/// # Errors
///
/// Returns [`CaError::InvalidSignature`] for any failure (bad key
/// bytes, bad signature bytes, verification mismatch).
pub fn verify_enrollment_signature(
    public_key: &[u8],
    payload: &[u8],
    signature_bytes: &[u8],
) -> CaResult<()> {
    let pk_bytes: [u8; 32] = public_key
        .try_into()
        .map_err(|_| CaError::InvalidSignature)?;
    let verifying_key =
        VerifyingKey::from_bytes(&pk_bytes).map_err(|_| CaError::InvalidSignature)?;
    let sig_bytes: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| CaError::InvalidSignature)?;
    let signature = Signature::from_bytes(&sig_bytes);
    if crate::identity::verify(&verifying_key, payload, &signature) {
        Ok(())
    } else {
        Err(CaError::InvalidSignature)
    }
}

/// Sign a fresh peer host cert.
///
/// Workflow:
///
/// 1. Verify the Ed25519 signature on the enrollment payload
///    (`public_key || node_id`).
/// 2. Allocate an overlay IP from `mesh_cidr` that's not already
///    claimed in `nebula_peer_certs`.
/// 3. Shell out to `nebula-cert sign -name <node_id> -ip
///    <overlay_ip>/<prefix> -groups role:<role> -ca-crt <ca.crt>
///    -ca-key <ca.key>` against the sealed CA in `MACKESD_NEBULA_CA_DIR`.
/// 4. Read the resulting `<node_id>.crt` + `<node_id>.key` bytes
///    out of the tempdir.
/// 5. Insert one row into `nebula_peer_certs` at the active epoch.
///
/// # Errors
///
/// See [`CaError`] variants. `InvalidSignature`, `NoOverlayAddressAvailable`,
/// `NebulaCertMissing`, `NebulaCertFailed`, `NebulaCertOutputMissing`,
/// `Io`, and `Sql` are the load-bearing cases.
#[allow(clippy::too_many_arguments)]
pub fn sign_peer_cert(
    conn: &Connection,
    mesh_id: &str,
    node_id: &str,
    role: &str,
    mesh_cidr: Ipv4Network,
    public_key: &[u8],
    signed_payload: &[u8],
    signature: &[u8],
) -> CaResult<SignedCert> {
    if node_id.is_empty() {
        return Err(CaError::Sql("node_id must not be empty".into()));
    }
    let role_groups = match role {
        "host" | "peer" => format!("role:{role}"),
        // Reject roles outside the locked-2026-05-19 enum to keep
        // the Nebula `groups` list inside the schema the panel
        // renders.
        _ => return Err(CaError::Sql(format!("unknown role: {role}"))),
    };

    verify_enrollment_signature(public_key, signed_payload, signature)?;

    let epoch = active_epoch(conn, mesh_id)?;
    let overlay_ip = allocate_overlay_ip(conn, mesh_cidr)?;

    let bin = nebula_cert_bin();
    if !bin.exists() {
        return Err(CaError::NebulaCertMissing(bin));
    }

    let dir = ca_dir();
    let ca_crt = dir.join("ca.crt");
    let ca_key = dir.join("ca.key");
    if !ca_crt.exists() {
        return Err(CaError::NebulaCertOutputMissing(ca_crt));
    }
    if !ca_key.exists() {
        return Err(CaError::NebulaCertOutputMissing(ca_key));
    }

    let tmp = tempfile::tempdir().map_err(CaError::Io)?;
    let ip_arg = format!("{overlay_ip}/{}", mesh_cidr.prefix_len());

    let output = Command::new(&bin)
        .arg("sign")
        .arg("-name")
        .arg(node_id)
        .arg("-ip")
        .arg(&ip_arg)
        .arg("-groups")
        .arg(&role_groups)
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

    let crt_path = tmp.path().join(format!("{node_id}.crt"));
    let key_path = tmp.path().join(format!("{node_id}.key"));
    if !crt_path.exists() {
        return Err(CaError::NebulaCertOutputMissing(crt_path));
    }
    if !key_path.exists() {
        return Err(CaError::NebulaCertOutputMissing(key_path));
    }
    let cert_pem = std::fs::read_to_string(&crt_path).map_err(CaError::Io)?;
    let key_pem = std::fs::read_to_string(&key_path).map_err(CaError::Io)?;

    // 10-year expiry mirrors the CA duration. Stored as epoch ms.
    let now_ms = unix_now_ms();
    let expires_ms = now_ms.saturating_add(10 * 365 * 24 * 60 * 60 * 1000);
    conn.execute(
        "INSERT INTO nebula_peer_certs \
         (node_id, epoch, cert_pem, overlay_ip, created_at, expires_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            node_id,
            i64::from(epoch),
            cert_pem,
            overlay_ip.to_string(),
            now_ms,
            expires_ms,
        ],
    )?;

    Ok(SignedCert {
        node_id: node_id.to_owned(),
        epoch,
        cert_pem,
        key_pem,
        overlay_ip,
        mesh_cidr,
    })
}

/// Return the active CA epoch for `mesh_id`. Reads the
/// most-recent un-retired row from `nebula_ca`.
fn active_epoch(conn: &Connection, mesh_id: &str) -> CaResult<u32> {
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

/// Allocate the next available overlay IP from `cidr` that isn't
/// already in the `nebula_peer_certs` table.
fn allocate_overlay_ip(conn: &Connection, cidr: Ipv4Network) -> CaResult<Ipv4Addr> {
    // Build a set of taken IPs (in-memory; the 16-peer cap keeps
    // the set tiny).
    let mut taken = std::collections::HashSet::<String>::new();
    {
        let mut stmt = conn.prepare(
            "SELECT overlay_ip FROM nebula_peer_certs \
             WHERE revoked_at IS NULL",
        )?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        for row in rows {
            taken.insert(row?);
        }
    }
    for candidate in cidr.hosts() {
        if !taken.contains(&candidate.to_string()) {
            return Ok(candidate);
        }
    }
    Err(CaError::NoOverlayAddressAvailable {
        cidr: cidr.to_string(),
    })
}

/// Return every active (`revoked_at IS NULL`) peer cert. Used by
/// the rotation path (NF-2.5).
///
/// # Errors
///
/// Returns [`CaError::Sql`] on any rusqlite failure.
pub fn list_active_peer_certs(conn: &Connection) -> CaResult<Vec<(String, u32, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT node_id, epoch, cert_pem, overlay_ip FROM nebula_peer_certs \
         WHERE revoked_at IS NULL \
         ORDER BY node_id ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        let node_id: String = r.get(0)?;
        let epoch: i64 = r.get(1)?;
        let cert_pem: String = r.get(2)?;
        let overlay_ip: String = r.get(3)?;
        Ok((node_id, epoch, cert_pem, overlay_ip))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (node_id, epoch_i64, cert_pem, overlay_ip) = row?;
        let epoch = u32::try_from(epoch_i64)
            .map_err(|_| CaError::Sql(format!("epoch {epoch_i64} doesn't fit in u32")))?;
        out.push((node_id, epoch, cert_pem, overlay_ip));
    }
    Ok(out)
}

/// Read the path of the sealed CA key (so callers don't have to
/// stitch `ca_dir() / ca.key` themselves).
#[must_use]
pub fn sealed_ca_key_path() -> PathBuf {
    ca_dir().join("ca.key")
}

/// Read the path of the sealed CA cert.
#[must_use]
pub fn sealed_ca_cert_path() -> PathBuf {
    ca_dir().join("ca.crt")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::NodeKey;

    fn open_in_memory_with_migrations() -> Connection {
        crate::store::open_in_memory().expect("open in-memory")
    }

    #[test]
    fn verify_enrollment_signature_accepts_genuine_sig() {
        let key = NodeKey::generate();
        let payload = b"node_id:peer:anvil";
        let sig = key.sign(payload);
        let pk = key.verifying_key();
        verify_enrollment_signature(pk.as_bytes(), payload, &sig.to_bytes()).expect("must verify");
    }

    #[test]
    fn verify_enrollment_signature_rejects_tampered_payload() {
        let key = NodeKey::generate();
        let sig = key.sign(b"original");
        let pk = key.verifying_key();
        let res = verify_enrollment_signature(pk.as_bytes(), b"tampered", &sig.to_bytes());
        assert!(matches!(res, Err(CaError::InvalidSignature)));
    }

    #[test]
    fn verify_enrollment_signature_rejects_bad_key_length() {
        let res = verify_enrollment_signature(b"too-short", b"payload", &[0u8; 64]);
        assert!(matches!(res, Err(CaError::InvalidSignature)));
    }

    #[test]
    fn default_mesh_cidr_is_10_42_slash_16() {
        let cidr = default_mesh_cidr();
        assert_eq!(cidr.network(), Ipv4Addr::new(10, 42, 0, 0));
        assert_eq!(cidr.prefix_len(), 16);
    }

    #[test]
    fn allocate_overlay_ip_returns_first_unused() {
        let conn = open_in_memory_with_migrations();
        let cidr = default_mesh_cidr();
        // No peer certs yet — first allocation is .0.1.
        let ip = allocate_overlay_ip(&conn, cidr).expect("alloc");
        assert_eq!(ip, Ipv4Addr::new(10, 42, 0, 1));
    }

    #[test]
    fn allocate_overlay_ip_skips_taken_rows() {
        let conn = open_in_memory_with_migrations();
        conn.execute(
            "INSERT INTO nebula_peer_certs (node_id, epoch, cert_pem, overlay_ip, created_at, expires_at) \
             VALUES (?1, 0, ?2, ?3, 0, 0)",
            params!["peer:a", "PEM", "10.42.0.1"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO nebula_peer_certs (node_id, epoch, cert_pem, overlay_ip, created_at, expires_at) \
             VALUES (?1, 0, ?2, ?3, 0, 0)",
            params!["peer:b", "PEM", "10.42.0.2"],
        )
        .unwrap();
        let cidr = default_mesh_cidr();
        let ip = allocate_overlay_ip(&conn, cidr).expect("alloc");
        assert_eq!(ip, Ipv4Addr::new(10, 42, 0, 3));
    }

    #[test]
    fn allocate_overlay_ip_treats_revoked_as_free() {
        let conn = open_in_memory_with_migrations();
        conn.execute(
            "INSERT INTO nebula_peer_certs (node_id, epoch, cert_pem, overlay_ip, created_at, expires_at, revoked_at) \
             VALUES (?1, 0, ?2, ?3, 0, 0, 100)",
            params!["peer:gone", "PEM", "10.42.0.1"],
        )
        .unwrap();
        let cidr = default_mesh_cidr();
        let ip = allocate_overlay_ip(&conn, cidr).expect("alloc");
        // Revoked row at .1 is freed; allocator returns .1 again.
        assert_eq!(ip, Ipv4Addr::new(10, 42, 0, 1));
    }

    #[test]
    fn active_epoch_returns_highest_unretired() {
        let conn = open_in_memory_with_migrations();
        conn.execute(
            "INSERT INTO nebula_ca (mesh_id, epoch, ca_cert_pem, created_at) \
             VALUES (?1, 0, 'PEM-A', 100)",
            params!["m"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO nebula_ca (mesh_id, epoch, ca_cert_pem, created_at, retired_at) \
             VALUES (?1, 1, 'PEM-B', 200, 300)",
            params!["m"],
        )
        .unwrap();
        // The retired epoch is ignored — only the active row counts.
        let e = active_epoch(&conn, "m").expect("epoch");
        assert_eq!(e, 0);
    }

    #[test]
    fn active_epoch_errors_when_mesh_unknown() {
        let conn = open_in_memory_with_migrations();
        let err = active_epoch(&conn, "absent").expect_err("must error");
        assert!(matches!(err, CaError::MeshNotFound { .. }));
    }

    #[test]
    fn list_active_peer_certs_filters_revoked() {
        let conn = open_in_memory_with_migrations();
        conn.execute(
            "INSERT INTO nebula_peer_certs (node_id, epoch, cert_pem, overlay_ip, created_at, expires_at) \
             VALUES ('peer:a', 0, 'PEM-A', '10.42.0.1', 0, 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO nebula_peer_certs (node_id, epoch, cert_pem, overlay_ip, created_at, expires_at, revoked_at) \
             VALUES ('peer:b', 0, 'PEM-B', '10.42.0.2', 0, 0, 50)",
            [],
        )
        .unwrap();
        let active = list_active_peer_certs(&conn).expect("list");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].0, "peer:a");
    }
}
