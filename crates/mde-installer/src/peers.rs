//! Peer registry + version-skew classification (PEERVER-3 / closes
//! INST-3b).
//!
//! Reads the converged peer-data directly from the GFS-replicated
//! `<mesh-home>/peers/` dir (no D-Bus / Bus / mackesd dependency, per
//! `docs/design/v2.7-peer-data-convergence.md`). Used by `mde-update`
//! (skew table, INST-9) and `mde-install`'s peer-impact preflight
//! (INST-5).

use std::io;
use std::path::PathBuf;
use std::process::Command;

pub use mackes_mesh_types::peers::{default_mesh_home, peers_dir, read_peers, PeerRecord};

/// Stale threshold: a peer whose file is older than this renders as
/// stale/offline (PEERVER-5). 10 minutes covers several heartbeat
/// intervals without flagging a peer that merely missed one tick.
pub const STALE_THRESHOLD_MS: u64 = 10 * 60 * 1000;

/// Version-skew of a peer relative to this node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skew {
    /// Same version as local.
    Match,
    /// Same MAJOR, different version.
    Minor,
    /// Different MAJOR version.
    Major,
    /// Either side's version is unknown/unparseable.
    Unknown,
}

impl Skew {
    /// Single-char marker for the table (`` / `(!)` / `(!!)` / `(?)`).
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Match => "",
            Self::Minor => "(!)",
            Self::Major => "(!!)",
            Self::Unknown => "(?)",
        }
    }
}

/// The converged peer list. Reads the GFS `peers/` dir and, if this
/// node's own row isn't present yet (e.g. before the mackesd heartbeat
/// writer has run — PEERVER-2), synthesizes a local row from `rpm -q`
/// so `mde-update` always shows at least this peer.
#[must_use]
pub fn list_peers() -> Vec<PeerRecord> {
    let dir = peers_dir(&default_mesh_home());
    let mut peers = read_peers(&dir);
    let local = local_hostname();
    if !peers.iter().any(|p| p.hostname == local) {
        peers.push(PeerRecord::now(local, local_mde_version(), "healthy"));
        peers.sort_by(|a, b| a.hostname.cmp(&b.hostname));
    }
    peers
}

/// Classify a remote version against the local version.
#[must_use]
pub fn classify_skew(local: Option<&str>, remote: Option<&str>) -> Skew {
    let (Some(l), Some(r)) = (local, remote) else {
        return Skew::Unknown;
    };
    if l == r {
        return Skew::Match;
    }
    match (major(l), major(r)) {
        (Some(lm), Some(rm)) if lm == rm => Skew::Minor,
        (Some(_), Some(_)) => Skew::Major,
        _ => Skew::Unknown,
    }
}

/// First dotted component of a version string as a number.
#[must_use]
pub fn major(version: &str) -> Option<u64> {
    version.split('.').next()?.trim().parse::<u64>().ok()
}

/// Remove this node's own peer-file (PEERVER-5). Own-row authority:
/// deleting `<mesh-home>/peers/<hostname>.json` is how this node leaves
/// the mesh registry (called from `mde-install`'s wipe sequence).
/// Returns the removed path, or `None` when there was nothing to remove.
///
/// # Errors
/// IO failures other than a missing file.
pub fn remove_local_peer_file() -> io::Result<Option<PathBuf>> {
    let path = peers_dir(&default_mesh_home()).join(format!("{}.json", local_hostname()));
    if path.exists() {
        std::fs::remove_file(&path)?;
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

/// This node's hostname (falls back to `localhost`).
#[must_use]
pub fn local_hostname() -> String {
    Command::new("hostname")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "localhost".to_string())
}

/// This node's installed `mde-core` RPM version, if queryable.
#[must_use]
pub fn local_mde_version() -> Option<String> {
    let out = Command::new("rpm")
        .args(["-q", "--qf", "%{VERSION}", "mde-core"])
        .output()
        .ok()?;
    if out.status.success() {
        let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (!v.is_empty()).then_some(v)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skew_match() {
        assert_eq!(classify_skew(Some("5.0.0"), Some("5.0.0")), Skew::Match);
    }

    #[test]
    fn skew_minor_same_major() {
        assert_eq!(classify_skew(Some("5.0.0"), Some("5.0.1")), Skew::Minor);
    }

    #[test]
    fn skew_major_differs() {
        assert_eq!(classify_skew(Some("5.0.0"), Some("6.0.0")), Skew::Major);
    }

    #[test]
    fn skew_unknown_on_missing() {
        assert_eq!(classify_skew(None, Some("5.0.0")), Skew::Unknown);
        assert_eq!(classify_skew(Some("5.0.0"), None), Skew::Unknown);
    }

    #[test]
    fn major_parses_first_component() {
        assert_eq!(major("5.0.1"), Some(5));
        assert_eq!(major("nope"), None);
    }

    #[test]
    fn markers() {
        assert_eq!(Skew::Match.marker(), "");
        assert_eq!(Skew::Minor.marker(), "(!)");
        assert_eq!(Skew::Major.marker(), "(!!)");
    }
}
