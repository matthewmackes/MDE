//! Portal-5 — swayipc-async workspace integration.
//!
//! Provides `WorkspaceInfo` (the trimmed-down data the Dock needs) and
//! `workspace_subscription()` (an Iced `Subscription` that emits
//! `Message::WorkspaceList` on startup and on every workspace change).
//!
//! Also provides `WindowInfo` (the running-zone data, Portal-8.a) and
//! `window_subscription()` (emits `Message::WindowList` on every window
//! open / close / focus / title change).
//!
//! Two swayipc connections are opened per watcher run:
//!   1. A command connection — used for `get_workspaces()` / `get_tree()` refreshes.
//!   2. An event connection — consumed by `subscribe()`, streams events.
//!
//! If swayipc is unavailable ($SWAYSOCK unset, Sway not running), the
//! subscriptions retry every 3 s without panicking.  The Dock renders
//! empty segments until a connection succeeds.

use futures_util::StreamExt as _;
use iced::Subscription;
use swayipc_async::{Connection, EventType, NodeType};

use crate::app::Message;

/// Adaptive-width floor for workspace cells (R4-Q64).
pub const WORKSPACE_CELL_MIN_PX: f32 = 24.0;

/// Characters above which a workspace name is truncated / marqueed (R4-Q64).
pub const WS_NAME_MAX_CHARS: usize = 8;

// ── Window info (Portal-8.a) ──────────────────────────────────────────────────

/// Trimmed window data the running-zone segment needs (Portal-8.a).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    /// sway container ID — stable within a session.
    pub con_id: i64,
    /// Wayland app_id (e.g. `"foot"`, `"firefox"`), if set.
    pub app_id: Option<String>,
    /// Window title / WM_NAME.
    pub title: Option<String>,
    /// Workspace number the window is currently on (−1 = scratchpad).
    pub workspace_num: i32,
    /// This window has keyboard focus.
    pub focused: bool,
    /// Portal-49 (R12-Q9): marks assigned to this window by sway.
    /// `mark --add <name>` ops land here in insertion order.
    pub marks: Vec<String>,
}

impl WindowInfo {
    /// Short label for the running zone: app_id (first 10 chars), or
    /// the window title (first 10 chars), or `con_id` as fallback.
    pub fn display_label(&self) -> String {
        let raw = self
            .app_id
            .as_deref()
            .or(self.title.as_deref())
            .unwrap_or("?");
        let max = 10;
        if raw.chars().count() > max {
            let prefix: String = raw.chars().take(max).collect();
            format!("{prefix}…")
        } else {
            raw.to_string()
        }
    }
}

/// Recursively collect `WindowInfo` for all tiling leaf windows.
///
/// `ws_num` tracks the workspace number as we descend the tree.
fn collect_windows(node: &swayipc_async::Node, ws_num: i32) -> Vec<WindowInfo> {
    let current_ws_num = if node.node_type == NodeType::Workspace {
        node.num.unwrap_or(ws_num)
    } else {
        ws_num
    };

    let mut windows = Vec::new();
    if node.node_type == NodeType::Con && node.nodes.is_empty() && node.app_id.is_some() {
        windows.push(WindowInfo {
            con_id: node.id,
            app_id: node.app_id.clone(),
            title: node.name.clone(),
            workspace_num: current_ws_num,
            focused: node.focused,
            marks: node.marks.clone(),
        });
    }
    for child in &node.nodes {
        windows.extend(collect_windows(child, current_ws_num));
    }
    windows
}

/// Trimmed workspace data the Dock strip needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceInfo {
    /// Workspace number (−1 for the scratch-pad / unnamed).
    pub num: i32,
    /// Human-readable name (may equal `num.to_string()` for numbered ws).
    pub name: String,
    /// This workspace has keyboard focus.
    pub focused: bool,
    /// This workspace is visible on some output.
    pub visible: bool,
    /// Output the workspace is assigned to (e.g. `"HDMI-A-1"`).
    pub output: String,
    /// At least one window has the urgent hint.
    pub urgent: bool,
}

