//! v4.0.1 WM-2 (2026-05-23) — minimized-windows popover.
//!
//! Lists every window sway has parked in its `__i3_scratch`
//! workspace (= "minimized" in MDE's UX vocabulary). Each row
//! shows `app_id` + window title; click restores the window
//! via `swaymsg [con_id=N] scratchpad show`.
//!
//! The original WM-2 spec called for both this popover + a
//! panel tray button with a badge count. This commit ships
//! the popover half so it's already invocable via keybind /
//! CLI; the tray-button half is WM-2.a (panel-side state
//! plumbing).
//!
//! Bind it from sway with:
//!
//!   bindsym $mod+Shift+s exec mde-popover minimized
//!
//! Anchor: top-right; 360 × auto-height.

use std::process::Command;

use iced::widget::{button, column, container, mouse_area, row, scrollable, text, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

const WIDTH: u32 = 360;
const HEIGHT: u32 = 420;

const ACCENT: Color = Color {
    r: 0.357,
    g: 0.416,
    b: 0.961,
    a: 1.0,
};
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
const FG_FAINT: Color = Color {
    r: 0.450,
    g: 0.450,
    b: 0.450,
    a: 1.0,
};
const SURFACE_BG: Color = Color {
    r: 0.055,
    g: 0.055,
    b: 0.063,
    a: 0.97,
};
const CARD_BG: Color = Color {
    r: 0.110,
    g: 0.110,
    b: 0.118,
    a: 1.0,
};

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    /// Restore the window with the given sway `con_id` (then close).
    Restore(i64),
    /// Esc / close button → exit the popover process.
    Esc,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MinimizedRow {
    pub con_id: i64,
    pub app_id: String,
    pub title: String,
}

#[derive(Debug, Default)]
pub struct App {
    pub rows: Vec<MinimizedRow>,
}

fn namespace() -> String {
    "mde-popover-minimized".to_string()
}

fn update(state: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::Refresh => {
            state.rows = scan_scratchpad();
            Task::none()
        }
        Message::Restore(con_id) => {
            let _ = Command::new("swaymsg")
                .args([&format!("[con_id={con_id}]"), "scratchpad", "show"])
                .status();
            std::process::exit(0);
        }
        Message::Esc => std::process::exit(0),
        _ => Task::none(),
    }
}

fn view(state: &App) -> Element<'_, Message> {
    let title = text("Minimized windows").size(15).color(FG_TEXT);
    let subtitle = text(format!(
        "{} window{} in scratchpad",
        state.rows.len(),
        if state.rows.len() == 1 { "" } else { "s" }
    ))
    .size(11)
    .color(FG_MUTED);

    let refresh_btn = button(text("Refresh").size(11).color(FG_TEXT))
        .padding(Padding::from([4u16, 10u16]))
        .style(|_, status| ghost_btn_style(status))
        .on_press(Message::Refresh);

    let header = row![
        column![title, subtitle].spacing(2),
        Space::new().width(Length::Fill),
        refresh_btn,
    ]
    .align_y(iced::alignment::Vertical::Center);

    let mut rows_col = column![].spacing(6);
    for r in &state.rows {
        rows_col = rows_col.push(window_row(r));
    }
    if state.rows.is_empty() {
        rows_col = rows_col.push(empty_card());
    }

    let footer = text("Esc / click outside closes · click a row to restore")
        .size(10)
        .color(FG_FAINT);

    // The visible content card. Identical layout to before;
    // wrapped in a fixed-size container so the surrounding
    // backdrop pixels stay click-receptive.
    let card: Element<'_, Message> = container(
        column![
            header,
            Space::new().height(Length::Fixed(12.0)),
            scrollable(rows_col).height(Length::Fill),
            Space::new().height(Length::Fixed(8.0)),
            footer,
        ]
        .spacing(2),
    )
    .padding(Padding::from([16u16, 18u16]))
    .width(Length::Fixed(WIDTH as f32))
    .height(Length::Fixed(HEIGHT as f32))
    .style(|_| container::Style {
        background: Some(Background::Color(SURFACE_BG)),
        border: Border {
            color: Color {
                a: 0.08,
                ..Color::WHITE
            },
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(FG_TEXT),
        snap: false,
    })
    .into();

    // v3.0.4 (2026-05-23) — backdrop: fullscreen surface,
    // card pinned to top-right, every other pixel is a
    // mouse_area that fires Esc on click. Outer container
    // paints transparent so the wallpaper / running windows
    // show through.
    let dismiss = || {
        mouse_area(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(Message::Esc)
    };
    let top_strip = row![
        dismiss(),
        container(card)
            .padding(Padding {
                top: 44.0,
                right: 14.0,
                bottom: 0.0,
                left: 0.0,
            }),
    ]
    .height(Length::Fixed((HEIGHT + 44) as f32));
    container(
        column![
            top_strip,
            dismiss(),
        ],
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_| container::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: Shadow::default(),
        text_color: None,
        snap: false,
    })
    .into()
}

