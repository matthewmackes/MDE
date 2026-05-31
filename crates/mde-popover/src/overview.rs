//! `mde-popover overview` — workspace overview surface (ANIM-6.b, Q46).
//!
//! Fullscreen Layer::Overlay grid of workspace cards. Each card shows
//! the workspace name, the first 5 running app_ids as small pills, and
//! a focused/active highlight. Click a card to switch to that workspace
//! and dismiss. Esc dismisses without switching. No wlr-screencopy —
//! this is a card-list overview, not a thumbnail grid.
//!
//! Bound via `bindsym $mod+Tab exec mde-popover overview` (or any sway
//! binding the operator prefers). Spawned by mde-portal on the Super+Tab
//! gesture once Portal-34 ships; for now, plain sway keybind.

use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Shadow, Subscription, Task, Theme};
use iced::widget::container::Style as ContainerStyle;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

// ── Design tokens ─────────────────────────────────────────────────────────────

const SURFACE_BG: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.82 };
const CARD_BG: Color = Color { r: 0.125, g: 0.129, b: 0.141, a: 1.0 };
const CARD_FOCUSED_BG: Color = Color { r: 0.11, g: 0.13, b: 0.28, a: 1.0 };
const CARD_BORDER: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.10 };
const CARD_FOCUSED_BORDER: Color = Color { r: 0.357, g: 0.416, b: 0.961, a: 0.85 };
const FG: Color = Color { r: 0.957, g: 0.957, b: 0.957, a: 1.0 };
const FG_DIM: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.55 };
const FG_LABEL: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.35 };
const ACCENT: Color = Color { r: 0.357, g: 0.416, b: 0.961, a: 1.0 };
const PILL_BG: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.08 };

/// Maximum app pills shown per workspace card.
const MAX_PILLS: usize = 5;
/// Maximum workspace cards per row.
const MAX_COLS: usize = 5;
/// Card fixed width in px.
const CARD_W: f32 = 180.0;

// ── Data model ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WorkspaceCard {
    /// Sway workspace number (1-based).
    pub num: i64,
    /// Workspace name (may be the number as string, or a custom name).
    pub name: String,
    /// Whether this is the currently focused workspace.
    pub focused: bool,
    /// app_ids of the first `MAX_PILLS` open windows.
    pub apps: Vec<String>,
}

// ── Sway data loading ─────────────────────────────────────────────────────────

/// Run `swaymsg -t get_workspaces` and `swaymsg -t get_tree`, combine
/// into a list of WorkspaceCard. Silently returns [] on any failure so
/// the overlay still mounts (shows "No workspaces" empty state).
fn load_workspace_cards() -> Vec<WorkspaceCard> {
    let workspaces = load_workspaces_json();
    let tree = load_tree_json();

    let mut cards: Vec<WorkspaceCard> = workspaces
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|ws| {
                    let num = ws.get("num").and_then(|v| v.as_i64()).unwrap_or(0);
                    let name = ws
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if num == 0 || name.is_empty() {
                        return None;
                    }
                    let focused = ws
                        .get("focused")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    Some(WorkspaceCard { num, name, focused, apps: Vec::new() })
                })
                .collect()
        })
        .unwrap_or_default();

    // Populate apps from the tree. Walk the tree and at each
    // workspace node collect descendant app_ids.
    if let Some(tree_val) = tree {
        for card in &mut cards {
            collect_apps_for_workspace(&tree_val, &card.name, &mut card.apps);
        }
    }

    cards.sort_by_key(|c| c.num);
    cards
}

fn load_workspaces_json() -> serde_json::Value {
    use std::process::Command;
    let out = Command::new("swaymsg")
        .args(["-t", "get_workspaces"])
        .stderr(std::process::Stdio::null())
        .output();
    match out {
        Ok(o) if o.status.success() => {
            serde_json::from_slice(&o.stdout).unwrap_or(serde_json::Value::Array(vec![]))
        }
        _ => serde_json::Value::Array(vec![]),
    }
}

fn load_tree_json() -> Option<serde_json::Value> {
    use std::process::Command;
    let out = Command::new("swaymsg")
        .args(["-t", "get_tree"])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    serde_json::from_slice(&out.stdout).ok()
}

