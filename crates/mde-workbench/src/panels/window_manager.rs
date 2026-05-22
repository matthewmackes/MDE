//! System → Window Manager panel — sway-mode controls for
//! inner/outer gaps + the default workspace layout.
//!
//! CB-1.9.c: replaces the sway branch of the v1.x
//! `mackes/workbench/system/window_manager.py`. The Python
//! panel was two-mode (xfwm4 vs i3/sway); v2.0.0's Wayland-
//! only target retires xfwm4 entirely, so this Iced port
//! ships only the sway mode.
//!
//! All three controls are runtime sway IPC commands routed
//! through `swaymsg` (matches the v1.x pattern + the Phase E
//! lock for mde-panel's sway integration). Persistence to
//! `~/.config/sway/config` is a Phase C applier job — for
//! now Apply propagates immediately to the running session
//! and the user re-applies after a sway restart. The
//! follow-up "CB-1.9.c follow-up: persist sway settings to
//! config file" captures the missing piece.

use iced::widget::{column, pick_list, row, text, text_input};
use iced::{Element, Length, Task};
use mde_theme::Palette;
use tokio::process::Command;

use crate::controls::{variant_button, ButtonVariant};

/// Layout values the sway IPC `layout` command accepts at the
/// container level.
pub const LAYOUTS: &[&str] = &["splith", "splitv", "tabbed", "stacking"];

#[derive(Debug, Clone, Default)]
pub struct WindowManagerPanel {
    /// Whether `swaymsg` returned a usable result on the last
    /// load. Drives the empty-state body.
    pub sway_available: bool,
    pub inner_gaps_input: String,
    pub outer_gaps_input: String,
    pub layout: String,
    pub status: String,
    pub busy: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded {
        sway_available: bool,
        inner_gaps: u32,
        outer_gaps: u32,
        layout: String,
    },
    Error(String),
    InnerGapsChanged(String),
    OuterGapsChanged(String),
    LayoutChanged(String),
    ApplyClicked,
    /// Sway IPC applied + persistence attempt complete. The
    /// inner Result reports whether the drop-in config write
    /// succeeded (Ok with the file path) or failed (Err with
    /// a friendly message — the runtime change still took
    /// effect either way).
    Applied(Result<String, String>),
}

