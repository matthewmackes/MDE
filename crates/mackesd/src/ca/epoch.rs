//! NF-2.5 (v2.5) — CA epoch rotation.
//!
//! Called when a new leader takes the lease — the prior CA is
//! retired (`UPDATE nebula_ca SET retired_at = now() WHERE
//! retired_at IS NULL`), a fresh CA is minted at
//! `epoch = max_epoch + 1`, and every active peer cert is
//! re-signed under the new CA. A hash-chained Lifecycle event
//! records the rotation so [`crate::audit::verify`] picks it up.
//!
//! Idempotency: a rotation that runs while no active CA exists
//! is a no-op (returns [`BumpOutcome::NoActiveCa`]) — the
//! caller should mint the CA first via [`super::mint::mint_ca`].

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::{
    seal, CaError, NebulaCertBackend, DEFAULT_CA_CERT_PATH, DEFAULT_CA_KEY_PATH,
};

/// Default lifetime applied to a re-signed peer cert when the
/// prior cert had already expired at rotation time. Operators
/// who need a different value pass an explicit lifetime to
/// [`bump_epoch`].
pub const DEFAULT_FALLBACK_LIFETIME_DAYS: u32 = 365;

/// Outcome of one [`bump_epoch`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BumpOutcome {
    /// CA rotation completed. Prior CA was retired, new CA
    /// minted at `new_epoch`, and `resigned_count` peer certs
    /// were re-issued.
    Rotated {
        /// The epoch that just retired.
        prior_epoch: i64,
        /// The freshly minted epoch.
        new_epoch: i64,
        /// PEM body of the new CA cert.
        ca_cert_pem: String,
        /// Number of peer certs re-issued under the new CA.
        resigned_count: usize,
    },
    /// No active CA exists for this mesh — nothing to rotate.
    /// Caller should call [`super::mint::mint_ca`] first.
    NoActiveCa,
}

/// Roster row recovered from `nebula_peer_certs` for re-signing.
#[derive(Debug, Clone)]
struct PriorPeer {
    node_id: String,
    overlay_ip: String,
    /// Original `expires_at` so we can carry the same lifetime
    /// window onto the new cert (rotation preserves cert
    /// lifetime — it does not extend it).
    expires_at: i64,
}