impl From<swayipc_async::Workspace> for WorkspaceInfo {
    fn from(ws: swayipc_async::Workspace) -> Self {
        WorkspaceInfo {
            num: ws.num,
            name: ws.name,
            focused: ws.focused,
            visible: ws.visible,
            output: ws.output,
            urgent: ws.urgent,
        }
    }
}

impl WorkspaceInfo {
    /// Display label: raw number if `name == num.to_string()`, else
    /// truncate to 8 chars + `…`.
    pub fn display_label(&self) -> String {
        if self.name == self.num.to_string() {
            self.name.clone()
        } else if self.name.chars().count() > WS_NAME_MAX_CHARS {
            let prefix: String = self.name.chars().take(WS_NAME_MAX_CHARS).collect();
            format!("{prefix}…")
        } else {
            self.name.clone()
        }
    }
}

/// Iced `Subscription` that emits `Message::WorkspaceList` on startup and
/// on every workspace-change event from i3/sway.
///
/// Uses an async-stream generator so the event loop is ergonomic and the
/// stream is lazy (starts only when iced's runtime runs it).
pub fn workspace_subscription() -> Subscription<Message> {
    Subscription::run_with_id(
        "mde-portal-workspaces",
        async_stream::stream! {
            loop {
                // Open command connection (for get_workspaces refreshes).
                let cmd_conn = match Connection::new().await {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::debug!(error = %e, "swayipc cmd connect failed; retrying in 3s");
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        continue;
                    }
                };
                let mut conn = cmd_conn;

                // Emit initial workspace list.
                if let Ok(wss) = conn.get_workspaces().await {
                    let infos: Vec<WorkspaceInfo> =
                        wss.into_iter().map(WorkspaceInfo::from).collect();
                    yield Message::WorkspaceList(infos);
                }

                // Open event connection (separate; subscribe() consumes self).
                let event_conn = match Connection::new().await {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::debug!(error = %e, "swayipc event connect failed; retrying in 3s");
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        continue;
                    }
                };

                let mut events = match event_conn.subscribe([EventType::Workspace]).await {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::debug!(error = %e, "workspace subscribe failed; retrying in 3s");
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        continue;
                    }
                };

                // Forward workspace-change events as WorkspaceList updates.
                while let Some(event_result) = events.next().await {
                    if let Ok(swayipc_async::Event::Workspace(_)) = event_result {
                        if let Ok(wss) = conn.get_workspaces().await {
                            let infos: Vec<WorkspaceInfo> =
                                wss.into_iter().map(WorkspaceInfo::from).collect();
                            yield Message::WorkspaceList(infos);
                        }
                    }
                }

                // Event stream ended (sway disconnected) — loop retries immediately.
                tracing::debug!("swayipc event stream ended; reconnecting");
            }
        },
    )
}

/// Iced `Subscription` that emits `Message::WindowList` on startup and on
/// every window event (open / close / focus / title change).
///
/// Uses the same two-connection pattern as `workspace_subscription()`.
pub fn window_subscription() -> Subscription<Message> {
    Subscription::run_with_id(
        "mde-portal-windows",
        async_stream::stream! {
            loop {
                let cmd_conn = match Connection::new().await {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::debug!(error = %e, "swayipc window cmd connect failed; retrying in 3s");
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        continue;
                    }
                };
                let mut conn = cmd_conn;

                // Emit initial window list from tree.
                if let Ok(tree) = conn.get_tree().await {
                    let windows = collect_windows(&tree, -1);
                    yield Message::WindowList(windows);
                }

                let event_conn = match Connection::new().await {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::debug!(error = %e, "swayipc window event connect failed; retrying in 3s");
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        continue;
                    }
                };

                let mut events = match event_conn.subscribe([EventType::Window]).await {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::debug!(error = %e, "window subscribe failed; retrying in 3s");
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        continue;
                    }
                };

                while let Some(event_result) = events.next().await {
                    if let Ok(swayipc_async::Event::Window(_)) = event_result {
                        if let Ok(tree) = conn.get_tree().await {
                            let windows = collect_windows(&tree, -1);
                            yield Message::WindowList(windows);
                        }
                    }
                }

                tracing::debug!("swayipc window event stream ended; reconnecting");
            }
        },
    )
}

