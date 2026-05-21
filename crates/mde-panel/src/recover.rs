//! Phase E.24 — `mde-panel --recover` CLI.
//!
//! Prints a plain-text preview of the most recent birthright
//! snapshot (per Phase 11.x). No Iced — just a sub-command of
//! `mde-panel` that reads the snapshot index + writes a summary
//! to stdout, exit 0.

use std::path::{Path, PathBuf};

/// Default snapshot root — matches the birthright cache lock.
#[must_use]
pub fn default_snapshot_root() -> PathBuf {
    dirs::config_dir()
        .map(|d| d.join("mde/snapshots"))
        .unwrap_or_else(|| PathBuf::from("/var/lib/mde/snapshots"))
}

/// Find the latest snapshot directory under `root`, sorted
/// reverse-alphanumerically by name (timestamp-prefixed names
/// guarantee chronological ordering).
#[must_use]
pub fn latest_snapshot(root: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(root).ok()?;
    let mut names: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    names.sort();
    names.pop()
}

/// Render the rollback preview. Empty string when no snapshot exists.
#[must_use]
pub fn render_preview(root: &Path) -> String {
    let Some(snap) = latest_snapshot(root) else {
        return "mde-panel --recover: no snapshots available.".into();
    };
    let name = snap
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "(unnamed)".into());
    let manifest = snap.join("manifest.json");
    let body = if manifest.exists() {
        format!(
            "rollback target: {name}\nmanifest:      {}\n(use `mde recover --apply` to roll back this snapshot)",
            manifest.display(),
        )
    } else {
        format!(
            "rollback target: {name}\nWARNING: manifest.json missing — snapshot may be incomplete."
        )
    };
    format!("mde-panel --recover\n\n{body}")
}

/// Entry point for `mde-panel --recover`. Prints to stdout, exit 0.
pub fn run() {
    let root = default_snapshot_root();
    println!("{}", render_preview(&root));
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn render_preview_empty_root_says_no_snapshots() {
        let tmp = tempdir().unwrap();
        let preview = render_preview(tmp.path());
        assert!(preview.contains("no snapshots"));
    }

    #[test]
    fn latest_snapshot_returns_lexicographically_last() {
        let tmp = tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("2026-05-19T120000")).unwrap();
        std::fs::create_dir(tmp.path().join("2026-05-20T080000")).unwrap();
        std::fs::create_dir(tmp.path().join("2026-05-18T230000")).unwrap();
        let latest = latest_snapshot(tmp.path()).unwrap();
        assert!(latest.ends_with("2026-05-20T080000"));
    }

    #[test]
    fn render_preview_calls_out_missing_manifest() {
        let tmp = tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("2026-05-20T120000")).unwrap();
        let preview = render_preview(tmp.path());
        assert!(preview.contains("manifest.json missing"));
    }

    #[test]
    fn render_preview_with_complete_snapshot_shows_manifest() {
        let tmp = tempdir().unwrap();
        let snap = tmp.path().join("2026-05-20T120000");
        std::fs::create_dir(&snap).unwrap();
        std::fs::write(snap.join("manifest.json"), "{}").unwrap();
        let preview = render_preview(tmp.path());
        assert!(preview.contains("rollback target"));
        assert!(preview.contains("manifest.json"));
        assert!(preview.contains("mde recover --apply"));
    }

    #[test]
    fn latest_snapshot_ignores_files() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("loose-file"), "x").unwrap();
        assert!(latest_snapshot(tmp.path()).is_none());
    }

    #[test]
    fn default_snapshot_root_ends_with_snapshots() {
        let root = default_snapshot_root();
        assert!(root.ends_with("snapshots"));
    }
}
