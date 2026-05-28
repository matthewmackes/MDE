//! Start-menu popover applet — Win10 left-anchored Start
//! button popover (project_v1_1_0_win10_layout lock).
//!
//! Phase E1.2.8: parses every visible `*.desktop` file
//! under `$XDG_DATA_DIRS/applications/` + per-user
//! `~/.local/share/applications/`, plus the
//! `~/.config/mde/start-pinned` pinned list (TSV
//! `desktop_id\tpane`, where pane is `pinned` or
//! `all`). Renders the popover into three panes:
//! 1. Pinned (top, 3×3 tile grid in the final Iced
//!    view).
//! 2. All Apps (scrollable, alpha-sorted).
//! 3. Search (fuzzy match against Name + Comment).
//!
//! Right-click on any entry surfaces the Material Symbols
//! icon mapper popover so the user can re-skin the launcher
//! glyph (locked 2026-05-19).

#![forbid(unsafe_code)]

use std::path::PathBuf;

use mde_applet_api::{AppletId, AppletSlot, HostMessage};

/// Build the static applet manifest the host registers at
/// startup. Slot = Overlay because the Start popover renders
/// over the panel rather than embedded inside a top-bar slot.
#[must_use]
pub fn manifest() -> mde_applet_api::AppletManifest {
    mde_applet_api::AppletManifest {
        id: AppletId::from_static("start-menu"),
        binary: "mde-applet-start-menu".into(),
        slot: AppletSlot::Overlay,
        summary: "Win10 Start popover — pinned + all apps + search".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

/// One row in the popover, derived from a `.desktop`
/// file. Survives invalid escape sequences + missing
/// optional fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppEntry {
    /// Desktop-id without the `.desktop` suffix —
    /// matches the Plank/pinned-favorites convention.
    pub id: String,
    /// `Name=…` value (already locale-stripped to the
    /// English fallback).
    pub name: String,
    /// `Comment=…` value — empty when absent.
    pub comment: String,
    /// `Exec=…` value — empty when absent.
    pub exec: String,
    /// `Icon=…` value — empty when absent.
    pub icon: String,
    /// `Categories=…` split on `;`. Empty when absent.
    pub categories: Vec<String>,
    /// True when `NoDisplay=true` or `Hidden=true` —
    /// the host should suppress these in the All Apps
    /// pane but allow them in search results.
    pub hidden: bool,
}

/// One pinned row from `~/.config/mde/start-pinned`,
/// stored as `desktop_id\tpane` (pane = `pinned` |
/// `all`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedRow {
    /// Desktop-id without the `.desktop` suffix — matches the
    /// freedesktop application identity convention.
    pub desktop_id: String,
    /// Which Start pane this row belongs to (pinned tiles vs
    /// the all-apps list).
    pub pane: PinnedPane,
}

/// Which Start pane a [`PinnedRow`] is assigned to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinnedPane {
    /// The user's pinned tiles pane.
    Pinned,
    /// The all-apps list pane (the right-hand column in the
    /// Win10-style popover).
    All,
}

/// Absolute path to the `start-pinned` config file. Honors
/// `$XDG_CONFIG_HOME` first, then falls back to
/// `$HOME/.config/mde/start-pinned`. Returns `/var/empty/...`
/// only in the degenerate case where neither env var is set.
#[must_use]
pub fn pinned_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .ok()
        .unwrap_or_else(|| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".config"))
                .unwrap_or_else(|_| PathBuf::from("/var/empty"))
        });
    base.join("mde/start-pinned")
}

/// Parse a single `.desktop` file body (entire file
/// contents as a string). Returns the entry with id =
/// `<base>` (caller supplies the base from the filename).
#[must_use]
pub fn parse_desktop_file(base: &str, raw: &str) -> AppEntry {
    let mut e = AppEntry {
        id: base.to_string(),
        ..Default::default()
    };
    let mut in_main_section = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_main_section = trimmed == "[Desktop Entry]";
            continue;
        }
        if !in_main_section {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        match key.trim() {
            "Name" => e.name = value.trim().to_string(),
            "Comment" => e.comment = value.trim().to_string(),
            "Exec" => e.exec = value.trim().to_string(),
            "Icon" => e.icon = value.trim().to_string(),
            "Categories" => {
                e.categories = value
                    .split(';')
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.trim().to_string())
                    .collect();
            }
            "NoDisplay" | "Hidden" => {
                if value.trim().eq_ignore_ascii_case("true") {
                    e.hidden = true;
                }
            }
            _ => {}
        }
    }
    e
}