/// Portal-45 (R12-Q5) — Iced `Subscription` that emits
/// `Message::ModeChanged(Option<String>)` on every sway binding-mode
/// transition. `None` for the `"default"` mode (no mode segment
/// rendered); `Some(name)` for any other mode (segment renders with
/// that name).
///
/// Reuses the two-connection retry pattern from `workspace_subscription`:
/// the subscribe call gets its own connection; failures back off 3 s
/// before reconnecting.
pub fn mode_subscription() -> Subscription<Message> {
    Subscription::run_with_id(
        "mde-portal-mode",
        async_stream::stream! {
            loop {
                let event_conn = match Connection::new().await {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::debug!(error = %e, "swayipc mode event connect failed; retrying in 3s");
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        continue;
                    }
                };
                let mut events = match event_conn.subscribe([EventType::Mode]).await {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::debug!(error = %e, "mode subscribe failed; retrying in 3s");
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        continue;
                    }
                };
                while let Some(event_result) = events.next().await {
                    if let Ok(swayipc_async::Event::Mode(mode_ev)) = event_result {
                        let next = mode_change_to_message(&mode_ev.change);
                        yield Message::ModeChanged(next);
                    }
                }
                tracing::debug!("swayipc mode event stream ended; reconnecting");
            }
        },
    )
}

/// Portal-45 (R12-Q5) — pure mapping from sway's mode-change string
/// to the `Option<String>` the Dock state expects. `"default"` →
/// `None` (no mode); anything else → `Some(name)`. Exposed so the
/// transition contract is independently testable.
pub fn mode_change_to_message(change: &str) -> Option<String> {
    if change == "default" {
        None
    } else {
        Some(change.to_string())
    }
}

/// Focus a window by container ID via a fresh swayipc connection (Portal-8.a).
pub async fn focus_window_by_id(con_id: i64) {
    match Connection::new().await {
        Ok(mut conn) => {
            if let Err(e) = conn.run_command(&format!("[con_id={con_id}] focus")).await {
                tracing::warn!(con_id, error = %e, "focus_window_by_id command failed");
            }
        }
        Err(e) => tracing::warn!(error = %e, "focus_window_by_id: swayipc connect failed"),
    }
}

/// Focus a workspace by name via a fresh swayipc connection.
///
/// The subscription's event loop will deliver an updated `WorkspaceList`
/// automatically once sway emits the workspace-change event.
pub async fn focus_workspace(name: String) {
    match Connection::new().await {
        Ok(mut conn) => {
            if let Err(e) = conn.run_command(&format!("workspace \"{}\"", name)).await {
                tracing::warn!(workspace = %name, error = %e, "focus_workspace command failed");
            }
        }
        Err(e) => tracing::warn!(error = %e, "focus_workspace: swayipc connect failed"),
    }
}

/// Portal-43 (R12-Q3): focus a workspace by its i3/sway numeric ID,
/// not by name. Used by the previous-workspace breadcrumb segment so
/// the click survives a Portal-41 rename in flight (the auto-derived
/// name might have changed by the time the operator clicks).
pub async fn focus_workspace_by_num(num: i32) {
    match Connection::new().await {
        Ok(mut conn) => {
            if let Err(e) = conn.run_command(&format!("workspace number {num}")).await {
                tracing::warn!(workspace_num = num, error = %e, "focus_workspace_by_num command failed");
            }
        }
        Err(e) => tracing::warn!(error = %e, "focus_workspace_by_num: swayipc connect failed"),
    }
}