impl WindowManagerPanel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            layout: "splith".to_string(),
            inner_gaps_input: "0".to_string(),
            outer_gaps_input: "0".to_string(),
            ..Self::default()
        }
    }

    pub fn load() -> Task<crate::Message> {
        Task::perform(
            async move {
                let probe = run_swaymsg(&["-t", "get_version"]).await;
                let sway_available = !probe.is_empty();
                if !sway_available {
                    return Message::Loaded {
                        sway_available,
                        inner_gaps: 0,
                        outer_gaps: 0,
                        layout: "splith".into(),
                    };
                }
                // sway has no `get_config` IPC type, so we read the
                // current focused-workspace layout via get_tree.
                let tree = run_swaymsg(&["-t", "get_tree"]).await;
                Message::Loaded {
                    sway_available,
                    // Persistence isn't wired yet (see follow-up);
                    // surface 0/0 as the starting guess + let the
                    // user type the values they want.
                    inner_gaps: 0,
                    outer_gaps: 0,
                    layout: focused_workspace_layout(&tree).unwrap_or_else(|| "splith".to_string()),
                }
            },
            crate::Message::WindowManager,
        )
    }

    pub fn update(&mut self, message: Message) -> Task<crate::Message> {
        match message {
            Message::Loaded {
                sway_available,
                inner_gaps,
                outer_gaps,
                layout,
            } => {
                self.sway_available = sway_available;
                self.inner_gaps_input = inner_gaps.to_string();
                self.outer_gaps_input = outer_gaps.to_string();
                self.layout = if LAYOUTS.contains(&layout.as_str()) {
                    layout
                } else {
                    "splith".to_string()
                };
                self.status.clear();
                self.busy = false;
                Task::none()
            }
            Message::Error(msg) => {
                self.status = msg;
                self.busy = false;
                Task::none()
            }
            Message::InnerGapsChanged(v) => {
                self.inner_gaps_input = v;
                Task::none()
            }
            Message::OuterGapsChanged(v) => {
                self.outer_gaps_input = v;
                Task::none()
            }
            Message::LayoutChanged(v) => {
                self.layout = v;
                Task::none()
            }
            Message::ApplyClicked => {
                if self.busy {
                    return Task::none();
                }
                let inner = match parse_gap(&self.inner_gaps_input) {
                    Ok(v) => v,
                    Err(msg) => {
                        self.status = msg;
                        return Task::none();
                    }
                };
                let outer = match parse_gap(&self.outer_gaps_input) {
                    Ok(v) => v,
                    Err(msg) => {
                        self.status = msg;
                        return Task::none();
                    }
                };
                self.busy = true;
                self.status = "Applying…".into();
                let layout = self.layout.clone();
                Task::perform(
                    async move {
                        let _ = run_swaymsg(&[&format!("gaps inner all set {inner}")]).await;
                        let _ = run_swaymsg(&[&format!("gaps outer all set {outer}")]).await;
                        let _ = run_swaymsg(&[&format!("layout {layout}")]).await;
                        // CB-1.9.c follow-up — persist to the
                        // drop-in config so settings survive a
                        // sway restart.
                        let persist = write_sway_overrides(inner, outer, &layout).await;
                        Message::Applied(persist)
                    },
                    crate::Message::WindowManager,
                )
            }
            Message::Applied(persist) => {
                self.status = match persist {
                    Ok(path) => format!("Applied + persisted to {path}."),
                    Err(msg) => format!("Applied to running sway; persistence failed: {msg}"),
                };
                self.busy = false;
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, crate::Message> {
        if !self.sway_available {
            return column![
                text("sway IPC unavailable").size(18),
                text(
                    "MDE talks to sway through `swaymsg`. Launch the MDE \
                     session (sway is the active compositor) and reopen \
                     this panel.",
                )
                .size(13),
            ]
            .spacing(8)
            .width(Length::Fill)
            
            .into();
        }

        let apply_label = if self.busy { "Applying…" } else { "Apply" };
        // UX-7.a — apply routed through the shared button variant.
        let apply_btn = variant_button(
            apply_label,
            ButtonVariant::Primary,
            (!self.busy).then(|| crate::Message::WindowManager(Message::ApplyClicked)),
            Palette::dark(),
        );

        let layout_pick: pick_list::PickList<'_, &'static str, _, _, crate::Message> = pick_list(
            LAYOUTS,
            LAYOUTS.iter().copied().find(|l| *l == self.layout),
            |v| crate::Message::WindowManager(Message::LayoutChanged(v.to_string())),
        );

        column![
            row![
                text("Inner gaps (px)").width(Length::Fixed(180.0)),
                text_input("0", &self.inner_gaps_input)
                    .on_input(|v| crate::Message::WindowManager(Message::InnerGapsChanged(v))),
            ]
            .spacing(12),
            row![
                text("Outer gaps (px)").width(Length::Fixed(180.0)),
                text_input("0", &self.outer_gaps_input)
                    .on_input(|v| crate::Message::WindowManager(Message::OuterGapsChanged(v))),
            ]
            .spacing(12),
            row![
                text("Default layout").width(Length::Fixed(180.0)),
                layout_pick,
            ]
            .spacing(12),
            row![apply_btn, text(&self.status).size(13)].spacing(12),
        ]
        .spacing(12)
        .width(Length::Fill)
        
        .into()
    }
}

/// Parse a gap-pixel input. Empty → 0 (matches the v1.x "off"
/// semantics for sway gap settings). Negative + non-numeric
/// surface a validation error.
fn parse_gap(input: &str) -> Result<u32, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(0);
    }
    trimmed
        .parse::<u32>()
        .map_err(|_| "Gap must be a non-negative integer.".to_string())
}

/// Pull the focused workspace's layout from a `swaymsg -t
/// get_tree` JSON payload. Returns `None` when no workspace is
/// focused (e.g. fresh sway boot before any workspace exists).
#[must_use]
pub fn focused_workspace_layout(tree_json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(tree_json).ok()?;
    // Two-pass: prefer a focused workspace, fall back to the
    // first workspace in tree order so the panel has a value
    // to surface on a fresh sway boot.
    let chosen = find_workspace(&v, true).or_else(|| find_workspace(&v, false))?;
    chosen
        .get("layout")
        .and_then(|l| l.as_str())
        .map(str::to_string)
}