/// Walk the sway tree looking for workspace nodes whose name matches
/// `ws_name`; collect up to MAX_PILLS distinct app_ids from their leaves.
fn collect_apps_for_workspace(
    node: &serde_json::Value,
    ws_name: &str,
    apps: &mut Vec<String>,
) {
    let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if node_type == "workspace" {
        let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if name == ws_name {
            collect_leaf_apps(node, apps);
            return;
        }
    }
    for key in ["nodes", "floating_nodes"] {
        if let Some(children) = node.get(key).and_then(|v| v.as_array()) {
            for child in children {
                if apps.len() >= MAX_PILLS {
                    return;
                }
                collect_apps_for_workspace(child, ws_name, apps);
            }
        }
    }
}

/// Depth-first collect app_ids from leaf nodes (windows) up to MAX_PILLS.
fn collect_leaf_apps(node: &serde_json::Value, apps: &mut Vec<String>) {
    if apps.len() >= MAX_PILLS {
        return;
    }
    // A leaf has a pid (real window).
    if node.get("pid").is_some_and(|v| !v.is_null()) {
        let app_id = node
            .get("app_id")
            .and_then(|v| v.as_str())
            .or_else(|| {
                node.get("window_properties")
                    .and_then(|w| w.get("class"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("")
            .to_string();
        if !app_id.is_empty() && !apps.iter().any(|a| a == &app_id) {
            apps.push(app_id);
        }
    }
    for key in ["nodes", "floating_nodes"] {
        if let Some(children) = node.get(key).and_then(|v| v.as_array()) {
            for child in children {
                if apps.len() >= MAX_PILLS {
                    return;
                }
                collect_leaf_apps(child, apps);
            }
        }
    }
}

// ── Sway switch helper ────────────────────────────────────────────────────────

fn swaymsg_switch(num: i64) {
    let _ = std::process::Command::new("swaymsg")
        .arg(format!("workspace number {num}"))
        .stderr(std::process::Stdio::null())
        .status();
}

// ── Iced app ──────────────────────────────────────────────────────────────────

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    /// Switch to workspace then exit.
    SwitchTo(i64),
    Exit,
}

pub struct App {
    workspaces: Vec<WorkspaceCard>,
}

fn namespace() -> String {
    "mde-popover-overview".to_string()
}

fn update(_state: &mut App, msg: Message) -> Task<Message> {
    match msg {
        Message::SwitchTo(num) => {
            swaymsg_switch(num);
            std::process::exit(0);
        }
        Message::Exit => std::process::exit(0),
        _ => Task::none(),
    }
}

fn view(state: &App) -> Element<'_, Message> {
    if state.workspaces.is_empty() {
        return container(text("No workspaces").size(16).color(FG_DIM))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_: &Theme| ContainerStyle {
                background: Some(Background::Color(SURFACE_BG)),
                ..Default::default()
            })
            .into();
    }

    let cols = (state.workspaces.len().min(MAX_COLS)).max(1);
    let mut grid = column![].spacing(16);
    let mut row_buf: Vec<Element<'_, Message>> = Vec::new();

    for (i, ws) in state.workspaces.iter().enumerate() {
        row_buf.push(workspace_card_view(ws));
        if (i + 1) % cols == 0 {
            let mut r = row![].spacing(16).align_y(Alignment::Start);
            for el in row_buf.drain(..) {
                r = r.push(el);
            }
            grid = grid.push(r);
        }
    }
    if !row_buf.is_empty() {
        let mut r = row![].spacing(16).align_y(Alignment::Start);
        for el in row_buf.drain(..) {
            r = r.push(el);
        }
        grid = grid.push(r);
    }

    let footer = text("Esc closes · click a card to switch workspace")
        .size(11)
        .color(FG_LABEL);

    let body = column![
        Space::new().height(Length::Fixed(48.0)),
        container(
            text("Workspaces").size(14).color(FG_DIM),
        )
        .center_x(Length::Fill),
        Space::new().height(Length::Fixed(20.0)),
        container(grid)
            .center_x(Length::Fill),
        Space::new().height(Length::Fixed(24.0)),
        container(footer).center_x(Length::Fill),
        Space::new().height(Length::Fixed(48.0)),
    ]
    .padding(Padding {
        top: 0.0,
        right: 40.0,
        bottom: 0.0,
        left: 40.0,
    });

    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_: &Theme| ContainerStyle {
            background: Some(Background::Color(SURFACE_BG)),
            ..Default::default()
        })
        .into()
}