/// Recursively collect the container IDs of all tiling-window leaf nodes.
///
/// A tiling window is a `Con` node with no child `nodes` (i.e., a leaf
/// that isn't a split container, workspace, or output).
fn collect_tiling_ids(node: &swayipc_async::Node) -> Vec<i64> {
    let mut ids = Vec::new();
    if node.node_type == NodeType::Con && node.nodes.is_empty() {
        ids.push(node.id);
    }
    for child in &node.nodes {
        ids.extend(collect_tiling_ids(child));
    }
    ids
}

/// Move all tiling windows to the scratchpad (Portal-12 show-wallpaper on).
///
/// Returns the container IDs of every window moved, so `show_desktop_restore`
/// can bring exactly those windows back without disturbing pre-existing
/// scratchpad items.
pub async fn show_desktop_hide() -> Vec<i64> {
    let conn = match Connection::new().await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "show_desktop: swayipc connect failed");
            return Vec::new();
        }
    };
    let mut conn = conn;

    let tree = match conn.get_tree().await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(error = %e, "show_desktop: get_tree failed");
            return Vec::new();
        }
    };

    let ids = collect_tiling_ids(&tree);
    for id in &ids {
        if let Err(e) = conn.run_command(&format!("[con_id={id}] move to scratchpad")).await {
            tracing::warn!(error = %e, con_id = id, "show_desktop: move to scratchpad failed");
        }
    }
    ids
}

/// Restore tiling windows from the scratchpad by container ID (Portal-12).
///
/// Only windows whose IDs were returned by `show_desktop_hide()` are
/// restored, leaving any pre-existing scratchpad items untouched.
pub async fn show_desktop_restore(ids: Vec<i64>) {
    let conn = match Connection::new().await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "show_desktop restore: swayipc connect failed");
            return;
        }
    };
    let mut conn = conn;
    for id in &ids {
        if let Err(e) = conn.run_command(&format!("[con_id={id}] scratchpad show")).await {
            tracing::warn!(error = %e, con_id = id, "show_desktop: scratchpad show failed");
        }
    }
}

// ── WM micro-actions (Portal-8.b) ─────────────────────────────────────────────

/// Kill the window with the given container ID (Portal-8.b close button).
pub async fn wm_close(con_id: i64) {
    if let Ok(mut conn) = Connection::new().await {
        let _ = conn.run_command(&format!("[con_id={con_id}] kill")).await;
    }
}

/// Toggle floating state for the given container (Portal-8.b float button).
pub async fn wm_float_toggle(con_id: i64) {
    if let Ok(mut conn) = Connection::new().await {
        let _ = conn.run_command(&format!("[con_id={con_id}] floating toggle")).await;
    }
}

/// Toggle fullscreen for the given container (Portal-8.b fullscreen button).
pub async fn wm_fullscreen_toggle(con_id: i64) {
    if let Ok(mut conn) = Connection::new().await {
        let _ = conn.run_command(&format!("[con_id={con_id}] fullscreen toggle")).await;
    }
}

/// Park the given container at workspace 99 (Portal-8.b 5th micro-button,
/// Portal-59 R12-Q24 supersedes the scratchpad model). Workspace 99 is the
/// platform's reserved "parked-window" slot — Portal mini-tree + running-zone
/// filter it out (Portal-5 + Portal-8.a) so a parked window feels minimized
/// to the operator while staying first-class in sway's tree (no scratchpad-
/// stack semantics to manage). The sequence is three swayipc commands:
///   1. `move container to workspace number 99` — relocates the window.
///   2. `workspace number 99` — briefly switches there so sway records 99
///      as the "previous" workspace.
///   3. `workspace back_and_forth` — returns to where the operator was.
/// The natural inverse `bindsym $mod+Shift+m` (data/sway/config) un-parks
/// the most-recently-focused parked window into the current workspace.
pub async fn wm_minimize(con_id: i64) {
    let Ok(mut conn) = Connection::new().await else {
        return;
    };
    let _ = conn
        .run_command(&format!("[con_id={con_id}] move container to workspace number 99"))
        .await;
    let _ = conn.run_command("workspace number 99").await;
    let _ = conn.run_command("workspace back_and_forth").await;
}

