//! BUS-5.7 — clipboard pin store.
//!
//! `PinStore` maintains a persisted set of pinned entry ULIDs. Pinned
//! entries appear above rolling history in Super+V and their blob files
//! are exempted from the BUS-5.3 GC pass even when the bus message has
//! been retention-evicted (BUS-1.9).
//!
//! File: `<data_home>/mde/clipboard/pins.json`
//! Format: `{"pinned":["01H...","01J..."]}`
//! Write pattern: temp file + `fs::rename()` (atomic, same as blobstore).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Persisted set of pinned clipboard-entry ULIDs.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PinStore {
    pub pinned: HashSet<String>,
}

impl PinStore {
    /// Load from `path`; returns an empty store if the file is missing or
    /// malformed.
    pub fn load_from(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Atomically persist to `path`. Creates parent directories as needed.
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json =
            serde_json::to_string(self).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, json.as_bytes())?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Mark `ulid` as pinned. Duplicate inserts are idempotent.
    pub fn pin(&mut self, ulid: &str) {
        self.pinned.insert(ulid.to_owned());
    }

    /// Remove `ulid` from the pinned set. No-op if not present.
    pub fn unpin(&mut self, ulid: &str) {
        self.pinned.remove(ulid);
    }

    /// Returns `true` if `ulid` is pinned.
    pub fn is_pinned(&self, ulid: &str) -> bool {
        self.pinned.contains(ulid)
    }
}

/// `<data_home>/mde/clipboard/pins.json`.
pub fn pin_store_path(data_home: &Path) -> PathBuf {
    data_home.join("mde/clipboard/pins.json")
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_from_missing_returns_empty() {
        let path = PathBuf::from("/tmp/mde-pin-test-nonexistent-xyz.json");
        let store = PinStore::load_from(&path);
        assert!(store.pinned.is_empty());
    }

    #[test]
    fn load_from_malformed_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pins.json");
        std::fs::write(&path, b"not json").unwrap();
        let store = PinStore::load_from(&path);
        assert!(store.pinned.is_empty());
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pins.json");
        let mut store = PinStore::default();
        store.pin("01HAAAAAAAAAAAAAAAAAAAAAAAA");
        store.pin("01HBBBBBBBBBBBBBBBBBBBBBBBB");
        store.save_to(&path).unwrap();

        let loaded = PinStore::load_from(&path);
        assert!(loaded.is_pinned("01HAAAAAAAAAAAAAAAAAAAAAAAA"));
        assert!(loaded.is_pinned("01HBBBBBBBBBBBBBBBBBBBBBBBB"));
        assert!(!loaded.is_pinned("01HCCCCCCCCCCCCCCCCCCCCCCCC"));
    }

    #[test]
    fn save_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b/c/pins.json");
        let store = PinStore::default();
        store.save_to(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn save_is_atomic_temp_not_leftover() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pins.json");
        let store = PinStore::default();
        store.save_to(&path).unwrap();
        let tmp = path.with_extension("tmp");
        assert!(!tmp.exists(), "temp file must not linger");
    }

    #[test]
    fn pin_is_idempotent() {
        let mut store = PinStore::default();
        store.pin("ULID-A");
        store.pin("ULID-A");
        assert_eq!(store.pinned.len(), 1);
    }

    #[test]
    fn unpin_removes_entry() {
        let mut store = PinStore::default();
        store.pin("ULID-A");
        store.unpin("ULID-A");
        assert!(!store.is_pinned("ULID-A"));
    }

    #[test]
    fn unpin_nonexistent_is_noop() {
        let mut store = PinStore::default();
        store.unpin("ULID-NEVER-SEEN");
        assert!(store.pinned.is_empty());
    }

    #[test]
    fn is_pinned_false_by_default() {
        let store = PinStore::default();
        assert!(!store.is_pinned("ULID-X"));
    }

    #[test]
    fn pin_store_path_is_under_data_home() {
        let base = PathBuf::from("/home/user/.local/share");
        let p = pin_store_path(&base);
        assert!(p.starts_with(&base));
        assert!(p.ends_with("pins.json"));
    }
}
