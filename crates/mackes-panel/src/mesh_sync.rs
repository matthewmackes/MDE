// Mesh-sync API consumed by Phase 2.6's GTK drift-status row.
#![allow(dead_code)]

//! QNM-Shared mirror + drift detection for `panel.toml`.
//!
//! Per Q19 / Q20 the panel config replicates whole-file via QNM-Shared:
//!
//! * `mirror_dir()` resolves `~/.qnm-sync/mackes-panel/`.
//! * `mirror(path)` copies `panel.toml` into the mirror dir on every
//!   write so the QNM-Shared replication picks it up.
//! * `compute_drift()` walks `mirror_dir()/peers/` and SHA-256-compares
//!   each peer's mirrored `panel.toml` to the local one. Returns a
//!   `DriftSummary` Phase 2.6's UI consumes.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Relative path under `$HOME` where QNM-Shared keeps the mackes-panel
/// mirror tree. Local copy at the top level, per-peer copies under
/// `peers/<peer-name>/panel.toml`.
const REL_MIRROR_DIR: &str = ".qnm-sync/mackes-panel";

/// One peer's drift state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerDrift {
    pub peer: String,
    pub status: DriftStatus,
}

/// Per-peer state surfaced in the Look & Feel → Panel → Sync row (Q22).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftStatus {
    /// Hash matches the local file byte-for-byte.
    InSync,
    /// Peer mirror exists but its hash differs.
    Drifted,
    /// Peer mirror file is missing (peer offline or hasn't joined yet).
    Missing,
    /// I/O error reading the peer mirror.
    Unreadable,
}

/// Aggregate drift across every peer mirror.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DriftSummary {
    pub peers: Vec<PeerDrift>,
}

impl DriftSummary {
    /// Number of peers whose mirror differs from the local file.
    #[must_use]
    pub fn drifted_count(&self) -> usize {
        self.peers
            .iter()
            .filter(|p| p.status == DriftStatus::Drifted)
            .count()
    }

    /// True if every peer is in sync (or there are no peers at all —
    /// the standalone-mesh case is implicitly "no drift").
    #[must_use]
    pub fn fully_in_sync(&self) -> bool {
        self.peers.iter().all(|p| p.status == DriftStatus::InSync)
    }
}

/// Resolve `~/.qnm-sync/mackes-panel/`. Returns `None` if `$HOME` isn't
/// set (the same edge case `config_store::path()` handles).
#[must_use]
pub fn mirror_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(REL_MIRROR_DIR))
}

/// Copy `src` (typically `~/.config/mackes-panel/panel.toml`) into the
/// QNM-Shared mirror at `~/.qnm-sync/mackes-panel/panel.toml`. The
/// mesh-side replication takes it from there.
///
/// Idempotent and content-aware: re-writes only when the bytes
/// actually differ so we don't churn QNM-Shared inotify events.
///
/// # Errors
/// Returns any underlying `std::io::Error`.
pub fn mirror(src: &Path) -> std::io::Result<()> {
    let Some(dir) = mirror_dir() else {
        return Err(std::io::Error::other(
            "HOME unset; cannot resolve mirror dir",
        ));
    };
    fs::create_dir_all(&dir)?;
    let dest = dir.join("panel.toml");
    if let Ok(existing) = fs::read(&dest) {
        let new = fs::read(src)?;
        if existing == new {
            return Ok(());
        }
    }
    fs::copy(src, &dest)?;
    Ok(())
}

/// Walk `mirror_dir()/peers/<peer>/panel.toml` and SHA-256-compare each
/// to the local mirror. Empty `peers/` (or missing entirely) → empty
/// `DriftSummary` — the "I'm the only one here" case.
#[must_use]
pub fn compute_drift() -> DriftSummary {
    let Some(dir) = mirror_dir() else {
        return DriftSummary::default();
    };
    let Some(local_hash) = hash_file(&dir.join("panel.toml")) else {
        return DriftSummary::default();
    };

    let peers_root = dir.join("peers");
    let Ok(entries) = fs::read_dir(&peers_root) else {
        return DriftSummary::default();
    };

    let mut peers: Vec<PeerDrift> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| {
            let peer = e.file_name().to_string_lossy().to_string();
            let path = e.path().join("panel.toml");
            let status = peer_status(&path, &local_hash);
            PeerDrift { peer, status }
        })
        .collect();

    peers.sort_by(|a, b| a.peer.cmp(&b.peer));
    DriftSummary { peers }
}

fn peer_status(path: &Path, local_hash: &[u8]) -> DriftStatus {
    if !path.exists() {
        return DriftStatus::Missing;
    }
    match hash_file(path) {
        Some(h) if h == local_hash => DriftStatus::InSync,
        Some(_) => DriftStatus::Drifted,
        None => DriftStatus::Unreadable,
    }
}

fn hash_file(path: &Path) -> Option<Vec<u8>> {
    let mut f = fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(hasher.finalize().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drift_summary_drifted_count() {
        let s = DriftSummary {
            peers: vec![
                PeerDrift {
                    peer: "a".into(),
                    status: DriftStatus::InSync,
                },
                PeerDrift {
                    peer: "b".into(),
                    status: DriftStatus::Drifted,
                },
                PeerDrift {
                    peer: "c".into(),
                    status: DriftStatus::Drifted,
                },
                PeerDrift {
                    peer: "d".into(),
                    status: DriftStatus::Missing,
                },
            ],
        };
        assert_eq!(s.drifted_count(), 2);
        assert!(!s.fully_in_sync());
    }

    #[test]
    fn empty_summary_is_fully_in_sync() {
        let s = DriftSummary::default();
        assert_eq!(s.drifted_count(), 0);
        assert!(s.fully_in_sync());
    }

    #[test]
    fn drift_status_classifies_paths() {
        // Use a non-existent path to exercise the Missing branch.
        let missing = peer_status(Path::new("/definitely/not/here.toml"), &[]);
        assert_eq!(missing, DriftStatus::Missing);
    }
}
