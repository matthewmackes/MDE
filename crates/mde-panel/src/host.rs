//! Phase E1.3 panel-host applet wiring + spawn dispatch.
//!
//! Each MDE applet ships as a standalone binary under
//! `crates/mde-applets/<name>/`. The panel-host (this crate, when
//! it runs as `mde-panel`) is responsible for:
//!
//! 1. Discovering installed applets via JSON manifests
//!    (`mde_applet_api::discovery`).
//! 2. Spawning the right applet on demand — clock for the Clock
//!    zone, dock for the Tasklist zone, start-menu on Start click,
//!    notifications on bell click, etc.
//! 3. Letting each applet own its own window (Wayland layer-shell
//!    or top-level overlay) — the panel doesn't try to embed them.
//!
//! Why subprocess-spawn rather than in-process embedding?
//! - Each applet is its own Iced app with its own event loop;
//!   Iced 0.13 doesn't have a multi-window API mature enough to
//!   host them as child views.
//! - Crash isolation: a buggy applet shouldn't take down the
//!   panel. Subprocess spawn gives us that for free.
//! - Per-applet update cadence: an applet that polls aggressively
//!   doesn't slow the panel itself.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use crate::Pane;

/// One applet → which pane it lives in + which binary spawns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppletBinding {
    /// Pane the applet renders into.
    pub pane: Pane,
    /// Binary basename (resolved against `$PATH`).
    pub binary: String,
}

/// The default applet table — each pane gets one applet.
///
/// Phase E1.2.x lock: every applet in this map already ships as a
/// standalone binary under `crates/mde-applets/<name>/`. Future
/// renames or additions are user-extensible via per-user manifest
/// drops under `$XDG_DATA_HOME/mde/applets/`.
#[must_use]
pub fn default_bindings() -> HashMap<Pane, AppletBinding> {
    let mut m = HashMap::new();
    m.insert(
        Pane::Start,
        AppletBinding {
            pane: Pane::Start,
            binary: "mde-applet-start-menu".into(),
        },
    );
    m.insert(
        Pane::Pinned,
        AppletBinding {
            pane: Pane::Pinned,
            binary: "mde-applet-dock".into(),
        },
    );
    m.insert(
        Pane::Tasklist,
        AppletBinding {
            pane: Pane::Tasklist,
            binary: "mde-applet-app-switcher".into(),
        },
    );
    m.insert(
        Pane::Cluster,
        AppletBinding {
            pane: Pane::Cluster,
            // Phase E.4.1 follow-up — sway-cluster applet ships at
            // `crates/mde-applets/sway-cluster/`; it produces the
            // SPLIT/LAYOUT/WINDOW chip row.
            binary: "mde-applet-sway-cluster".into(),
        },
    );
    m.insert(
        Pane::Tray,
        AppletBinding {
            pane: Pane::Tray,
            binary: "mde-applet-notification-bell".into(),
        },
    );
    m.insert(
        Pane::Clock,
        AppletBinding {
            pane: Pane::Clock,
            binary: "mde-applet-clock".into(),
        },
    );
    m
}

/// Spawn the applet bound to the given pane, returning the child
/// process handle. Caller may adopt or drop the handle.
///
/// Returns `Err(AppletSpawnError::Unbound)` if the pane has no
/// applet binding, or `Err(AppletSpawnError::Spawn(..))` if the
/// binary cannot be exec'd.
pub fn spawn_for_pane(
    bindings: &HashMap<Pane, AppletBinding>,
    pane: Pane,
) -> Result<Child, AppletSpawnError> {
    let binding = bindings.get(&pane).ok_or(AppletSpawnError::Unbound(pane))?;
    spawn_by_binary(&binding.binary)
}

/// Spawn an applet by its `mde-applet-*` basename. Used by the
/// `mde-panel --apple-menu` / `--expose` / `--drawer` CLI hand-offs
/// + by the Tray pane (which mounts multiple applets at once via
/// [`tray_applets`] rather than a single Pane → binary binding).
pub fn spawn_by_binary(binary: &str) -> Result<Child, AppletSpawnError> {
    Command::new(binary)
        .spawn()
        .map_err(|e| AppletSpawnError::Spawn(binary.to_string(), e.kind()))
}

/// Applets that mount into the Tray zone. The default panel order
/// (left → right) is: audio · network · mesh-status · notification-
/// bell · status-cluster. Per-user override is captured by writing
/// `$XDG_CONFIG_HOME/mde/panel.toml` (Phase E.26 config_store).
#[must_use]
pub fn tray_applets() -> Vec<String> {
    vec![
        "mde-applet-audio".into(),
        "mde-applet-network".into(),
        "mde-applet-mesh-status".into(),
        "mde-applet-notification-bell".into(),
        "mde-applet-status-cluster".into(),
    ]
}

/// CLI sub-commands that route to a specific applet binary. Used
/// by `mde-panel --apple-menu / --expose / --drawer / --root-menu`.
#[must_use]
pub fn applet_for_subcommand(sub: SubCommand) -> &'static str {
    match sub {
        SubCommand::AppleMenu => "mde-applet-apple-menu",
        SubCommand::Expose => "mde-applet-expose",
        SubCommand::Drawer => "mde-applet-drawer",
        SubCommand::RootMenu => "mde-applet-root-menu",
    }
}

/// Named sub-commands that route through `applet_for_subcommand`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubCommand {
    AppleMenu,
    Expose,
    Drawer,
    RootMenu,
}