/// Bump the CA epoch for `mesh_id`. Side effects:
///
///   1. `UPDATE nebula_ca SET retired_at = unixepoch()` on the
///      active row.
///   2. Insert a fresh row at `epoch = prior_epoch + 1` with a
///      newly minted CA cert + key.
///   3. Re-sign every active peer cert from the prior epoch
///      under the new CA, preserving each peer's overlay IP +
///      expiry. New rows land in `nebula_peer_certs` at the
///      new epoch; the prior rows remain (their epoch is now
///      historical — readers always join against the active
///      CA's epoch).
///   4. Emit one Lifecycle event recording the rotation so the
///      audit chain reflects the change.
///
/// The CA key + cert are written to `ca_crt_path` /
/// `ca_key_path` (defaulted to [`DEFAULT_CA_CERT_PATH`] +
/// [`DEFAULT_CA_KEY_PATH`] when `None`). Per-peer cert + key
/// files land at `<peer_cert_dir>/<node_id>.{crt,key}` —
/// callers usually pass `/etc/nebula/peers/` or, in tests, a
/// tempdir.
///
/// # Errors
///
/// - [`CaError::Sql`] on database errors.
/// - [`CaError::Subprocess`] / [`CaError::BinaryMissing`] when
///   `nebula-cert` shell-outs fail.
/// - [`CaError::Io`] on cert / key write failures.
#[allow(clippy::too_many_arguments)]
pub fn bump_epoch<B: NebulaCertBackend>(
    backend: &B,
    conn: &mut Connection,
    mesh_id: &str,
    ca_crt_path: Option<&Path>,
    ca_key_path: Option<&Path>,
    peer_cert_dir: &Path,
    cert_lifetime_days_fallback: u32,
) -> Result<BumpOutcome, CaError> {
    // 1. Identify the active CA's epoch (return NoActiveCa if
    //    there isn't one).
    let prior_epoch = match super::mint::current_ca(conn, mesh_id)? {
        Some((epoch, _)) => epoch,
        None => return Ok(BumpOutcome::NoActiveCa),
    };
    let new_epoch = prior_epoch + 1;

    // 2. Collect the peers we'll re-sign before mutating the
    //    nebula_ca table — once we mint the new CA, sign_peer
    //    queries the active epoch (which would then be the new
    //    one, hiding the prior peers).
    let prior_peers = load_active_peers(conn, prior_epoch)?;

    // 3. Atomic CA swap: retire the old, insert the new.
    let crt = ca_crt_path.unwrap_or_else(|| Path::new(DEFAULT_CA_CERT_PATH));
    let key = ca_key_path.unwrap_or_else(|| Path::new(DEFAULT_CA_KEY_PATH));
    backend.mint_ca(mesh_id, crt, key)?;
    let key_bytes = std::fs::read(key)
        .map_err(|e| CaError::Io(format!("read CA key {}: {e}", key.display())))?;
    seal::write_sealed(key, &key_bytes)?;
    let new_cert_pem = std::fs::read_to_string(crt)
        .map_err(|e| CaError::Io(format!("read CA cert {}: {e}", crt.display())))?;

    let tx = conn
        .transaction()
        .map_err(|e| CaError::Sql(e.to_string()))?;
    tx.execute(
        "UPDATE nebula_ca SET retired_at = unixepoch() \
         WHERE mesh_id = ?1 AND retired_at IS NULL",
        rusqlite::params![mesh_id],
    )
    .map_err(|e| CaError::Sql(e.to_string()))?;
    tx.execute(
        "INSERT INTO nebula_ca (mesh_id, epoch, ca_cert_pem, retired_at) \
         VALUES (?1, ?2, ?3, NULL)",
        rusqlite::params![mesh_id, new_epoch, new_cert_pem.as_str()],
    )
    .map_err(|e| CaError::Sql(e.to_string()))?;
    tx.commit().map_err(|e| CaError::Sql(e.to_string()))?;

    // 4. Re-sign each prior peer under the new epoch.
    std::fs::create_dir_all(peer_cert_dir).map_err(|e| {
        CaError::Io(format!("mkdir {}: {e}", peer_cert_dir.display()))
    })?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mut resigned_count = 0usize;
    for peer in &prior_peers {
        let crt_out = peer_cert_dir.join(peer_filename(&peer.node_id, "crt"));
        let key_out = peer_cert_dir.join(peer_filename(&peer.node_id, "key"));
        // The host vs peer role doesn't change on rotation — but
        // the only role-distinguishing data we have here is the
        // existing groups in cert_pem (not stored separately).
        // Default to Peer; the lighthouse re-signs itself through
        // its own promote() path with PeerRole::Host.
        backend.sign_peer(
            crt,
            key,
            &peer.node_id,
            &peer.overlay_ip,
            super::sign::DEFAULT_CIDR_PREFIX,
            &["role:peer"],
            &crt_out,
            &key_out,
        )?;
        let cert_pem = std::fs::read_to_string(&crt_out).map_err(|e| {
            CaError::Io(format!("read peer cert {}: {e}", crt_out.display()))
        })?;
        let key_bytes = std::fs::read(&key_out).map_err(|e| {
            CaError::Io(format!("read peer key {}: {e}", key_out.display()))
        })?;
        seal::write_sealed(&key_out, &key_bytes)?;

        // Preserve the original expiry when it's still in the
        // future; otherwise issue a fresh lifetime so the
        // rotation doesn't immediately produce expired certs.
        let expires_at = if peer.expires_at > now {
            peer.expires_at
        } else {
            now + (cert_lifetime_days_fallback as i64) * 86_400
        };

        conn.execute(
            "INSERT INTO nebula_peer_certs \
             (node_id, epoch, cert_pem, overlay_ip, expires_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                peer.node_id,
                new_epoch,
                cert_pem,
                peer.overlay_ip,
                expires_at
            ],
        )
        .map_err(|e| CaError::Sql(e.to_string()))?;
        resigned_count += 1;
    }

    // 5. Emit a Lifecycle audit event recording the rotation.
    let payload = serde_json::json!({
        "event":          "nebula_ca_rotation",
        "mesh_id":        mesh_id,
        "prior_epoch":    prior_epoch,
        "new_epoch":      new_epoch,
        "resigned_peers": resigned_count,
    })
    .to_string();
    if let Err(e) = crate::store::insert_event(conn, "lifecycle", "nebula-supervisor", &payload) {
        // Audit-log failure is loud but non-fatal — the
        // rotation itself succeeded. Surface via tracing so
        // operators see the gap.
        tracing::warn!(error = %e, "nebula_ca_rotation: audit event insert failed");
    }

    tracing::info!(
        mesh_id, prior_epoch, new_epoch, resigned_count,
        "nebula CA rotated"
    );

    Ok(BumpOutcome::Rotated {
        prior_epoch,
        new_epoch,
        ca_cert_pem: new_cert_pem,
        resigned_count,
    })
}