/// Depth-first search the sway tree for a workspace node. When
/// `focused_only` is true, only return workspaces with
/// `focused: true`; otherwise return the first workspace in
/// traversal order.
fn find_workspace(node: &serde_json::Value, focused_only: bool) -> Option<&serde_json::Value> {
    let obj = node.as_object()?;
    if obj.get("type").and_then(|t| t.as_str()) == Some("workspace") {
        if !focused_only {
            return Some(node);
        }
        if obj
            .get("focused")
            .and_then(|f| f.as_bool())
            .unwrap_or(false)
        {
            return Some(node);
        }
    }
    if let Some(arr) = obj.get("nodes").and_then(|n| n.as_array()) {
        for child in arr {
            if let Some(found) = find_workspace(child, focused_only) {
                return Some(found);
            }
        }
    }
    None
}

/// Shell out to `swaymsg` with the given args. Empty string on
/// any error — the caller uses that as the "sway not running"
/// signal.
/// CB-1.9.c follow-up — write the user's gaps + layout
/// choices to `~/.config/sway/config.d/mde-overrides.conf`
/// so they survive a sway restart. The directory is sourced
/// by the conventional `include $HOME/.config/sway/config.d/*`
/// at the bottom of the user's sway config — if they haven't
/// added that include, the persist still succeeds but the
/// values stay runtime-only across restarts.
///
/// Returns the absolute path on success or a human-readable
/// error message on failure.
pub async fn write_sway_overrides(
    inner_gaps: u32,
    outer_gaps: u32,
    layout: &str,
) -> Result<String, String> {
    let home = std::env::var("HOME").map_err(|e| format!("$HOME unset: {e}"))?;
    let dir = std::path::PathBuf::from(home)
        .join(".config")
        .join("sway")
        .join("config.d");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("mkdir {}: {e}", dir.display()))?;
    let path = dir.join("mde-overrides.conf");
    let body = sway_overrides_body(inner_gaps, outer_gaps, layout);
    tokio::fs::write(&path, body)
        .await
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path.to_string_lossy().into_owned())
}

/// Pure formatter for the drop-in config body. Kept testable
/// so we can guarantee the syntax sway expects without an
/// I/O round-trip.
#[must_use]
pub fn sway_overrides_body(inner_gaps: u32, outer_gaps: u32, layout: &str) -> String {
    format!(
        "# Generated by MDE Workbench (System → Window Manager).
# Edits here are overwritten on the next Apply.

gaps inner all set {inner_gaps}
gaps outer all set {outer_gaps}
default_orientation horizontal
workspace_layout {layout}
"
    )
}

