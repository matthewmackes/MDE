//! Super+Tab app-switcher overlay.
//!
//! Phase E1.2.11: reads `swaymsg -t get_tree` to enumerate
//! windows in MRU order, renders a horizontal strip of
//! window thumbnails. Today the strip is text-only (one row
//! per window); the screenshot-thumbnail integration via
//! grim ships at Phase E.4.3 when the host gets layer-shell
//! support.

#![forbid(unsafe_code)]

use mde_applet_api::{AppletId, AppletSlot, HostMessage};
use serde::Deserialize;

#[must_use]
pub fn manifest() -> mde_applet_api::AppletManifest {
    mde_applet_api::AppletManifest {
        id: AppletId::from_static("app-switcher"),
        binary: "mde-applet-app-switcher".into(),
        slot: AppletSlot::Overlay,
        summary: "Super+Tab window switcher".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

/// One window row the switcher renders. Only the fields the
/// strip cares about.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct WindowRow {
    /// sway container id.
    #[serde(default)]
    pub id: u64,
    /// Window title — usually `WM_NAME`.
    #[serde(default)]
    pub name: String,
    /// X11/Wayland app id / WM_CLASS.
    #[serde(default)]
    pub app_id: String,
    /// `true` if the window is currently focused.
    #[serde(default)]
    pub focused: bool,
}

/// Walk a sway `get_tree` JSON payload, collecting every
/// leaf window (type "con" with a "name"). Returns rows in
/// the order sway emits them, which for fresh trees matches
/// recency.
#[must_use]
pub fn parse_windows(tree_json: &str) -> Vec<WindowRow> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(tree_json) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    walk(&v, &mut out);
    out
}

fn walk(node: &serde_json::Value, out: &mut Vec<WindowRow>) {
    let Some(obj) = node.as_object() else {
        return;
    };
    // Leaves: "con" with no nodes + a non-empty name.
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
        out.push(WindowRow {
            id: obj.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
            name: name.to_string(),
            app_id: obj
                .get("app_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            focused: obj
                .get("focused")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
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

/// Format the switcher strip as one line per window:
/// `<focus-marker> <app_id> · <truncated-name>`.
#[must_use]
pub fn format_strip(rows: &[WindowRow]) -> String {
    if rows.is_empty() {
        return "(no windows)".to_string();
    }
    rows.iter()
        .map(|w| {
            let marker = if w.focused { "▶" } else { " " };
            let name: String = w.name.chars().take(48).collect();
            let app = if w.app_id.is_empty() { "?" } else { &w.app_id };
            format!("{marker} {app} · {name}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[must_use]
pub fn handle_host(msg: &HostMessage) -> bool {
    !matches!(msg, HostMessage::Shutdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_lands_in_overlay_slot() {
        let m = manifest();
        assert_eq!(m.id.as_str(), "app-switcher");
        assert_eq!(m.slot, AppletSlot::Overlay);
    }

    #[test]
    fn parse_windows_walks_a_typical_tree() {
        let tree = r#"{
            "type": "root",
            "nodes": [
                {
                    "type": "output",
                    "nodes": [
                        {
                            "type": "workspace",
                            "nodes": [
                                {
                                    "type": "con",
                                    "id": 5,
                                    "name": "Firefox",
                                    "app_id": "firefox",
                                    "focused": true,
                                    "nodes": [],
                                    "floating_nodes": []
                                },
                                {
                                    "type": "con",
                                    "id": 6,
                                    "name": "Terminal",
                                    "app_id": "foot",
                                    "focused": false,
                                    "nodes": [],
                                    "floating_nodes": []
                                }
                            ]
                        }
                    ]
                }
            ]
        }"#;
        let rows = parse_windows(tree);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].app_id, "firefox");
        assert!(rows[0].focused);
        assert_eq!(rows[1].app_id, "foot");
        assert!(!rows[1].focused);
    }

    #[test]
    fn parse_windows_returns_empty_on_garbage() {
        assert!(parse_windows("").is_empty());
        assert!(parse_windows("not json").is_empty());
    }

    #[test]
    fn format_strip_empty_message() {
        assert_eq!(format_strip(&[]), "(no windows)");
    }

    #[test]
    fn format_strip_marks_focused_row() {
        let rows = vec![
            WindowRow {
                id: 1,
                name: "Firefox".into(),
                app_id: "firefox".into(),
                focused: true,
            },
            WindowRow {
                id: 2,
                name: "Terminal".into(),
                app_id: "foot".into(),
                focused: false,
            },
        ];
        let s = format_strip(&rows);
        let lines: Vec<&str> = s.lines().collect();
        assert!(lines[0].starts_with("▶"));
        assert!(lines[1].starts_with(" "));
        assert!(s.contains("firefox"));
        assert!(s.contains("foot"));
    }

    #[test]
    fn format_strip_handles_empty_app_id() {
        let rows = vec![WindowRow {
            id: 1,
            name: "Plain".into(),
            app_id: "".into(),
            focused: false,
        }];
        assert!(format_strip(&rows).contains("?"));
    }

    #[test]
    fn handle_host_short_circuits_shutdown() {
        assert!(!handle_host(&HostMessage::Shutdown));
    }
}