fn window_row<'a>(r: &'a MinimizedRow) -> Element<'a, Message> {
    let title_text = text(if r.title.is_empty() {
        r.app_id.clone()
    } else {
        r.title.clone()
    })
    .size(13)
    .color(FG_TEXT);
    let id_text = text(format!("{} · con_id={}", r.app_id, r.con_id))
        .size(11)
        .color(FG_MUTED);

    button(
        container(column![title_text, id_text].spacing(2))
            .padding(Padding::from([8u16, 12u16]))
            .width(Length::Fill),
    )
    .padding(0)
    .style(|_, status| iced::widget::button::Style {
        background: Some(Background::Color(match status {
            iced::widget::button::Status::Hovered => Color {
                r: 0.15,
                g: 0.15,
                b: 0.18,
                a: 1.0,
            },
            _ => CARD_BG,
        })),
        text_color: FG_TEXT,
        border: Border {
            color: Color {
                a: 0.06,
                ..Color::WHITE
            },
            width: 1.0,
            radius: 5.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    })
    .on_press(Message::Restore(r.con_id))
    .into()
}

fn empty_card<'a>() -> Element<'a, Message> {
    container(
        column![
            text("Nothing minimized").size(13).color(FG_MUTED),
            text("Send a window to scratchpad with Super+Shift+- (or your binding).")
                .size(11)
                .color(FG_FAINT),
        ]
        .spacing(4)
        .align_x(iced::alignment::Horizontal::Center),
    )
    .padding(Padding::from([24u16, 12u16]))
    .width(Length::Fill)
    .into()
}

fn ghost_btn_style(status: iced::widget::button::Status) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color {
            r: 0.15,
            g: 0.15,
            b: 0.17,
            a: 1.0,
        },
        _ => Color::TRANSPARENT,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: FG_TEXT,
        border: Border {
            color: Color {
                a: 0.10,
                ..Color::WHITE
            },
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

// ---- I/O ------------------------------------------------------

/// Walk `swaymsg -t get_tree` and pull out the scratchpad
/// workspace's windows. Returns rows in get_tree's order
/// (sway-defined; typically newest-last). Empty Vec if
/// swaymsg fails or no scratchpad nodes exist.
#[must_use]
pub fn scan_scratchpad() -> Vec<MinimizedRow> {
    let out = Command::new("swaymsg").args(["-t", "get_tree"]).output();
    match out {
        Ok(o) if o.status.success() => {
            parse_scratchpad(&String::from_utf8_lossy(&o.stdout))
        }
        _ => Vec::new(),
    }
}

/// Pure parser exposed for tests. Walks the sway get_tree
/// JSON tree, finds the scratchpad workspace (`name ==
/// "__i3_scratch"`), and collects con_id + app_id + name for
/// every leaf inside it. Sway nests scratchpad windows in
/// `floating_nodes` (not `nodes`) since they're tossed into
/// the scratch workspace as floating containers.
#[must_use]
pub fn parse_scratchpad(raw: &str) -> Vec<MinimizedRow> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    walk_for_scratchpad(&root, &mut out);
    out
}

fn walk_for_scratchpad(node: &serde_json::Value, out: &mut Vec<MinimizedRow>) {
    let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("");
    if name == "__i3_scratch" {
        // Collect every leaf inside this workspace's nodes +
        // floating_nodes recursively.
        for arr_key in ["nodes", "floating_nodes"] {
            if let Some(arr) = node.get(arr_key).and_then(|v| v.as_array()) {
                for child in arr {
                    collect_leaves(child, out);
                }
            }
        }
        return;
    }
    // Otherwise descend through nodes + floating_nodes looking
    // for the scratchpad workspace.
    for arr_key in ["nodes", "floating_nodes"] {
        if let Some(arr) = node.get(arr_key).and_then(|v| v.as_array()) {
            for child in arr {
                walk_for_scratchpad(child, out);
            }
        }
    }
}