/// Parse the TSV pinned-favorites file.
#[must_use]
pub fn parse_pinned(raw: &str) -> Vec<PinnedRow> {
    let mut rows = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = trimmed.splitn(2, '\t').collect();
        let desktop_id = parts[0].to_string();
        let pane = match parts.get(1).copied().unwrap_or("pinned") {
            "all" => PinnedPane::All,
            _ => PinnedPane::Pinned,
        };
        rows.push(PinnedRow { desktop_id, pane });
    }
    rows
}

/// Fuzzy-match for the search pane. Case-insensitive
/// substring against name + comment. Hidden entries
/// are surfaced in search but not in All Apps (handled
/// by the caller).
#[must_use]
pub fn search<'a>(entries: &'a [AppEntry], query: &str) -> Vec<&'a AppEntry> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    entries
        .iter()
        .filter(|e| {
            e.name.to_ascii_lowercase().contains(&q) || e.comment.to_ascii_lowercase().contains(&q)
        })
        .collect()
}

/// All-Apps pane: alpha-sorted by Name, with hidden
/// entries filtered out. Pure-fn; the caller has
/// already loaded everything.
#[must_use]
pub fn all_apps(entries: Vec<AppEntry>) -> Vec<AppEntry> {
    let mut visible: Vec<AppEntry> = entries.into_iter().filter(|e| !e.hidden).collect();
    visible.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
    });
    visible
}

/// Build the pinned pane from a registry + pinned list.
/// Returns one row per pinned desktop-id where the
/// matching `AppEntry` exists. Drops orphans silently.
#[must_use]
pub fn pinned_pane(entries: &[AppEntry], pinned: &[PinnedRow]) -> Vec<AppEntry> {
    pinned
        .iter()
        .filter(|p| matches!(p.pane, PinnedPane::Pinned))
        .filter_map(|p| entries.iter().find(|e| e.id == p.desktop_id).cloned())
        .collect()
}

/// One-line summary of the popover state for the
/// `--now` smoke output (matches the contract on the
/// other applets).
#[must_use]
pub fn format_now(pinned: &[AppEntry], all: &[AppEntry], search_hits: usize) -> String {
    format!(
        "start-menu: {} pinned · {} apps · {} search-hits",
        pinned.len(),
        all.len(),
        search_hits
    )
}