pub async fn run_swaymsg(args: &[&str]) -> String {
    let Ok(output) = Command::new("swaymsg").args(args).output().await else {
        return String::new();
    };
    if !output.status.success() {
        return String::new();
    }
    String::from_utf8(output.stdout).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_lock_matches_sway_ipc_values() {
        assert_eq!(LAYOUTS, &["splith", "splitv", "tabbed", "stacking"]);
    }

    #[test]
    fn parse_gap_accepts_empty_zero_and_positive() {
        assert_eq!(parse_gap(""), Ok(0));
        assert_eq!(parse_gap("   "), Ok(0));
        assert_eq!(parse_gap("12"), Ok(12));
    }

    #[test]
    fn parse_gap_rejects_garbage_and_negatives() {
        assert!(parse_gap("forever").is_err());
        assert!(parse_gap("-1").is_err());
        assert!(parse_gap("3.14").is_err());
    }

    #[test]
    fn focused_workspace_layout_extracts_focused_workspace() {
        let tree = r#"{
            "type": "root",
            "nodes": [
                {
                    "type": "output",
                    "nodes": [
                        {
                            "type": "workspace",
                            "focused": false,
                            "layout": "splith"
                        },
                        {
                            "type": "workspace",
                            "focused": true,
                            "layout": "tabbed"
                        }
                    ]
                }
            ]
        }"#;
        assert_eq!(focused_workspace_layout(tree), Some("tabbed".into()));
    }

    #[test]
    fn focused_workspace_layout_falls_back_to_first_workspace_when_none_focused() {
        let tree = r#"{
            "type": "root",
            "nodes": [{
                "type": "workspace",
                "focused": false,
                "layout": "splitv"
            }]
        }"#;
        assert_eq!(focused_workspace_layout(tree), Some("splitv".into()));
    }

    #[test]
    fn focused_workspace_layout_none_on_garbage_or_no_workspace() {
        assert_eq!(focused_workspace_layout(""), None);
        assert_eq!(focused_workspace_layout("not json"), None);
        assert_eq!(focused_workspace_layout("{\"type\": \"root\"}"), None);
    }

    #[test]
    fn loaded_records_state_and_rejects_unknown_layout() {
        let mut panel = WindowManagerPanel::new();
        let _ = panel.update(Message::Loaded {
            sway_available: true,
            inner_gaps: 4,
            outer_gaps: 0,
            layout: "totally-bogus".into(),
        });
        assert!(panel.sway_available);
        assert_eq!(panel.inner_gaps_input, "4");
        // Unknown layouts fall back to splith.
        assert_eq!(panel.layout, "splith");
    }

    #[test]
    fn loaded_with_known_layout_preserves_it() {
        let mut panel = WindowManagerPanel::new();
        let _ = panel.update(Message::Loaded {
            sway_available: true,
            inner_gaps: 8,
            outer_gaps: 16,
            layout: "tabbed".into(),
        });
        assert_eq!(panel.layout, "tabbed");
        assert_eq!(panel.outer_gaps_input, "16");
    }

    #[test]
    fn loaded_sway_unavailable_clears_state() {
        let mut panel = WindowManagerPanel::new();
        let _ = panel.update(Message::Loaded {
            sway_available: false,
            inner_gaps: 0,
            outer_gaps: 0,
            layout: "splith".into(),
        });
        assert!(!panel.sway_available);
    }

    #[test]
    fn apply_clicked_with_garbage_gap_surfaces_validation() {
        let mut panel = WindowManagerPanel::new();
        panel.sway_available = true;
        panel.inner_gaps_input = "forever".into();
        let _ = panel.update(Message::ApplyClicked);
        assert!(panel.status.contains("integer"));
        assert!(!panel.busy);
    }

    #[test]
    fn apply_clicked_while_busy_is_noop() {
        let mut panel = WindowManagerPanel::new();
        panel.busy = true;
        panel.status = "Applying…".into();
        let _ = panel.update(Message::ApplyClicked);
        assert_eq!(panel.status, "Applying…");
    }

    #[test]
    fn applied_clears_busy_and_records_status() {
        let mut panel = WindowManagerPanel::new();
        panel.busy = true;
        panel.status = "Applying…".into();
        let _ = panel.update(Message::Applied(Ok(
            "/home/u/.config/sway/config.d/mde-overrides.conf".into(),
        )));
        assert!(!panel.busy);
        assert!(panel.status.contains("Applied"));
        assert!(panel.status.contains("persisted"));
    }

    #[test]
    fn applied_with_persist_error_still_clears_busy() {
        let mut panel = WindowManagerPanel::new();
        panel.busy = true;
        let _ = panel.update(Message::Applied(Err("perm denied".into())));
        assert!(!panel.busy);
        assert!(panel.status.contains("Applied to running sway"));
        assert!(panel.status.contains("perm denied"));
    }

    #[test]
    fn sway_overrides_body_contains_each_setting() {
        let body = sway_overrides_body(8, 12, "tabbed");
        assert!(body.contains("gaps inner all set 8"));
        assert!(body.contains("gaps outer all set 12"));
        assert!(body.contains("workspace_layout tabbed"));
        assert!(body.starts_with("# Generated by MDE Workbench"));
    }

    #[test]
    fn input_messages_mutate_matching_fields() {
        let mut panel = WindowManagerPanel::new();
        let _ = panel.update(Message::InnerGapsChanged("12".into()));
        assert_eq!(panel.inner_gaps_input, "12");
        let _ = panel.update(Message::OuterGapsChanged("4".into()));
        assert_eq!(panel.outer_gaps_input, "4");
        let _ = panel.update(Message::LayoutChanged("stacking".into()));
        assert_eq!(panel.layout, "stacking");
    }

    #[test]
    fn error_message_clears_busy_and_stores_msg() {
        let mut panel = WindowManagerPanel::new();
        panel.busy = true;
        let _ = panel.update(Message::Error("swaymsg not on PATH".into()));
        assert_eq!(panel.status, "swaymsg not on PATH");
        assert!(!panel.busy);
    }
}