/// Cycle the parent layout: split → tabbed → stacking (Portal-8.b layout button).
pub async fn wm_layout_cycle(con_id: i64) {
    if let Ok(mut conn) = Connection::new().await {
        let _ = conn
            .run_command(&format!(
                "[con_id={con_id}] layout toggle split tabbed stacking"
            ))
            .await;
    }
}

/// Show or hide the Portal-full scratchpad surface via sway IPC.
///
/// `scratchpad show` with an app_id criterion toggles: if the window is in
/// the scratchpad (hidden) it appears on the focused output; if it is
/// already visible sway moves it back to the scratchpad.
pub async fn portal_full_scratchpad_toggle() {
    match Connection::new().await {
        Ok(mut conn) => {
            if let Err(e) = conn
                .run_command(
                    r#"[app_id="dev.mackes.MDE.Portal.Full"] scratchpad show"#,
                )
                .await
            {
                tracing::warn!(error = %e, "portal_full_scratchpad_toggle: command failed");
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "portal_full_scratchpad_toggle: swayipc connect failed");
        }
    }
}

/// Switch to the lowest unused workspace number ≥ 1.
pub async fn new_workspace(taken_nums: Vec<i32>) {
    let next = (1i32..).find(|n| !taken_nums.contains(n)).unwrap_or(1);
    match Connection::new().await {
        Ok(mut conn) => {
            if let Err(e) = conn.run_command(&format!("workspace {next}")).await {
                tracing::warn!(error = %e, "new_workspace command failed");
            }
        }
        Err(e) => tracing::warn!(error = %e, "new_workspace: swayipc connect failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ws(num: i32, name: &str, focused: bool, visible: bool, output: &str, urgent: bool) -> WorkspaceInfo {
        WorkspaceInfo {
            num,
            name: name.to_string(),
            focused,
            visible,
            output: output.to_string(),
            urgent,
        }
    }

    #[test]
    fn workspace_cell_min_px_is_24() {
        assert!((WORKSPACE_CELL_MIN_PX - 24.0).abs() < f32::EPSILON);
    }

    #[test]
    fn display_label_numeric_workspace() {
        let ws = make_ws(1, "1", false, false, "HDMI-A-1", false);
        assert_eq!(ws.display_label(), "1");
    }

    #[test]
    fn display_label_short_named_workspace() {
        let ws = make_ws(2, "dev", false, false, "HDMI-A-1", false);
        assert_eq!(ws.display_label(), "dev");
    }

    #[test]
    fn display_label_long_named_workspace_truncates() {
        let ws = make_ws(3, "my-very-long-project-name", false, false, "HDMI-A-1", false);
        let label = ws.display_label();
        assert!(label.ends_with('…'), "long name should end with ellipsis: {label}");
        assert!(label.chars().count() <= WS_NAME_MAX_CHARS + 1, "truncated label too long: {label}");
    }

    #[test]
    fn display_label_exactly_max_chars_no_truncation() {
        let name = "abcdefgh"; // exactly WS_NAME_MAX_CHARS = 8
        let ws = make_ws(4, name, false, false, "HDMI-A-1", false);
        assert_eq!(ws.display_label(), name);
    }

    #[test]
    fn workspace_info_fields_preserved() {
        let ws = make_ws(5, "test", true, true, "eDP-1", true);
        assert_eq!(ws.num, 5);
        assert!(ws.focused);
        assert!(ws.visible);
        assert!(ws.urgent);
        assert_eq!(ws.output, "eDP-1");
    }

    #[test]
    fn new_workspace_finds_lowest_gap() {
        let taken = vec![1, 2];
        let next = (1i32..).find(|n| !taken.contains(n)).unwrap_or(1);
        assert_eq!(next, 3);
    }

    #[test]
    fn new_workspace_finds_gap_in_middle() {
        let taken = vec![1, 3, 5];
        let next = (1i32..).find(|n| !taken.contains(n)).unwrap_or(1);
        assert_eq!(next, 2);
    }

    #[test]
    fn new_workspace_empty_taken_starts_at_1() {
        let taken: Vec<i32> = vec![];
        let next = (1i32..).find(|n| !taken.contains(n)).unwrap_or(1);
        assert_eq!(next, 1);
    }

    // ── Portal-8.a WindowInfo tests ───────────────────────────────────────────

    fn make_window_info(app_id: &str, focused: bool) -> WindowInfo {
        WindowInfo {
            con_id: 99,
            app_id: Some(app_id.to_string()),
            title: Some(format!("{app_id} window")),
            workspace_num: 1,
            focused,
            marks: Vec::new(),
        }
    }

    #[test]
    fn window_display_label_short_app_id() {
        let w = make_window_info("foot", false);
        assert_eq!(w.display_label(), "foot");
    }

    #[test]
    fn window_display_label_truncates_long_app_id() {
        let w = make_window_info("com.example.very-long-app-id", false);
        let label = w.display_label();
        assert!(label.ends_with('…'), "long app_id should be truncated: {label}");
        assert!(label.chars().count() <= 11, "truncated label too long: {label}");
    }

    #[test]
    fn window_display_label_falls_back_to_title() {
        let w = WindowInfo {
            con_id: 1,
            app_id: None,
            title: Some("Doc".to_string()),
            workspace_num: 1,
            focused: false,
            marks: Vec::new(),
        };
        assert_eq!(w.display_label(), "Doc", "should use title when no app_id");
    }

    #[test]
    fn window_display_label_falls_back_to_question_mark() {
        let w = WindowInfo {
            con_id: 1,
            app_id: None,
            title: None,
            workspace_num: 1,
            focused: false,
            marks: Vec::new(),
        };
        assert_eq!(w.display_label(), "?");
    }

    #[test]
    fn collect_windows_skips_non_con_nodes() {
        // A Workspace node with a child leaf Con that has an app_id.
        let json = workspace_json(10, 99);
        // workspace_json uses con_leaf_json which doesn't set app_id.
        // The collect_windows function only collects when app_id.is_some(),
        // so this workspace should yield an empty list (leaf has no app_id).
        let node: swayipc_async::Node = serde_json::from_str(&json).unwrap();
        let windows = collect_windows(&node, -1);
        assert!(windows.is_empty(), "leaf without app_id should not be collected");
    }

    // ── Portal-12 show-desktop tree-walk tests ────────────────────────────────
    //
    // `swayipc_async::Node` is #[non_exhaustive] so we can't construct it via
    // struct literals from external crates.  We deserialize from minimal JSON
    // (swayipc types derive Deserialize) to build test nodes instead.

    /// Minimal JSON for a leaf `Con` node with no children.
    fn con_leaf_json(id: i64) -> String {
        format!(
            r#"{{
                "id": {id},
                "name": "win-{id}",
                "type": "con",
                "border": "none",
                "current_border_width": 0,
                "layout": "splith",
                "orientation": "none",
                "percent": null,
                "rect": {{"x":0,"y":0,"width":100,"height":100}},
                "window_rect": {{"x":0,"y":0,"width":100,"height":100}},
                "deco_rect": {{"x":0,"y":0,"width":0,"height":0}},
                "geometry": {{"x":0,"y":0,"width":100,"height":100}},
                "urgent": false,
                "focused": false,
                "focus": [],
                "floating": null,
                "floating_nodes": [],
                "sticky": false
            }}"#
        )
    }

    /// Minimal JSON for a `Con` with one child (a leaf Con with given ID).
    fn con_split_json(parent_id: i64, child_id: i64) -> String {
        let child = con_leaf_json(child_id);
        format!(
            r#"{{
                "id": {parent_id},
                "name": "split-{parent_id}",
                "type": "con",
                "border": "none",
                "current_border_width": 0,
                "layout": "splith",
                "orientation": "none",
                "percent": null,
                "rect": {{"x":0,"y":0,"width":200,"height":100}},
                "window_rect": {{"x":0,"y":0,"width":200,"height":100}},
                "deco_rect": {{"x":0,"y":0,"width":0,"height":0}},
                "geometry": {{"x":0,"y":0,"width":200,"height":100}},
                "urgent": false,
                "focused": false,
                "focus": [],
                "floating": null,
                "nodes": [{child}],
                "floating_nodes": [],
                "sticky": false
            }}"#
        )
    }

    /// Minimal JSON for a `Workspace` node with a child leaf Con.
    fn workspace_json(ws_id: i64, child_id: i64) -> String {
        let child = con_leaf_json(child_id);
        format!(
            r#"{{
                "id": {ws_id},
                "name": "1",
                "type": "workspace",
                "border": "none",
                "current_border_width": 0,
                "layout": "splith",
                "orientation": "none",
                "percent": null,
                "rect": {{"x":0,"y":0,"width":1920,"height":1080}},
                "window_rect": {{"x":0,"y":0,"width":1920,"height":1080}},
                "deco_rect": {{"x":0,"y":0,"width":0,"height":0}},
                "geometry": {{"x":0,"y":0,"width":1920,"height":1080}},
                "urgent": false,
                "focused": true,
                "focus": [],
                "floating": null,
                "nodes": [{child}],
                "floating_nodes": [],
                "sticky": false,
                "num": 1,
                "representation": "H[xterm]"
            }}"#
        )
    }

    fn parse_node(json: &str) -> swayipc_async::Node {
        serde_json::from_str(json).expect("test node JSON should parse")
    }

    // ── Portal-8.b WM action function existence tests ─────────────────────────
    //
    // Sway is not running in test; these verify the functions are callable
    // without panicking (they fail gracefully when the socket is absent).

    #[tokio::test]
    async fn wm_close_does_not_panic_without_sway() {
        wm_close(999_999).await; // graceful no-op when SWAYSOCK absent
    }

    #[tokio::test]
    async fn wm_float_toggle_does_not_panic_without_sway() {
        wm_float_toggle(999_999).await;
    }

    #[tokio::test]
    async fn wm_fullscreen_toggle_does_not_panic_without_sway() {
        wm_fullscreen_toggle(999_999).await;
    }

    #[tokio::test]
    async fn wm_minimize_does_not_panic_without_sway() {
        wm_minimize(999_999).await;
    }

    #[tokio::test]
    async fn wm_layout_cycle_does_not_panic_without_sway() {
        wm_layout_cycle(999_999).await;
    }

    #[tokio::test]
    async fn portal_full_scratchpad_toggle_does_not_panic_without_sway() {
        portal_full_scratchpad_toggle().await;
    }

    #[test]
    fn collect_tiling_ids_leaf_con_returned() {
        let leaf = parse_node(&con_leaf_json(42));
        let ids = collect_tiling_ids(&leaf);
        assert_eq!(ids, vec![42]);
    }

    #[test]
    fn collect_tiling_ids_workspace_not_returned() {
        let ws = parse_node(&workspace_json(10, 99));
        // The workspace itself should NOT be in the list, only the leaf Con child
        let ids = collect_tiling_ids(&ws);
        assert!(!ids.contains(&10), "workspace node should not be collected");
        assert!(ids.contains(&99), "leaf Con inside workspace should be collected");
    }

    #[test]
    fn collect_tiling_ids_non_leaf_con_not_returned() {
        let split = parse_node(&con_split_json(10, 99));
        let ids = collect_tiling_ids(&split);
        assert!(!ids.contains(&10), "non-leaf Con should not be in result");
        assert!(ids.contains(&99), "leaf child should be collected");
    }

    #[test]
    fn collect_tiling_ids_leaf_con_empty_nodes() {
        // A leaf Con with no children should always be collected.
        let leaf = parse_node(&con_leaf_json(7));
        let ids = collect_tiling_ids(&leaf);
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], 7);
    }
}