fn subscription(_state: &App) -> Subscription<Message> {
    use iced::event;
    event::listen_with(|event, status, _window| {
        use iced::keyboard;
        match event {
            iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. })
                if status == event::Status::Ignored =>
            {
                use iced::keyboard::{key::Named, Key};
                if matches!(key, Key::Named(Named::Escape)) {
                    Some(Message::Exit)
                } else {
                    None
                }
            }
            _ => None,
        }
    })
}

/// Build a single workspace card element.
fn workspace_card_view(ws: &WorkspaceCard) -> Element<'_, Message> {
    let name_row = container(
        text(ws.name.clone())
            .size(13)
            .color(if ws.focused { ACCENT } else { FG }),
    )
    .padding(Padding {
        top: 0.0,
        right: 0.0,
        bottom: 8.0,
        left: 0.0,
    });

    let pills_col: Element<'_, Message> = if ws.apps.is_empty() {
        text("Empty").size(11).color(FG_LABEL).into()
    } else {
        let mut col = column![].spacing(4);
        for app in &ws.apps {
            let chip = container(
                text(shorten_app_id(app)).size(11).color(FG_DIM),
            )
            .padding(Padding::from([2u16, 6u16]))
            .style(|_: &Theme| ContainerStyle {
                background: Some(Background::Color(PILL_BG)),
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });
            col = col.push(chip);
        }
        if ws.apps.len() == MAX_PILLS {
            col = col.push(text("…").size(11).color(FG_LABEL));
        }
        col.into()
    };

    let inner = column![name_row, pills_col]
        .padding(Padding::from([10u16, 12u16]));

    let bg = if ws.focused { CARD_FOCUSED_BG } else { CARD_BG };
    let border_color = if ws.focused { CARD_FOCUSED_BORDER } else { CARD_BORDER };
    let border_width = if ws.focused { 1.5_f32 } else { 1.0_f32 };

    button(inner)
        .width(Length::Fixed(CARD_W))
        .on_press(Message::SwitchTo(ws.num))
        .style(move |_t: &Theme, _status: iced::widget::button::Status| {
            iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                border: Border {
                    color: border_color,
                    width: border_width,
                    radius: 8.0.into(),
                },
                text_color: FG,
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .into()
}

/// Trim common prefixes to keep pills compact.
/// `org.gnome.Nautilus` → `Nautilus`, `com.github.foo` → `foo`
#[must_use]
pub fn shorten_app_id(app_id: &str) -> String {
    app_id
        .rsplit('.')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(app_id)
        .to_string()
}

pub fn run() -> iced_layershell::Result {
    iced_layershell::application(
        || {
            let workspaces = load_workspace_cards();
            tracing::info!(count = workspaces.len(), "overview popover loaded");
            App { workspaces }
        },
        namespace,
        update,
        view,
    )
    .theme(|_: &App| iced::Theme::Dark)
    .subscription(subscription)
    .settings(Settings {
        id: Some("mde-popover-overview".to_string()),
        fonts: crate::fonts::load_fallback_fonts(),
        layer_settings: LayerShellSettings {
            layer: Layer::Overlay,
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            exclusive_zone: -1,
            margin: (0, 0, 0, 0),
            size: None,
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            ..Default::default()
        },
        ..Default::default()
    })
    .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── collect_leaf_apps ──────────────────────────────────────────────────────

    fn make_window(pid: u64, app_id: &str) -> serde_json::Value {
        serde_json::json!({
            "pid": pid,
            "app_id": app_id,
            "nodes": [],
            "floating_nodes": []
        })
    }

    fn make_ws(name: &str, windows: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::json!({
            "type": "workspace",
            "name": name,
            "nodes": windows,
            "floating_nodes": []
        })
    }

    #[test]
    fn collect_leaf_apps_basic() {
        let tree = make_ws("1", vec![make_window(100, "firefox"), make_window(101, "foot")]);
        let mut apps = Vec::new();
        collect_leaf_apps(&tree, &mut apps);
        assert_eq!(apps, vec!["firefox", "foot"]);
    }

    #[test]
    fn collect_leaf_apps_deduplicates() {
        let tree = make_ws("1", vec![make_window(100, "foot"), make_window(101, "foot")]);
        let mut apps = Vec::new();
        collect_leaf_apps(&tree, &mut apps);
        assert_eq!(apps, vec!["foot"]);
    }

    #[test]
    fn collect_leaf_apps_respects_max_pills() {
        let windows: Vec<_> = (0..8u64)
            .map(|i| make_window(i + 100, &format!("app{i}")))
            .collect();
        let tree = make_ws("1", windows);
        let mut apps = Vec::new();
        collect_leaf_apps(&tree, &mut apps);
        assert_eq!(apps.len(), MAX_PILLS);
    }

    #[test]
    fn collect_leaf_apps_skips_empty_app_id() {
        let tree = make_ws("1", vec![
            serde_json::json!({ "pid": 100, "app_id": "", "nodes": [], "floating_nodes": [] }),
            make_window(101, "firefox"),
        ]);
        let mut apps = Vec::new();
        collect_leaf_apps(&tree, &mut apps);
        assert_eq!(apps, vec!["firefox"]);
    }

    // ── collect_apps_for_workspace ─────────────────────────────────────────────

    #[test]
    fn collect_apps_for_workspace_targets_correct_workspace() {
        let output_node = serde_json::json!({
            "type": "output",
            "nodes": [
                make_ws("1", vec![make_window(10, "firefox")]),
                make_ws("2", vec![make_window(20, "foot")]),
            ],
            "floating_nodes": []
        });
        let mut apps = Vec::new();
        collect_apps_for_workspace(&output_node, "2", &mut apps);
        assert_eq!(apps, vec!["foot"]);
    }

    #[test]
    fn collect_apps_for_workspace_unknown_name_returns_empty() {
        let output_node = serde_json::json!({
            "type": "output",
            "nodes": [make_ws("1", vec![make_window(10, "firefox")])],
            "floating_nodes": []
        });
        let mut apps = Vec::new();
        collect_apps_for_workspace(&output_node, "99", &mut apps);
        assert!(apps.is_empty());
    }

    // ── shorten_app_id ─────────────────────────────────────────────────────────

    #[test]
    fn shorten_app_id_trims_reversed_domain() {
        assert_eq!(shorten_app_id("org.gnome.Nautilus"), "Nautilus");
        assert_eq!(shorten_app_id("com.github.alacritty"), "alacritty");
    }

    #[test]
    fn shorten_app_id_plain_name_unchanged() {
        assert_eq!(shorten_app_id("firefox"), "firefox");
        assert_eq!(shorten_app_id("foot"), "foot");
    }

    #[test]
    fn shorten_app_id_empty_returns_empty() {
        assert_eq!(shorten_app_id(""), "");
    }

    // ── design lock constants ──────────────────────────────────────────────────

    #[test]
    fn max_pills_is_five() {
        assert_eq!(MAX_PILLS, 5);
    }

    #[test]
    fn max_cols_is_five() {
        assert_eq!(MAX_COLS, 5);
    }

    #[test]
    fn card_width_is_180() {
        assert!((CARD_W - 180.0).abs() < f32::EPSILON);
    }

    // ── load_workspace_cards integration shape ─────────────────────────────────

    #[test]
    fn workspace_card_fields_populated() {
        let card = WorkspaceCard {
            num: 1,
            name: "1".to_string(),
            focused: true,
            apps: vec!["firefox".to_string(), "foot".to_string()],
        };
        assert_eq!(card.num, 1);
        assert!(card.focused);
        assert_eq!(card.apps.len(), 2);
    }
}
