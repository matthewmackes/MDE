//! Taskbar dock — bottom-bar applet that surfaces open
//! windows + pinned apps.
//!
//! Phase E1.2.7: companion to the app-switcher (E1.2.11).
//! The switcher is the Super+Tab overlay; the dock is the
//! always-visible bottom-bar strip. Same data source
//! (`swaymsg -t get_tree`) but with a different render
//! shape — one cell per window with a focus indicator,
//! plus pinned-app slots from
//! `~/.config/mde/dock-pinned.json` (read but writes
//! come from the dock-DnD applet in Phase E.9).

#![forbid(unsafe_code)]

use std::path::PathBuf;

use mde_applet_api::{AppletId, AppletSlot, HostMessage};
use serde::Deserialize;

#[must_use]
pub fn manifest() -> mde_applet_api::AppletManifest {
    mde_applet_api::AppletManifest {
        id: AppletId::from_static("dock"),
        binary: "mde-applet-dock".into(),
        slot: AppletSlot::Dock,
        summary: "Bottom-bar taskbar + pinned-app strip".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

/// One window cell in the dock. Subset of the
/// app-switcher's WindowRow that matches dock-render
/// needs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct DockWindow {
    pub id: u64,
    pub app_id: String,
    pub focused: bool,
    pub urgent: bool,
}

/// One pinned-app row, read from
/// `~/.config/mde/dock-pinned.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct PinnedApp {
    /// `.desktop` file basename (e.g. `firefox.desktop`).
    pub desktop_id: String,
    /// Display label.
    pub label: String,
}

#[must_use]
pub fn pinned_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".config/mde/dock-pinned.json")
}

/// Pure parser for the pinned-apps JSON.
#[must_use]
pub fn parse_pinned(raw: &str) -> Vec<PinnedApp> {
    serde_json::from_str(raw).unwrap_or_default()
}

/// Walk a sway `get_tree` payload, collecting every leaf
/// window into a `DockWindow`. Same algorithm as the
/// app-switcher with fewer fields.
#[must_use]
pub fn parse_windows(tree_json: &str) -> Vec<DockWindow> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(tree_json) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    walk(&v, &mut out);
    out
}