/// Read every active (non-revoked) peer cert at `epoch`.
fn load_active_peers(conn: &Connection, epoch: i64) -> Result<Vec<PriorPeer>, CaError> {
    let mut stmt = conn
        .prepare(
            "SELECT node_id, overlay_ip, expires_at \
             FROM nebula_peer_certs \
             WHERE epoch = ?1 AND revoked_at IS NULL \
             ORDER BY overlay_ip",
        )
        .map_err(|e| CaError::Sql(e.to_string()))?;
    let rows = stmt
        .query_map([epoch], |r| {
            Ok(PriorPeer {
                node_id: r.get::<_, String>(0)?,
                overlay_ip: r.get::<_, String>(1)?,
                expires_at: r.get::<_, i64>(2)?,
            })
        })
        .map_err(|e| CaError::Sql(e.to_string()))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| CaError::Sql(e.to_string()))?);
    }
    Ok(out)
}

/// Build the per-peer filename used under the peer-cert dir.
/// Sanitises the `:` in `peer:<name>` so the file is portable.
fn peer_filename(node_id: &str, ext: &str) -> PathBuf {
    PathBuf::from(format!("{}.{}", node_id.replace(':', "_"), ext))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ca::{mint, sign, MockBackend};

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("memory db");
        crate::store::migrate(&conn).expect("migrate");
        conn
    }

    fn mint_then_sign(conn: &mut Connection, dir: &Path, names: &[&str]) {
        let crt = dir.join("ca.crt");
        let key = dir.join("ca.key");
        mint::mint_ca(&MockBackend, conn, "m1", Some(&crt), Some(&key))
            .expect("mint");
        for name in names {
            let pcrt = dir.join(format!("{name}.crt"));
            let pkey = dir.join(format!("{name}.key"));
            sign::sign_peer_cert(
                &MockBackend,
                conn,
                "m1",
                name,
                sign::PeerRole::Peer,
                &crt,
                &key,
                &pcrt,
                &pkey,
                30,
            )
            .expect("sign");
        }
    }

    #[test]
    fn no_active_ca_returns_no_active_ca() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut conn = fresh_conn();
        let outcome = bump_epoch(
            &MockBackend,
            &mut conn,
            "never-minted",
            Some(&tmp.path().join("ca.crt")),
            Some(&tmp.path().join("ca.key")),
            tmp.path(),
            30,
        )
        .expect("bump");
        assert_eq!(outcome, BumpOutcome::NoActiveCa);
    }

    #[test]
    fn rotation_advances_epoch_and_retires_prior() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut conn = fresh_conn();
        mint_then_sign(&mut conn, tmp.path(), &["peer:alpha", "peer:beta"]);

        let outcome = bump_epoch(
            &MockBackend,
            &mut conn,
            "m1",
            Some(&tmp.path().join("ca.crt")),
            Some(&tmp.path().join("ca.key")),
            tmp.path(),
            30,
        )
        .expect("bump");
        match outcome {
            BumpOutcome::Rotated {
                prior_epoch,
                new_epoch,
                resigned_count,
                ..
            } => {
                assert_eq!(prior_epoch, 0);
                assert_eq!(new_epoch, 1);
                assert_eq!(resigned_count, 2);
            }
            other => panic!("expected Rotated, got {other:?}"),
        }

        // Prior CA row retired_at is non-null.
        let prior_retired: Option<i64> = conn
            .query_row(
                "SELECT retired_at FROM nebula_ca WHERE mesh_id='m1' AND epoch=0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(prior_retired.is_some());

        // The new CA is now the active one.
        let active = mint::current_ca(&conn, "m1").unwrap().expect("active");
        assert_eq!(active.0, 1);
    }

    #[test]
    fn rotation_resigns_each_active_peer() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut conn = fresh_conn();
        mint_then_sign(&mut conn, tmp.path(), &["peer:alpha", "peer:beta"]);

        bump_epoch(
            &MockBackend,
            &mut conn,
            "m1",
            Some(&tmp.path().join("ca.crt")),
            Some(&tmp.path().join("ca.key")),
            tmp.path(),
            30,
        )
        .expect("bump");

        // Each peer has rows at both epoch 0 (historical) +
        // epoch 1 (active).
        let row_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM nebula_peer_certs WHERE node_id='peer:alpha'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(row_count, 2);

        // The new epoch's row uses the same overlay_ip the
        // prior epoch allocated (rotation preserves IP).
        let prior_ip: String = conn
            .query_row(
                "SELECT overlay_ip FROM nebula_peer_certs \
                 WHERE node_id='peer:alpha' AND epoch=0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let new_ip: String = conn
            .query_row(
                "SELECT overlay_ip FROM nebula_peer_certs \
                 WHERE node_id='peer:alpha' AND epoch=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(prior_ip, new_ip);
    }

    #[test]
    fn rotation_emits_lifecycle_audit_event() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut conn = fresh_conn();
        mint_then_sign(&mut conn, tmp.path(), &["peer:alpha"]);
        let prior_event_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
            .unwrap();

        bump_epoch(
            &MockBackend,
            &mut conn,
            "m1",
            Some(&tmp.path().join("ca.crt")),
            Some(&tmp.path().join("ca.key")),
            tmp.path(),
            30,
        )
        .expect("bump");

        let new_event_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(new_event_count, prior_event_count + 1);

        let payload: String = conn
            .query_row(
                "SELECT payload_json FROM events ORDER BY seq DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(payload.contains("nebula_ca_rotation"));
        assert!(payload.contains("\"prior_epoch\":0"));
        assert!(payload.contains("\"new_epoch\":1"));
    }

    #[test]
    fn rotation_with_no_peers_resigns_zero() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut conn = fresh_conn();
        // Mint with no peers.
        mint::mint_ca(
            &MockBackend,
            &mut conn,
            "m1",
            Some(&tmp.path().join("ca.crt")),
            Some(&tmp.path().join("ca.key")),
        )
        .expect("mint");

        let outcome = bump_epoch(
            &MockBackend,
            &mut conn,
            "m1",
            Some(&tmp.path().join("ca.crt")),
            Some(&tmp.path().join("ca.key")),
            tmp.path(),
            30,
        )
        .expect("bump");
        match outcome {
            BumpOutcome::Rotated { resigned_count, .. } => {
                assert_eq!(resigned_count, 0);
            }
            other => panic!("expected Rotated, got {other:?}"),
        }
    }

    #[test]
    fn rotation_revoked_peer_not_re_signed() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut conn = fresh_conn();
        mint_then_sign(&mut conn, tmp.path(), &["peer:alpha", "peer:beta"]);
        // Revoke beta.
        conn.execute(
            "UPDATE nebula_peer_certs SET revoked_at = unixepoch() \
             WHERE node_id='peer:beta'",
            [],
        )
        .unwrap();

        let outcome = bump_epoch(
            &MockBackend,
            &mut conn,
            "m1",
            Some(&tmp.path().join("ca.crt")),
            Some(&tmp.path().join("ca.key")),
            tmp.path(),
            30,
        )
        .expect("bump");
        match outcome {
            BumpOutcome::Rotated { resigned_count, .. } => {
                assert_eq!(resigned_count, 1);
            }
            other => panic!("expected Rotated, got {other:?}"),
        }
    }

    #[test]
    fn rotation_extends_expired_cert_lifetime() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut conn = fresh_conn();
        mint_then_sign(&mut conn, tmp.path(), &["peer:alpha"]);
        // Backdate the cert so it's already expired.
        conn.execute(
            "UPDATE nebula_peer_certs SET expires_at = 1 WHERE node_id='peer:alpha'",
            [],
        )
        .unwrap();

        bump_epoch(
            &MockBackend,
            &mut conn,
            "m1",
            Some(&tmp.path().join("ca.crt")),
            Some(&tmp.path().join("ca.key")),
            tmp.path(),
            30,
        )
        .expect("bump");

        // New epoch's row has expires_at >> 1.
        let new_expires: i64 = conn
            .query_row(
                "SELECT expires_at FROM nebula_peer_certs \
                 WHERE node_id='peer:alpha' AND epoch=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(new_expires > 1);
    }

    #[test]
    fn peer_filename_sanitises_colon() {
        let p = peer_filename("peer:anvil", "crt");
        assert_eq!(p.to_str().unwrap(), "peer_anvil.crt");
    }

    #[test]
    fn rotation_resigned_cert_carries_peer_role_group() {
        // Documents the subtle behavior: the rotation path
        // re-signs every peer with `role:peer` regardless of
        // the prior role. The lighthouse's own re-sign path
        // (driven by the supervisor's promote()) re-issues
        // its own cert via sign_peer_cert(..., PeerRole::Host)
        // after bump_epoch.
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut conn = fresh_conn();
        mint_then_sign(&mut conn, tmp.path(), &["peer:alpha"]);
        bump_epoch(
            &MockBackend,
            &mut conn,
            "m1",
            Some(&tmp.path().join("ca.crt")),
            Some(&tmp.path().join("ca.key")),
            tmp.path(),
            30,
        )
        .expect("bump");
        let cert_pem: String = conn
            .query_row(
                "SELECT cert_pem FROM nebula_peer_certs \
                 WHERE node_id='peer:alpha' AND epoch=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(cert_pem.contains("groups=role:peer"));
    }
}
