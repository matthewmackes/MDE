//! Phase E.14 — wallpaper-area right-click menu (Iced port).
//!
//! The GTK version (mackes-panel/src/root_menu.rs) shipped 4 locked
//! actions (Phase 8.4 / v3.0.0 Q40): Change wallpaper / Open mesh
//! share / Send file to peer (per-peer submenu) / Display settings.
//!
//! The Iced port preserves the same 4 actions + the per-peer
//! submenu, ported away from `zenity` (X11-only) to `kdialog`
//! (the Qt6-based picker that ships with cosmic-files Recommends:)
//! falling through to `mde-files` itself when available.

use std::path::{Path, PathBuf};

/// Four locked actions for the root menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootMenuAction {
    /// Open the Look & Feel panel.
    ChangeWallpaper,
    /// `xdg-open ~/QNM-Shared/`.
    OpenMeshShare,
    /// Per-peer submenu — picks a file via portal, copies into the
    /// peer's QNM-Shared dir.
    SendFileToPeer(String),
    /// Open the Devices panel.
    DisplaySettings,
}

impl RootMenuAction {
    /// Display label for the action.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            RootMenuAction::ChangeWallpaper => "Change wallpaper…".into(),
            RootMenuAction::OpenMeshShare => "Open mesh share".into(),
            RootMenuAction::SendFileToPeer(peer) => format!("Send file → {peer}"),
            RootMenuAction::DisplaySettings => "Display settings".into(),
        }
    }

    /// Argv that performs the action. Pure — no subprocess spawn,
    /// returning a plain `Vec<String>` lets tests pin behavior.
    #[must_use]
    pub fn argv(&self, qnm_root: &Path) -> Vec<String> {
        match self {
            RootMenuAction::ChangeWallpaper => vec!["mde".into(), "--focus".into(), "look_and_feel.wallpaper".into()],
            RootMenuAction::OpenMeshShare => vec!["xdg-open".into(), qnm_root.display().to_string()],
            RootMenuAction::SendFileToPeer(peer) => {
                let dest = qnm_root.join(peer);
                vec![
                    "mde-files".into(),
                    "--send-to".into(),
                    dest.display().to_string(),
                ]
            }
            RootMenuAction::DisplaySettings => vec!["mde".into(), "--focus".into(), "devices.displays".into()],
        }
    }
}

/// Discover peers under `~/QNM-Shared/<peer>/`. Each immediate
/// sub-directory whose name doesn't start with `.` counts.
#[must_use]
pub fn discover_peers(qnm_root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(qnm_root) else {
        return Vec::new();
    };
    let mut peers: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .filter(|name| !name.starts_with('.'))
        .collect();
    peers.sort();
    peers
}

/// Build the complete action set (4 fixed + per-peer Send-To
/// entries). Caller renders these as the right-click menu rows.
#[must_use]
pub fn build_menu(qnm_root: &Path) -> Vec<RootMenuAction> {
    let mut menu = vec![
        RootMenuAction::ChangeWallpaper,
        RootMenuAction::OpenMeshShare,
    ];
    for peer in discover_peers(qnm_root) {
        menu.push(RootMenuAction::SendFileToPeer(peer));
    }
    menu.push(RootMenuAction::DisplaySettings);
    menu
}

/// Default QNM-Shared location resolver — `$HOME/QNM-Shared/`.
#[must_use]
pub fn default_qnm_root() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join("QNM-Shared"))
        .unwrap_or_else(|| PathBuf::from("/tmp/QNM-Shared"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn labels_match_lock() {
        assert_eq!(RootMenuAction::ChangeWallpaper.label(), "Change wallpaper…");
        assert_eq!(RootMenuAction::OpenMeshShare.label(), "Open mesh share");
        assert_eq!(RootMenuAction::DisplaySettings.label(), "Display settings");
        assert_eq!(
            RootMenuAction::SendFileToPeer("lab-01".into()).label(),
            "Send file → lab-01"
        );
    }

    #[test]
    fn argv_change_wallpaper_uses_mde_focus() {
        let root = PathBuf::from("/home/u/QNM-Shared");
        let argv = RootMenuAction::ChangeWallpaper.argv(&root);
        assert_eq!(argv, vec!["mde", "--focus", "look_and_feel.wallpaper"]);
    }

    #[test]
    fn argv_open_mesh_share_uses_xdg_open() {
        let root = PathBuf::from("/home/u/QNM-Shared");
        let argv = RootMenuAction::OpenMeshShare.argv(&root);
        assert_eq!(argv[0], "xdg-open");
        assert!(argv[1].ends_with("QNM-Shared"));
    }

    #[test]
    fn argv_send_file_to_peer_targets_peer_dir() {
        let root = PathBuf::from("/home/u/QNM-Shared");
        let argv = RootMenuAction::SendFileToPeer("lab-01".into()).argv(&root);
        assert_eq!(argv[0], "mde-files");
        assert_eq!(argv[1], "--send-to");
        assert!(argv[2].ends_with("QNM-Shared/lab-01"));
    }

    #[test]
    fn discover_peers_returns_sorted_subdirs() {
        let tmp = tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("zeta")).unwrap();
        std::fs::create_dir(tmp.path().join("alpha")).unwrap();
        std::fs::create_dir(tmp.path().join("mike")).unwrap();
        // Hidden + file should be ignored.
        std::fs::create_dir(tmp.path().join(".hidden")).unwrap();
        std::fs::write(tmp.path().join("readme.txt"), "skip me").unwrap();

        let peers = discover_peers(tmp.path());
        assert_eq!(peers, vec!["alpha", "mike", "zeta"]);
    }

    #[test]
    fn discover_peers_handles_missing_dir() {
        let peers = discover_peers(Path::new("/nonexistent/path/QNM-Shared"));
        assert!(peers.is_empty());
    }

    #[test]
    fn build_menu_has_fixed_items_plus_peers() {
        let tmp = tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("peer-1")).unwrap();
        std::fs::create_dir(tmp.path().join("peer-2")).unwrap();

        let menu = build_menu(tmp.path());
        assert_eq!(menu.len(), 5); // wallpaper + open + 2 peers + display
        assert!(matches!(menu[0], RootMenuAction::ChangeWallpaper));
        assert!(matches!(menu[1], RootMenuAction::OpenMeshShare));
        assert!(matches!(menu[2], RootMenuAction::SendFileToPeer(ref n) if n == "peer-1"));
        assert!(matches!(menu[3], RootMenuAction::SendFileToPeer(ref n) if n == "peer-2"));
        assert!(matches!(menu[4], RootMenuAction::DisplaySettings));
    }

    #[test]
    fn build_menu_with_no_peers_still_has_4_fixed_items() {
        let tmp = tempdir().unwrap();
        let menu = build_menu(tmp.path());
        assert_eq!(menu.len(), 3); // wallpaper + open + display (no peers)
    }

    #[test]
    fn default_qnm_root_is_in_home() {
        let root = default_qnm_root();
        assert!(root.ends_with("QNM-Shared"));
    }
}