fn walk(node: &serde_json::Value, out: &mut Vec<DockWindow>) {
    let Some(obj) = node.as_object() else {
        return;
    };
    let kind = obj.get("type").and_then(|v| v.as_str());
    let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let nodes_empty = obj
        .get("nodes")
        .and_then(|n| n.as_array())
        .map_or(true, |a| a.is_empty());
    let floating_empty = obj
        .get("floating_nodes")
        .and_then(|n| n.as_array())
        .map_or(true, |a| a.is_empty());
    if kind == Some("con") && !name.is_empty() && nodes_empty && floating_empty {
        out.push(DockWindow {
            id: obj.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
            app_id: obj
                .get("app_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            focused: obj
                .get("focused")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            urgent: obj.get("urgent").and_then(|v| v.as_bool()).unwrap_or(false),
        });
    }
    for child in obj
        .get("nodes")
        .and_then(|n| n.as_array())
        .into_iter()
        .flatten()
    {
        walk(child, out);
    }
    for child in obj
        .get("floating_nodes")
        .and_then(|n| n.as_array())
        .into_iter()
        .flatten()
    {
        walk(child, out);
    }
}

/// Render the dock strip — one cell per pinned app + one
/// cell per running window. Cell format:
/// `[<focus> <urgent> <app_id>]`. Pinned-but-not-running
/// apps show with a dim "·" marker; running pinned apps
/// show as a single cell (no duplicate). Focus marker
/// "▶" / urgent marker "!" / pinned marker "•".
#[must_use]
pub fn format_dock(pinned: &[PinnedApp], windows: &[DockWindow]) -> String {
    use std::collections::HashSet;
    let pinned_app_ids: HashSet<&str> = pinned
        .iter()
        .map(|p| p.desktop_id.trim_end_matches(".desktop"))
        .collect();
    let running_app_ids: HashSet<&str> = windows.iter().map(|w| w.app_id.as_str()).collect();

    let mut cells: Vec<String> = Vec::new();

    // Pinned-but-not-running: dim marker.
    for p in pinned {
        let bare = p.desktop_id.trim_end_matches(".desktop");
        if !running_app_ids.contains(bare) {
            cells.push(format!("[· {}]", p.label));
        }
    }
    // Running windows: focus / urgent markers + pinned
    // marker.
    for w in windows {
        let mut markers = String::new();
        if w.focused {
            markers.push('▶');
        }
        if w.urgent {
            markers.push('!');
        }
        if pinned_app_ids.contains(w.app_id.as_str()) {
            markers.push('•');
        }
        if markers.is_empty() {
            markers.push(' ');
        }
        let app = if w.app_id.is_empty() { "?" } else { &w.app_id };
        cells.push(format!("[{markers} {app}]"));
    }
    if cells.is_empty() {
        return "(empty dock)".to_string();
    }
    cells.join(" ")
}

#[must_use]
pub fn handle_host(msg: &HostMessage) -> bool {
    !matches!(msg, HostMessage::Shutdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_lands_in_dock_slot() {
        let m = manifest();
        assert_eq!(m.id.as_str(), "dock");
        assert_eq!(m.slot, AppletSlot::Dock);
    }

    #[test]
    fn parse_pinned_returns_empty_on_garbage() {
        assert!(parse_pinned("").is_empty());
        assert!(parse_pinned("not json").is_empty());
    }

    #[test]
    fn parse_pinned_extracts_desktop_id_and_label() {
        let raw = r#"[
            {"desktop_id": "firefox.desktop", "label": "Firefox"},
            {"desktop_id": "foot.desktop",   "label": "Terminal"}
        ]"#;
        let rows = parse_pinned(raw);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].desktop_id, "firefox.desktop");
        assert_eq!(rows[1].label, "Terminal");
    }

    #[test]
    fn parse_windows_walks_tree() {
        let tree = r#"{
            "type": "root",
            "nodes": [{
                "type": "workspace",
                "nodes": [{
                    "type": "con",
                    "id": 1,
                    "name": "Firefox",
                    "app_id": "firefox",
                    "focused": true,
                    "urgent": false,
                    "nodes": [],
                    "floating_nodes": []
                }]
            }]
        }"#;
        let rows = parse_windows(tree);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].app_id, "firefox");
        assert!(rows[0].focused);
    }

    #[test]
    fn format_dock_empty_message() {
        assert_eq!(format_dock(&[], &[]), "(empty dock)");
    }

    #[test]
    fn format_dock_renders_pinned_then_running() {
        let pinned = vec![
            PinnedApp {
                desktop_id: "firefox.desktop".into(),
                label: "Firefox".into(),
            },
            PinnedApp {
                desktop_id: "foot.desktop".into(),
                label: "Terminal".into(),
            },
        ];
        let windows = vec![DockWindow {
            id: 1,
            app_id: "firefox".into(),
            focused: true,
            urgent: false,
        }];
        let s = format_dock(&pinned, &windows);
        // foot is pinned but not running -> dim cell first.
        // firefox is pinned + running + focused -> ▶• marker.
        assert!(s.contains("[· Terminal]"));
        assert!(s.contains("▶"));
        assert!(s.contains("•"));
        assert!(s.contains("firefox"));
    }

    #[test]
    fn format_dock_marks_urgent_windows() {
        let windows = vec![DockWindow {
            id: 1,
            app_id: "foo".into(), // voice-allow:test-data
            focused: false,
            urgent: true,
        }];
        let s = format_dock(&[], &windows);
        assert!(s.contains("!"));
    }

    #[test]
    fn format_dock_empty_app_id_renders_question_mark() {
        let windows = vec![DockWindow {
            id: 1,
            app_id: "".into(),
            focused: false,
            urgent: false,
        }];
        let s = format_dock(&[], &windows);
        assert!(s.contains("?"));
    }

    #[test]
    fn handle_host_short_circuits_shutdown() {
        assert!(!handle_host(&HostMessage::Shutdown));
    }
}
