//! KDC2-4.1+4.2 — phones-reachability manifest in QNM-Shared.
//!
//! When `KdcHost` on peer A pairs a phone, it calls
//! [`write_manifest`] to publish the phone's identity at
//! `QNM-Shared/<peer-A>/connect/phones.json`. Other peers on
//! the same QNM-Shared mount read those manifests on their
//! reconcile tick ([`read_manifest`]); peer B then injects
//! synthetic-mDNS announces (KDC2-4.3) so phone X is
//! reachable from B without re-pairing.
//!
//! On-disk shape (one file per peer):
//!
//! ```json
//! {
//!   "schema": 1,
//!   "peer_id": "peer-A",
//!   "phones": [
//!     {
//!       "id": "abc-123-def",
//!       "name": "Pixel 8",
//!       "fingerprint": "AB:CD:EF:...",
//!       "capabilities": ["kdeconnect.clipboard", "kdeconnect.notification"],
//!       "last_seen": 1700000000
//!     }
//!   ]
//! }
//! ```
//!
//! Atomic writes via temp-file + rename so a crash mid-write
//! doesn't leave a half-written manifest neighbors might
//! parse + crash on.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Schema version of the on-disk manifest. Bump when a breaking
/// change to `PhoneRecord` lands; older readers refuse to parse
/// newer-schema files (forward-incompat by design — operators
/// see a clean "unsupported schema" error rather than a silent
/// truncation).
pub const SCHEMA_VERSION: u32 = 1;

/// One paired phone's identity as published to QNM-Shared.
/// Subset of `mde_kdc::pairing::PairedDevice` — only the
/// fields neighbors need to inject synthetic-mDNS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhoneRecord {
    /// Stable device id (KDC UUID).
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// SHA-256 fingerprint (KDC format, `AB:CD:...`).
    pub fingerprint: String,
    /// Plugin tokens the phone advertised under
    /// `incomingCapabilities`. Neighbors use this to gate
    /// what they're allowed to send.
    pub capabilities: Vec<String>,
    /// Unix epoch seconds of the most recent direct
    /// reachability observation from the publishing peer.
    pub last_seen: i64,
}

/// On-disk `phones.json` shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhonesManifest {
    /// Schema version. Bumped on breaking changes.
    pub schema: u32,
    /// Publishing peer's id. Matches the parent directory name
    /// under `QNM-Shared/<peer>/connect/`.
    pub peer_id: String,
    /// Currently-paired phones.
    pub phones: Vec<PhoneRecord>,
}

impl PhonesManifest {
    /// Build a manifest with the current schema version.
    #[must_use]
    pub fn new(peer_id: impl Into<String>, phones: Vec<PhoneRecord>) -> Self {
        Self {
            schema: SCHEMA_VERSION,
            peer_id: peer_id.into(),
            phones,
        }
    }
}

/// Errors the manifest API may surface.
#[derive(Debug)]
pub enum ManifestError {
    /// I/O failed reading or writing.
    Io(std::io::Error),
    /// JSON serialize/deserialize failed.
    Json(serde_json::Error),
    /// On-disk file has a `schema` field newer than
    /// [`SCHEMA_VERSION`] — refuse to parse to avoid silent
    /// truncation of fields this version doesn't know about.
    UnsupportedSchema(u32),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::Io(e) => write!(f, "io: {e}"),
            ManifestError::Json(e) => write!(f, "json: {e}"),
            ManifestError::UnsupportedSchema(v) => {
                write!(f, "unsupported_schema: {v} > {SCHEMA_VERSION}")
            }
        }
    }
}

impl std::error::Error for ManifestError {}

impl From<std::io::Error> for ManifestError {
    fn from(e: std::io::Error) -> Self {
        ManifestError::Io(e)
    }
}

impl From<serde_json::Error> for ManifestError {
    fn from(e: serde_json::Error) -> Self {
        ManifestError::Json(e)
    }
}

/// Path of the manifest file for a given peer under
/// `qnm_root`.
#[must_use]
pub fn manifest_path(qnm_root: &Path, peer_id: &str) -> PathBuf {
    qnm_root.join(peer_id).join("connect").join("phones.json")
}

