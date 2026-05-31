//! Phase E.4.4 — exposé grid (F3).
//!
//! v3.0.3 — moved from mde-panel/src/expose.rs (where the
//! Phase E.4.4 [✓] entry shipped pure-fn layout math as dead code
//! — audit 2026-05-22). The popover now mounts a real layer-shell
//! overlay surface anchored bottom-left at full output width:
//! reads `swaymsg -t get_tree`, flattens to "normal" window leaves,
//! renders a grid of cards (title + app_id, click = focus + close).
//!
//! Bound to F3 in `data/sway/config` (`bindsym F3 exec
//! mde-popover expose`). The popover process exits when the user
//! picks a card or presses Esc.

/// Maximum columns per row (caps at 6 even when there are
/// hundreds of windows).
pub const MAX_COLUMNS: usize = 6;

/// Compute grid column count for N windows. Capped at
/// [`MAX_COLUMNS`].
#[must_use]
pub fn grid_columns(window_count: usize) -> usize {
    if window_count == 0 {
        return 1;
    }
    let sqrt = (window_count as f64).sqrt().ceil() as usize;
    sqrt.min(MAX_COLUMNS).max(1)
}

/// Truncate a window title with `…` once it exceeds `max` chars.
/// Multi-byte safe.
#[must_use]
pub fn truncate_title(title: &str, max: usize) -> String {
    let count = title.chars().count();
    if count <= max {
        return title.to_string();
    }
    let mut out: String = title.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// One exposé card.
#[derive(Debug, Clone)]
pub struct ExposeCard {
    pub con_id: u64,
    pub title: String,
    pub app_id: String,
}

// v3.0.3 — the original `cards_from_windows(SwayWindow)` helper +
// the `SwayWindow` struct were used only by tests against a mocked
// window list. The runtime now goes directly from sway-IPC JSON
// to ExposeCard via `walk_tree_for_cards` below, so the test-only
// helpers were dead per §0.12 and got removed. The behavior they
// asserted (filter to normal windows, normalize titles) is now
// covered indirectly via `walk_tree_for_cards` reading real
// get_tree output.

// ──────────────────────────────────────────────────────────────
// v3.0.3 — Iced layer-shell overlay
// ──────────────────────────────────────────────────────────────

use iced::widget::{button, column, container, row, text, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Subscription, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

const FG_TEXT: Color = Color {
    r: 0.957,
    g: 0.957,
    b: 0.957,
    a: 1.0,
};
const FG_MUTED: Color = Color {
    r: 0.659,
    g: 0.659,
    b: 0.659,
    a: 1.0,
};
const ACCENT: Color = Color {
    r: 0.169,
    g: 0.604,
    b: 0.953,
    a: 1.0,
};
const SURFACE_BG: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.78,
};

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    /// User clicked a card — focus the window then exit.
    FocusWindow(u64),
    Exit,
}

pub struct App {
    cards: Vec<ExposeCard>,
}

fn namespace() -> String {
    "mde-popover-expose".into()
}

fn update(_state: &mut App, msg: Message) -> Task<Message> {
    match msg {
        Message::FocusWindow(con_id) => {
            swaymsg_focus(con_id);
            std::process::exit(0);
        }
        Message::Exit => std::process::exit(0),
        _ => Task::none(),
    }
}

fn view(state: &App) -> Element<'_, Message> {
    if state.cards.is_empty() {
        return container(text("No windows").size(16).color(FG_MUTED))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(surface_style)
            .into();
    }
    let cols = grid_columns(state.cards.len());
    let mut grid = column![].spacing(12);
    let mut row_buf: Vec<Element<'_, Message>> = Vec::new();
    for (i, card) in state.cards.iter().enumerate() {
        row_buf.push(card_view(card));
        if (i + 1) % cols == 0 {
            let mut r = row![].spacing(12);
            for el in row_buf.drain(..) {
                r = r.push(el);
            }
            grid = grid.push(r);
        }
    }
    if !row_buf.is_empty() {
        let mut r = row![].spacing(12);
        for el in row_buf.drain(..) {
            r = r.push(el);
        }
        grid = grid.push(r);
    }

    let footer = text("F3 / Esc closes · click a card to focus")
        .size(11)
        .color(FG_MUTED);

    let body = column![
        Space::new().height(Length::Fixed(40.0)),
        grid,
        Space::new().height(Length::Fixed(20.0)),
        container(footer).center_x(Length::Fill),
        Space::new().height(Length::Fixed(40.0)),
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
        .style(surface_style)
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
                if matches!(key, Key::Named(Named::Escape) | Key::Named(Named::F3)) {
                    Some(Message::Exit)
                } else {
                    None
                }
            }
            _ => None,
        }
    })
}

pub fn run() -> iced_layershell::Result {
    iced_layershell::application(
        || {
            let cards = load_cards_via_swaymsg();
            tracing::info!(count = cards.len(), "expose popover loaded");
            App { cards }
        },
        namespace,
        update,
        view,
    )
    .theme(|_: &App| iced::Theme::Dark)
    .subscription(subscription)
    .settings(Settings {
        id: Some("mde-popover-expose".into()),
        fonts: crate::fonts::load_fallback_fonts(),
        layer_settings: LayerShellSettings {
            // Overlay layer + fullscreen anchor so the grid covers
            // the entire output. exclusive_zone=-1 makes the
            // surface ignore the panel's reserved zone too.
            layer: Layer::Overlay,
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            exclusive_zone: -1,
            margin: (0, 0, 0, 0),
            size: None,
            // Exclusive keyboard so Esc + F3 reliably reach this
            // surface (we're modal-ish — user picks a window or
            // dismisses, then we're gone).
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            ..Default::default()
        },
        ..Default::default()
    })
    .run()
}