/// Process a host control message and return `true` when the
/// applet should keep running. Only [`HostMessage::Shutdown`]
/// stops the event loop; every other variant is a host-side
/// hint the applet renderer reacts to elsewhere.
#[must_use]
pub fn handle_host(msg: &HostMessage) -> bool {
    !matches!(msg, HostMessage::Shutdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entries() -> Vec<AppEntry> {
        vec![
            AppEntry {
                id: "firefox".into(),
                name: "Firefox".into(),
                comment: "Web Browser".into(),
                exec: "firefox %u".into(),
                icon: "firefox".into(),
                categories: vec!["Network".into(), "WebBrowser".into()],
                hidden: false,
            },
            AppEntry {
                id: "thunar".into(),
                name: "File Manager".into(),
                comment: "Thunar".into(),
                exec: "thunar %F".into(),
                icon: "thunar".into(),
                categories: vec!["FileTools".into()],
                hidden: false,
            },
            AppEntry {
                id: "secret".into(),
                name: "Hidden Tool".into(),
                comment: "Should not show".into(),
                exec: "secret".into(),
                icon: "".into(),
                categories: vec![],
                hidden: true,
            },
        ]
    }

    #[test]
    fn manifest_lands_in_overlay_slot() {
        let m = manifest();
        assert_eq!(m.id.as_str(), "start-menu");
        assert_eq!(m.slot, AppletSlot::Overlay);
    }

    #[test]
    fn parse_desktop_file_extracts_main_section() {
        let raw = r#"[Desktop Entry]
Name=Firefox
Comment=Web Browser
Exec=firefox %u
Icon=firefox
Categories=Network;WebBrowser;
NoDisplay=false

[Desktop Action new-window]
Name=New Window
Exec=firefox --new-window
"#;
        let e = parse_desktop_file("firefox", raw);
        assert_eq!(e.id, "firefox");
        assert_eq!(e.name, "Firefox");
        assert_eq!(e.comment, "Web Browser");
        assert_eq!(e.exec, "firefox %u");
        assert_eq!(e.icon, "firefox");
        assert_eq!(e.categories, vec!["Network", "WebBrowser"]);
        assert!(!e.hidden);
    }

    #[test]
    fn parse_desktop_file_ignores_action_sections() {
        let raw = r#"[Desktop Entry]
Name=App

[Desktop Action a]
Name=action-name
"#;
        let e = parse_desktop_file("app", raw);
        // Must remain "App", not "action-name".
        assert_eq!(e.name, "App");
    }

    #[test]
    fn parse_desktop_file_honors_hidden_and_nodisplay() {
        let raw_a = "[Desktop Entry]\nName=A\nNoDisplay=true\n";
        let raw_b = "[Desktop Entry]\nName=B\nHidden=true\n";
        assert!(parse_desktop_file("a", raw_a).hidden);
        assert!(parse_desktop_file("b", raw_b).hidden);
    }

    #[test]
    fn parse_pinned_splits_pane_field() {
        let raw = "firefox\tpinned\nthunar\tall\nsolo\n# comment\n";
        let rows = parse_pinned(raw);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].desktop_id, "firefox");
        assert_eq!(rows[0].pane, PinnedPane::Pinned);
        assert_eq!(rows[1].pane, PinnedPane::All);
        // `solo` with no pane defaults to Pinned.
        assert_eq!(rows[2].desktop_id, "solo");
        assert_eq!(rows[2].pane, PinnedPane::Pinned);
    }

    #[test]
    fn search_case_insensitive_substring() {
        let entries = sample_entries();
        let hits = search(&entries, "FIRE");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "firefox");
        // Match on Comment field too.
        let by_comment = search(&entries, "thunar");
        assert_eq!(by_comment.len(), 1);
        assert_eq!(by_comment[0].id, "thunar");
    }

    #[test]
    fn search_includes_hidden_entries() {
        let entries = sample_entries();
        let hits = search(&entries, "hidden tool");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "secret");
    }

    #[test]
    fn search_empty_query_yields_empty() {
        let entries = sample_entries();
        assert!(search(&entries, "").is_empty());
        assert!(search(&entries, "   ").is_empty());
    }

    #[test]
    fn all_apps_filters_hidden_and_sorts_alpha() {
        let entries = sample_entries();
        let all = all_apps(entries);
        let names: Vec<&str> = all.iter().map(|e| e.name.as_str()).collect();
        // Only two visible, sorted alpha case-insensitive.
        assert_eq!(names, vec!["File Manager", "Firefox"]);
    }

    #[test]
    fn pinned_pane_drops_orphans() {
        let entries = sample_entries();
        let pinned = vec![
            PinnedRow {
                desktop_id: "firefox".into(),
                pane: PinnedPane::Pinned,
            },
            PinnedRow {
                desktop_id: "nonexistent".into(),
                pane: PinnedPane::Pinned,
            },
            PinnedRow {
                desktop_id: "thunar".into(),
                pane: PinnedPane::All,
            },
        ];
        let p = pinned_pane(&entries, &pinned);
        // Only firefox survives — nonexistent is orphan,
        // thunar is the wrong pane.
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].id, "firefox");
    }

    #[test]
    fn format_now_renders_three_counts() {
        let s = format_now(&[], &[], 0);
        assert!(s.contains("0 pinned"));
        assert!(s.contains("0 apps"));
        assert!(s.contains("0 search-hits"));
    }

    #[test]
    fn handle_host_short_circuits_shutdown() {
        assert!(!handle_host(&HostMessage::Shutdown));
    }
}