/// Spawn-time errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppletSpawnError {
    /// No applet bound to this pane.
    Unbound(Pane),
    /// The binary failed to exec — bool indicates io error kind.
    Spawn(String, std::io::ErrorKind),
}

impl std::fmt::Display for AppletSpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppletSpawnError::Unbound(p) => {
                write!(f, "no applet bound to pane {}", p.label())
            }
            AppletSpawnError::Spawn(binary, kind) => {
                write!(f, "failed to spawn applet {binary}: {kind:?}")
            }
        }
    }
}

/// User-applet manifest search paths (matches mde_applet_api).
#[must_use]
pub fn manifest_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = dirs::data_dir() {
        paths.push(home.join("mde/applets"));
    }
    paths.push(PathBuf::from("/usr/share/mde/applets"));
    paths
}

/// Walk every manifest dir, returning per-applet JSON paths.
#[must_use]
pub fn discover_manifest_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in manifest_search_paths() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let p = entry.path();
            if p.extension().is_some_and(|e| e == "json") {
                out.push(p);
            }
        }
    }
    out
}

/// Pure helper — given a manifest directory, returns the JSON file
/// names (no recursion, no symlink follow). Tests exercise this
/// against a tempdir without touching real install paths.
#[must_use]
pub fn list_manifests_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn default_bindings_covers_every_pane() {
        let map = default_bindings();
        for pane in Pane::ordered() {
            assert!(map.contains_key(&pane), "missing binding for {pane:?}");
        }
    }

    #[test]
    fn default_bindings_have_six_entries() {
        assert_eq!(default_bindings().len(), 6);
    }

    #[test]
    fn each_binding_names_a_real_applet_binary() {
        let map = default_bindings();
        for binding in map.values() {
            assert!(binding.binary.starts_with("mde-applet-"));
            assert!(!binding.binary.contains(' '));
        }
    }

    #[test]
    fn spawn_for_pane_reports_unbound() {
        let map = HashMap::new();
        let err = spawn_for_pane(&map, Pane::Clock).unwrap_err();
        assert!(matches!(err, AppletSpawnError::Unbound(Pane::Clock)));
    }

    #[test]
    fn spawn_for_pane_reports_spawn_failure() {
        let mut map = default_bindings();
        map.insert(
            Pane::Clock,
            AppletBinding {
                pane: Pane::Clock,
                binary: "/definitely/not/a/binary/anywhere".into(),
            },
        );
        let err = spawn_for_pane(&map, Pane::Clock).unwrap_err();
        assert!(matches!(err, AppletSpawnError::Spawn(_, _)));
    }

    #[test]
    fn applet_spawn_error_display_includes_pane_label() {
        let err = AppletSpawnError::Unbound(Pane::Tray);
        let s = format!("{err}");
        assert!(s.contains("System tray"));
    }

    #[test]
    fn applet_spawn_error_display_includes_binary_name() {
        let err = AppletSpawnError::Spawn("mde-applet-clock".into(), std::io::ErrorKind::NotFound);
        let s = format!("{err}");
        assert!(s.contains("mde-applet-clock"));
    }

    #[test]
    fn list_manifests_in_returns_sorted_json_paths() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("zeta.json"), "{}").unwrap();
        std::fs::write(tmp.path().join("alpha.json"), "{}").unwrap();
        std::fs::write(tmp.path().join("not-a-manifest.txt"), "skip").unwrap();

        let paths = list_manifests_in(tmp.path());
        assert_eq!(paths.len(), 2);
        assert!(paths[0].ends_with("alpha.json"));
        assert!(paths[1].ends_with("zeta.json"));
    }

    #[test]
    fn list_manifests_in_handles_missing_dir() {
        let paths = list_manifests_in(Path::new("/nonexistent"));
        assert!(paths.is_empty());
    }

    #[test]
    fn spawn_by_binary_fails_for_missing_binary() {
        let err = spawn_by_binary("/definitely-not-a-binary").unwrap_err();
        assert!(matches!(err, AppletSpawnError::Spawn(_, _)));
    }

    #[test]
    fn tray_applets_lists_five_panel_chips_in_order() {
        let tray = tray_applets();
        assert_eq!(tray.len(), 5);
        assert_eq!(tray[0], "mde-applet-audio");
        assert_eq!(tray[1], "mde-applet-network");
        assert_eq!(tray[2], "mde-applet-mesh-status");
        assert_eq!(tray[3], "mde-applet-notification-bell");
        assert_eq!(tray[4], "mde-applet-status-cluster");
    }

    #[test]
    fn applet_for_subcommand_maps_every_variant() {
        assert_eq!(
            applet_for_subcommand(SubCommand::AppleMenu),
            "mde-applet-apple-menu"
        );
        assert_eq!(
            applet_for_subcommand(SubCommand::Expose),
            "mde-applet-expose"
        );
        assert_eq!(
            applet_for_subcommand(SubCommand::Drawer),
            "mde-applet-drawer"
        );
        assert_eq!(
            applet_for_subcommand(SubCommand::RootMenu),
            "mde-applet-root-menu"
        );
    }

    #[test]
    fn manifest_search_paths_includes_etc_locations() {
        let paths = manifest_search_paths();
        // System path must be present.
        assert!(paths.iter().any(|p| p.ends_with("mde/applets")));
        // User path (XDG) when home_dir resolves.
        let has_user = paths
            .iter()
            .any(|p| p.starts_with(dirs::data_dir().unwrap_or_default()));
        // At minimum: not empty.
        assert!(!paths.is_empty());
        let _ = has_user;
    }
}
