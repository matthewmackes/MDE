//! Bundle writer for the per-peer Nebula enrollment payload (NF-2.7).
//!
//! Extends the enrollment surface with a `NebulaBundle` shape that
//! the joining peer's watcher reads to materialize its
//! `/etc/nebula/{config.yaml, ca.crt, host.crt, host.key}` tree.
//! The bundle file lives at
//! `~/QNM-Shared/<peer>/mackesd/nebula-bundle.json` next to the
//! existing `heartbeat.json` (atomic temp + rename write).
//!
//! The supervisor that actually starts `nebula.service` from the
//! bundle ships in NF-3 — this module just produces the JSON.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::error::{CaError, CaResult};

/// Wire-shape of the per-peer Nebula enrollment bundle.
///
/// Written atomically (tempfile + fsync + rename) so a reader that
/// sees the file always sees the complete JSON payload. Holds
/// every field the joining peer needs to spin up
/// `nebula.service` without contacting the leader again.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NebulaBundle {
    /// PEM-encoded CA cert (public — drives Nebula's trust roots).
    pub ca_cert_pem: String,
    /// PEM-encoded peer host cert signed by the CA.
    pub peer_cert_pem: String,
    /// PEM-encoded peer host key — kept private; the bundle file's
    /// 0600 permission protects it on the way to the peer.
    pub peer_key_pem: String,
    /// Overlay IP allocated for the peer (host-only, dotted-quad).
    pub overlay_ip: String,
    /// Mesh CIDR string the IP was allocated from
    /// (e.g. `10.42.0.0/16`).
    pub mesh_cidr: String,
    /// Lighthouse roster — list of static-IP:port endpoints every
    /// peer pins into `nebula.yaml::lighthouse.hosts`.
    pub lighthouse_roster: Vec<String>,
}

impl NebulaBundle {
    /// Build a bundle from the constituent pieces. Pure constructor;
    /// no I/O.
    #[must_use]
    pub fn new(
        ca_cert_pem: String,
        peer_cert_pem: String,
        peer_key_pem: String,
        overlay_ip: String,
        mesh_cidr: String,
        lighthouse_roster: Vec<String>,
    ) -> Self {
        Self {
            ca_cert_pem,
            peer_cert_pem,
            peer_key_pem,
            overlay_ip,
            mesh_cidr,
            lighthouse_roster,
        }
    }
}

/// Write the bundle to `path` atomically. The destination directory
/// is created if absent. Caller decides the path — typically
/// `~/QNM-Shared/<peer>/mackesd/nebula-bundle.json`.
///
/// The file lands at mode `0600` so an unsealed bundle never
/// becomes world-readable on the QNM-Shared root.
///
/// # Errors
///
/// - [`CaError::Io`] on filesystem failure (create_dir_all, write,
///   rename).
/// - [`CaError::Sql`] is reused with a serde context message when
///   serialization fails (no separate JSON variant — the existing
///   surface is dense enough).
pub fn write_bundle(path: &Path, bundle: &NebulaBundle) -> CaResult<()> {
    let json =
        serde_json::to_vec_pretty(bundle).map_err(|e| CaError::Sql(format!("bundle json: {e}")))?;
    super::seal::seal(path, &json, 0o600)
}

/// Read a bundle from disk. Used by the joining peer's watcher in
/// NF-3 — included here so the round-trip is exercised in unit
/// tests.
///
/// # Errors
///
/// - [`CaError::Io`] on filesystem failure (missing path, EACCES, …).
/// - [`CaError::Sql`] with a serde context message on JSON parse
///   failure.
pub fn read_bundle(path: &Path) -> CaResult<NebulaBundle> {
    let bytes = std::fs::read(path).map_err(CaError::Io)?;
    serde_json::from_slice(&bytes).map_err(|e| CaError::Sql(format!("bundle json: {e}")))
}

/// Resolve the canonical bundle path for `peer_name` under
/// `qnm_shared_root`. Mirrors the `~/QNM-Shared/<peer>/mackesd/`
/// layout the existing heartbeat writer uses.
#[must_use]
pub fn bundle_path(qnm_shared_root: &Path, peer_name: &str) -> std::path::PathBuf {
    qnm_shared_root
        .join(peer_name)
        .join("mackesd")
        .join("nebula-bundle.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fixture_bundle() -> NebulaBundle {
        NebulaBundle::new(
            "-----BEGIN NEBULA CERT-----\nCA-PEM\n-----END NEBULA CERT-----".into(),
            "-----BEGIN NEBULA CERT-----\nPEER-PEM\n-----END NEBULA CERT-----".into(),
            "-----BEGIN NEBULA KEY-----\nPEER-KEY\n-----END NEBULA KEY-----".into(),
            "10.42.0.7".into(),
            "10.42.0.0/16".into(),
            vec!["198.51.100.1:4242".into(), "198.51.100.2:4242".into()],
        )
    }

    #[test]
    fn write_then_read_round_trips_json() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("peer:anvil/mackesd/nebula-bundle.json");
        let b = fixture_bundle();
        write_bundle(&path, &b).expect("write");
        let back = read_bundle(&path).expect("read");
        assert_eq!(b, back);
    }

    #[test]
    fn write_seals_at_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nebula-bundle.json");
        write_bundle(&path, &fixture_bundle()).expect("write");
        let mode = std::fs::metadata(&path)
            .unwrap()
            .permissions()
            .mode()
            & 0o7777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn bundle_path_lays_out_under_qnm_shared() {
        let root = std::path::Path::new("/tmp/QNM-Shared");
        let path = bundle_path(root, "peer:anvil");
        assert_eq!(
            path,
            std::path::PathBuf::from("/tmp/QNM-Shared/peer:anvil/mackesd/nebula-bundle.json")
        );
    }

    #[test]
    fn read_errors_on_malformed_json() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.json");
        super::super::seal::seal(&path, b"not json", 0o600).unwrap();
        let err = read_bundle(&path).expect_err("must fail");
        match err {
            CaError::Sql(msg) => assert!(msg.contains("bundle json")),
            other => panic!("expected Sql parse err, got {other:?}"),
        }
    }
}
