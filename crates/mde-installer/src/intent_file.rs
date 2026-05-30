//! Mesh-storage upgrade-intent barrier files (INST-10).
//!
//! `mde-update --coordinate <version>` writes
//! `/mnt/mesh-storage/upgrade-intent/<version>.json`. Every peer's
//! mackesd polls the dir; on a new intent it upgrades on its own
//! schedule and writes its hostname into `ready`. Rollback = delete
//! the file.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// A fleet-wide upgrade barrier intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeIntent {
    /// Target `mde-core` version every peer should converge to.
    pub target_version: String,
    /// Hostname that started the barrier.
    pub initiated_by: String,
    /// Epoch milliseconds when the barrier was created.
    pub initiated_at_ms: u64,
    /// Hostnames that have completed the upgrade.
    pub ready: Vec<String>,
}

impl UpgradeIntent {
    /// Create a fresh intent stamped with the current time.
    #[must_use]
    pub fn new(target_version: impl Into<String>, initiated_by: impl Into<String>) -> Self {
        Self {
            target_version: target_version.into(),
            initiated_by: initiated_by.into(),
            initiated_at_ms: now_ms(),
            ready: Vec::new(),
        }
    }

    /// Mark `hostname` ready (idempotent).
    pub fn mark_ready(&mut self, hostname: &str) {
        if !self.ready.iter().any(|h| h == hostname) {
            self.ready.push(hostname.to_string());
        }
    }
}

/// The `upgrade-intent/` directory under a mesh-home mount.
#[must_use]
pub fn intent_dir(mesh_home: &Path) -> PathBuf {
    mesh_home.join("upgrade-intent")
}

/// Path of the intent file for `version` under `dir`.
#[must_use]
pub fn intent_path(dir: &Path, version: &str) -> PathBuf {
    dir.join(format!("{version}.json"))
}

/// Write `intent` as `<dir>/<version>.json`, creating `dir` if needed.
/// Returns the written path.
///
/// # Errors
/// IO or serialization failures.
pub fn write_intent(dir: &Path, intent: &UpgradeIntent) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let path = intent_path(dir, &intent.target_version);
    let json = serde_json::to_string_pretty(intent)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&path, json)?;
    Ok(path)
}

/// Read an intent file.
///
/// # Errors
/// IO or deserialization failures.
pub fn read_intent(path: &Path) -> io::Result<UpgradeIntent> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Record `hostname` as ready in an existing intent file (read-modify-write).
///
/// # Errors
/// IO or (de)serialization failures.
pub fn mark_ready_in_file(path: &Path, hostname: &str) -> io::Result<()> {
    let mut intent = read_intent(path)?;
    intent.mark_ready(hostname);
    let json = serde_json::to_string_pretty(&intent)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path, json)
}

/// Resolve the mesh-home mount: `$MDE_MESH_HOME` if set, else
/// `~/.mde-mesh` (the coordination mount per AI_GOVERNANCE §3.1).
#[must_use]
pub fn default_mesh_home() -> PathBuf {
    if let Ok(p) = std::env::var("MDE_MESH_HOME") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join(".mde-mesh")
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn write_then_read_roundtrips() {
        let dir = tempdir().unwrap();
        let intent = UpgradeIntent::new("2.7.1", "anvil");
        let path = write_intent(dir.path(), &intent).unwrap();
        assert!(path.ends_with("2.7.1.json"));
        let back = read_intent(&path).unwrap();
        assert_eq!(back, intent);
    }

    #[test]
    fn mark_ready_is_idempotent() {
        let mut intent = UpgradeIntent::new("2.7.1", "anvil");
        intent.mark_ready("forge");
        intent.mark_ready("forge");
        assert_eq!(intent.ready, vec!["forge".to_string()]);
    }

    #[test]
    fn mark_ready_in_file_persists() {
        let dir = tempdir().unwrap();
        let intent = UpgradeIntent::new("3.0.0", "anvil");
        let path = write_intent(dir.path(), &intent).unwrap();
        mark_ready_in_file(&path, "forge").unwrap();
        let back = read_intent(&path).unwrap();
        assert_eq!(back.ready, vec!["forge".to_string()]);
    }

    #[test]
    fn intent_dir_under_mesh_home() {
        let d = intent_dir(Path::new("/home/u/.mde-mesh"));
        assert_eq!(d, Path::new("/home/u/.mde-mesh/upgrade-intent"));
    }
}