/// Atomically write a `PhonesManifest` for `peer_id` under
/// `qnm_root`. Creates parent directories as needed.
pub fn write_manifest(qnm_root: &Path, manifest: &PhonesManifest) -> Result<(), ManifestError> {
    let path = manifest_path(qnm_root, &manifest.peer_id);
    let parent = path.parent().ok_or_else(|| {
        ManifestError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "manifest path has no parent dir",
        ))
    })?;
    std::fs::create_dir_all(parent)?;
    let raw = serde_json::to_vec_pretty(manifest)?;
    // Temp-file + rename for atomicity.
    let tmp = parent.join(format!(".phones.json.tmp.{}", std::process::id()));
    std::fs::write(&tmp, &raw)?;
    std::fs::rename(&tmp, &path).map_err(|e| {
        // Best-effort cleanup if the rename failed.
        let _ = std::fs::remove_file(&tmp);
        ManifestError::Io(e)
    })?;
    Ok(())
}

/// Read a `PhonesManifest` from `qnm_root/peer_id/connect/
/// phones.json`. `None` when the file is absent (peer hasn't
/// published yet). `Err` for I/O failure + JSON parse failure
/// + unsupported schema version.
pub fn read_manifest(
    qnm_root: &Path,
    peer_id: &str,
) -> Result<Option<PhonesManifest>, ManifestError> {
    let path = manifest_path(qnm_root, peer_id);
    let raw = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(ManifestError::Io(e)),
    };
    let manifest: PhonesManifest = serde_json::from_slice(&raw)?;
    if manifest.schema > SCHEMA_VERSION {
        return Err(ManifestError::UnsupportedSchema(manifest.schema));
    }
    Ok(Some(manifest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_phone(id: &str) -> PhoneRecord {
        PhoneRecord {
            id: id.into(),
            name: "Pixel".into(),
            fingerprint: "AB:CD:EF".into(),
            capabilities: vec!["kdeconnect.clipboard".into()],
            last_seen: 1_700_000_000,
        }
    }

    #[test]
    fn manifest_path_includes_peer_and_connect_dir() {
        let p = manifest_path(Path::new("/qnm"), "peer-A");
        assert_eq!(p, PathBuf::from("/qnm/peer-A/connect/phones.json"));
    }

    #[test]
    fn write_then_read_round_trips() {
        let tmp = tempdir().unwrap();
        let manifest = PhonesManifest::new("peer-A", vec![sample_phone("abc")]);
        write_manifest(tmp.path(), &manifest).unwrap();
        let back = read_manifest(tmp.path(), "peer-A").unwrap().unwrap();
        assert_eq!(back, manifest);
    }

    #[test]
    fn read_manifest_returns_none_when_absent() {
        let tmp = tempdir().unwrap();
        let r = read_manifest(tmp.path(), "never-paired").unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn write_creates_parent_directory_chain() {
        let tmp = tempdir().unwrap();
        let manifest = PhonesManifest::new("peer-X", vec![]);
        write_manifest(tmp.path(), &manifest).unwrap();
        assert!(tmp.path().join("peer-X").join("connect").exists());
    }

    #[test]
    fn write_overwrites_existing_atomically() {
        let tmp = tempdir().unwrap();
        let m1 = PhonesManifest::new("peer-A", vec![sample_phone("first")]);
        write_manifest(tmp.path(), &m1).unwrap();
        let m2 = PhonesManifest::new(
            "peer-A",
            vec![sample_phone("second"), sample_phone("third")],
        );
        write_manifest(tmp.path(), &m2).unwrap();
        let back = read_manifest(tmp.path(), "peer-A").unwrap().unwrap();
        assert_eq!(back.phones.len(), 2);
        assert_eq!(back.phones[0].id, "second");
    }

    #[test]
    fn read_rejects_unsupported_schema() {
        let tmp = tempdir().unwrap();
        let path = manifest_path(tmp.path(), "peer-X");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // Hand-craft a manifest with schema version 999.
        let raw = serde_json::json!({
            "schema": 999,
            "peer_id": "peer-X",
            "phones": []
        });
        std::fs::write(&path, serde_json::to_vec(&raw).unwrap()).unwrap();
        let err = read_manifest(tmp.path(), "peer-X").unwrap_err();
        match err {
            ManifestError::UnsupportedSchema(v) => assert_eq!(v, 999),
            other => panic!("expected UnsupportedSchema, got {other:?}"),
        }
    }

    #[test]
    fn manifest_serializes_with_stable_field_names() {
        // Wire-compat lock: neighbors on other peers parse
        // these files. Field names must stay stable.
        let m = PhonesManifest::new("p", vec![sample_phone("a")]);
        let raw = serde_json::to_string(&m).unwrap();
        assert!(raw.contains(r#""schema":1"#));
        assert!(raw.contains(r#""peer_id":"p""#));
        assert!(raw.contains(r#""fingerprint":"AB:CD:EF""#));
        assert!(raw.contains(r#""capabilities":["kdeconnect.clipboard"]"#));
        assert!(raw.contains(r#""last_seen":1700000000"#));
    }
}