/// Subprocess wrapper: `swaymsg -t get_tree`, walk the JSON, build
/// ExposeCard for each window leaf (normal window_type only).
fn load_cards_via_swaymsg() -> Vec<ExposeCard> {
    use std::process::Command;
    let output = Command::new("swaymsg")
        .args(["-t", "get_tree"])
        .stderr(std::process::Stdio::null())
        .output();
    let Ok(output) = output else {
        tracing::warn!("expose: swaymsg get_tree spawn failed");
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let json: serde_json::Value = match serde_json::from_slice(&output.stdout) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "expose: get_tree parse failed");
            return Vec::new();
        }
    };
    let mut out = Vec::new();
    walk_tree_for_cards(&json, &mut out);
    out
}

fn walk_tree_for_cards(node: &serde_json::Value, out: &mut Vec<ExposeCard>) {
    if node.get("pid").is_some_and(|v| !v.is_null()) {
        let con_id = node.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
        if con_id != 0 {
            let title = node
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
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
            out.push(ExposeCard {
                con_id,
                title,
                app_id,
            });
        }
    }
    for key in ["nodes", "floating_nodes"] {
        if let Some(arr) = node.get(key).and_then(|v| v.as_array()) {
            for child in arr {
                walk_tree_for_cards(child, out);
            }
        }
    }
}

fn swaymsg_focus(con_id: u64) {
    use std::process::Command;
    let selector = format!("[con_id={con_id}] focus");
    match Command::new("swaymsg")
        .arg(&selector)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            let _ = child.wait();
        }
        Err(e) => tracing::warn!(con_id, error = %e, "expose focus spawn failed"),
    }
}

fn card_view(card: &ExposeCard) -> Element<'_, Message> {
    let title = truncate_title(&card.title, 40);
    let app_id = card.app_id.clone();
    button(
        column![
            text(title).size(13).color(FG_TEXT),
            Space::new().height(Length::Fixed(4.0)),
            text(app_id).size(11).color(FG_MUTED),
        ]
        .padding(Padding {
            top: 16.0,
            right: 12.0,
            bottom: 16.0,
            left: 12.0,
        }),
    )
    .width(Length::Fixed(200.0))
    .height(Length::Fixed(120.0))
    .style(card_button_style)
    .on_press(Message::FocusWindow(card.con_id))
    .into()
}

fn surface_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE_BG)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        text_color: Some(FG_TEXT),
        shadow: Shadow::default(),
        snap: false,
    }
}

fn card_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color {
            r: ACCENT.r,
            g: ACCENT.g,
            b: ACCENT.b,
            a: 0.30,
        },
        button::Status::Pressed => Color {
            r: ACCENT.r,
            g: ACCENT.g,
            b: ACCENT.b,
            a: 0.45,
        },
        _ => Color {
            r: 0.106,
            g: 0.106,
            b: 0.114,
            a: 0.92,
        },
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: FG_TEXT,
        border: Border {
            color: Color {
                r: 0.957,
                g: 0.957,
                b: 0.957,
                a: 0.10,
            },
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_columns_minimum_one() {
        assert_eq!(grid_columns(0), 1);
        assert_eq!(grid_columns(1), 1);
    }

    #[test]
    fn grid_columns_caps_at_six() {
        assert_eq!(grid_columns(100), 6);
        assert_eq!(grid_columns(36), 6);
    }

    #[test]
    fn grid_columns_uses_ceil_sqrt() {
        assert_eq!(grid_columns(2), 2);
        assert_eq!(grid_columns(4), 2);
        assert_eq!(grid_columns(5), 3);
        assert_eq!(grid_columns(9), 3);
    }

    #[test]
    fn truncate_title_passes_short_strings() {
        assert_eq!(truncate_title("hi", 10), "hi");
    }

    #[test]
    fn truncate_title_ellipsizes_long_strings() {
        let out = truncate_title("0123456789abcdef", 8);
        assert_eq!(out.chars().count(), 8);
        assert!(out.ends_with('…'));
    }

    /// v3.0.3 — walk_tree_for_cards finds windows nested under
    /// workspace/container nodes. The tree shape mimics sway's
    /// get_tree output (workspace → splith container → window).
    #[test]
    fn walk_tree_for_cards_finds_nested_window_leaves() {
        let tree = serde_json::json!({
            "id": 1, "pid": null,
            "nodes": [
                { "id": 2, "pid": null,
                  "nodes": [
                      { "id": 100, "pid": 9000, "name": "Terminal", "app_id": "foot" }
                  ]
                }
            ]
        });
        let mut out = Vec::new();
        walk_tree_for_cards(&tree, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].con_id, 100);
        assert_eq!(out[0].title, "Terminal");
        assert_eq!(out[0].app_id, "foot");
    }

    /// floating_nodes are walked too (sway puts floating windows
    /// on a separate per-workspace list).
    #[test]
    fn walk_tree_for_cards_descends_into_floating_nodes() {
        let tree = serde_json::json!({
            "id": 1, "pid": null,
            "nodes": [],
            "floating_nodes": [
                { "id": 50, "pid": 1, "name": "imv", "app_id": "imv" }
            ]
        });
        let mut out = Vec::new();
        walk_tree_for_cards(&tree, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].con_id, 50);
    }

    /// X11 (xwayland) windows expose `window_properties.class`
    /// instead of `app_id`. The card builder falls through.
    #[test]
    fn walk_tree_for_cards_falls_back_to_xwayland_class() {
        let tree = serde_json::json!({
            "id": 1, "pid": null,
            "nodes": [
                { "id": 7, "pid": 1, "name": "X11",
                  "window_properties": { "class": "XTerm" } }
            ]
        });
        let mut out = Vec::new();
        walk_tree_for_cards(&tree, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].app_id, "XTerm");
    }

}