fn collect_leaves(node: &serde_json::Value, out: &mut Vec<MinimizedRow>) {
    let con_id = node.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
    let app_id = node
        .get("app_id")
        .and_then(|v| v.as_str())
        .or_else(|| {
            node.pointer("/window_properties/class")
                .and_then(|v| v.as_str())
        })
        .unwrap_or("")
        .to_string();
    let title = node
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let kids_empty = node
        .get("nodes")
        .and_then(|v| v.as_array())
        .map(|a| a.is_empty())
        .unwrap_or(true)
        && node
            .get("floating_nodes")
            .and_then(|v| v.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(true);
    if kids_empty && con_id != 0 && (!app_id.is_empty() || !title.is_empty()) {
        out.push(MinimizedRow {
            con_id,
            app_id,
            title,
        });
    } else {
        for arr_key in ["nodes", "floating_nodes"] {
            if let Some(arr) = node.get(arr_key).and_then(|v| v.as_array()) {
                for child in arr {
                    collect_leaves(child, out);
                }
            }
        }
    }
}

pub fn run() -> iced_layershell::Result {
    iced_layershell::application(
        || App { rows: scan_scratchpad() },
        namespace,
        update,
        view,
    )
    .theme(|_: &App| iced::Theme::custom(
        "mde-popover-minimized",
        iced::theme::Palette {
            background: SURFACE_BG,
            text: FG_TEXT,
            primary: ACCENT,
            warning: Color::from_rgb(0.96, 0.65, 0.14),
            success: Color::from_rgb(0.20, 0.80, 0.40),
            danger: Color::from_rgb(0.92, 0.32, 0.30),
        },
    ))
    .settings(Settings {
        id: Some("mde-popover-minimized".to_string()),
        fonts: crate::fonts::load_fallback_fonts(),
        layer_settings: LayerShellSettings {
            // v3.0.4 (2026-05-23) — fullscreen surface so the
            // outer mouse_area covering the rest of the screen
            // can catch click-outside-to-dismiss events. The
            // visible card stays at its previous 360×420 size,
            // positioned in the top-right corner by the view's
            // layout. The non-card pixels paint transparent so
            // the wallpaper / running windows show through.
            layer: Layer::Top,
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            margin: (0, 0, 0, 0),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            exclusive_zone: -1,
            size: None,
            ..Default::default()
        },
        ..Default::default()
    })
    .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scratchpad_returns_empty_for_garbage() {
        assert!(parse_scratchpad("not json").is_empty());
        assert!(parse_scratchpad("").is_empty());
    }

    #[test]
    fn parse_scratchpad_finds_one_window() {
        // Minimal get_tree shape: root → outputs[] → workspaces[]
        // where one workspace is __i3_scratch with a leaf
        // floating_nodes entry.
        let raw = r#"{
            "nodes": [
                {
                    "name": "DP-1",
                    "nodes": [
                        {
                            "name": "__i3_scratch",
                            "nodes": [],
                            "floating_nodes": [
                                {"id": 42, "app_id": "foot", "name": "shell",
                                 "nodes": [], "floating_nodes": []}
                            ]
                        }
                    ]
                }
            ]
        }"#;
        let rows = parse_scratchpad(raw);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].con_id, 42);
        assert_eq!(rows[0].app_id, "foot");
        assert_eq!(rows[0].title, "shell");
    }

    #[test]
    fn parse_scratchpad_ignores_non_scratch_workspaces() {
        let raw = r#"{
            "nodes": [
                {
                    "name": "workspace 1",
                    "nodes": [
                        {"id": 7, "app_id": "firefox", "name": "x",
                         "nodes": [], "floating_nodes": []}
                    ],
                    "floating_nodes": []
                }
            ]
        }"#;
        assert!(parse_scratchpad(raw).is_empty());
    }

    #[test]
    fn parse_scratchpad_handles_xwayland_via_window_properties_class() {
        let raw = r#"{
            "nodes": [
                {
                    "name": "__i3_scratch",
                    "nodes": [],
                    "floating_nodes": [
                        {"id": 99, "name": "Gimp", "window_properties":
                            {"class": "Gimp-2.10"},
                         "nodes": [], "floating_nodes": []}
                    ]
                }
            ]
        }"#;
        let rows = parse_scratchpad(raw);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].app_id, "Gimp-2.10");
    }

    #[test]
    fn parse_scratchpad_handles_nested_containers() {
        let raw = r#"{
            "nodes": [
                {
                    "name": "__i3_scratch",
                    "nodes": [
                        {"id": 100, "nodes": [
                            {"id": 101, "app_id": "foot", "name": "a",
                             "nodes": [], "floating_nodes": []},
                            {"id": 102, "app_id": "foot", "name": "b",
                             "nodes": [], "floating_nodes": []}
                        ], "floating_nodes": []}
                    ],
                    "floating_nodes": []
                }
            ]
        }"#;
        let rows = parse_scratchpad(raw);
        // Both leaves found.
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].con_id, 101);
        assert_eq!(rows[1].con_id, 102);
    }
}
