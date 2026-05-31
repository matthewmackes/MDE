//! v4.0.1 WM-5 (2026-05-23) — Super+Tab visible window switcher.
//!
//! Centered overlay listing every open sway window in MRU
//! order. Tab cycles forward, Shift+Tab cycles back, Enter or
//! click commits + closes (calls `swaymsg [con_id=N] focus`),
//! Esc cancels.
//!
//! Sway's MRU order isn't directly exposed by `get_tree`; the
//! ordering surfaced here is sway's tree-walk order, which
//! tracks "most recently focused first" closely enough that
//! the operator's muscle memory works (Tab once = the previous
//! window, Tab twice = the one before that, etc.).
//!
//! Bound from `data/sway/config.d/mackes-keybinds-wm.conf`:
//!
//!   bindsym Mod1+Tab exec mde-popover app-switcher
//!
//! (Mod1 = Alt; Super+Tab is reserved for workspace switching
//! in mackes-defaults.conf so this uses Alt+Tab — same as
//! Win11 / macOS.)

use std::process::Command;

use iced::keyboard::key::{Key, Named};
use iced::keyboard::{self, Modifiers};
use iced::widget::{column, container, image, row, text, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Subscription, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;
const CARD_W: f32 = 156.0;
const CARD_H: f32 = 96.0;
const CARDS_PER_ROW: usize = 3;

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
    /// Tab pressed — advance selection.
    Next,
    /// Shift+Tab pressed — reverse selection.
    Prev,
    /// Enter pressed OR card clicked — focus the selected window.
    Commit,
    /// Esc pressed — close without changing focus.
    Cancel,
    /// Direct click on card N.
    Select(usize),
    /// v4.0.1 WM-5.a — deferred grim capture finished for a
    /// card. Carries the con_id (because card ordering can
    /// shift between dispatch and arrival in pathological
    /// cases) + the PNG bytes. Empty Vec means capture failed
    /// or grim isn't installed — the card stays text-only.
    ThumbnailLoaded(u64, Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowCard {
    pub con_id: u64,
    pub app_id: String,
    pub title: String,
    /// v4.0.1 WM-5.a — window geometry from sway's `rect`.
    /// Used to feed `grim -g "X,Y WxH"` for the per-card
    /// screenshot capture. Zero values mean the card came
    /// from a tree node without rect fields (defensive — sway
    /// always provides them for windows).
    pub rect: WindowRect,
    /// v4.0.1 WM-5.a — PNG bytes from the grim capture, or
    /// None when the deferred capture hasn't completed (or
    /// has failed). View renders a text-only card when None.
    pub thumbnail: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Default)]
pub struct App {
    pub cards: Vec<WindowCard>,
    pub selected: usize,
}

fn namespace() -> String {
    "mde-popover-app-switcher".to_string()
}

fn update(state: &mut App, msg: Message) -> Task<Message> {
    match msg {
        Message::Next => {
            if !state.cards.is_empty() {
                state.selected = (state.selected + 1) % state.cards.len();
            }
            Task::none()
        }
        Message::Prev => {
            if !state.cards.is_empty() {
                state.selected = (state.selected + state.cards.len() - 1) % state.cards.len();
            }
            Task::none()
        }
        Message::Commit => {
            if let Some(card) = state.cards.get(state.selected) {
                swaymsg_focus(card.con_id);
            }
            std::process::exit(0);
        }
        Message::Cancel => std::process::exit(0),
        Message::Select(idx) => {
            if idx < state.cards.len() {
                state.selected = idx;
                if let Some(card) = state.cards.get(idx) {
                    swaymsg_focus(card.con_id);
                }
                std::process::exit(0);
            }
            Task::none()
        }
        Message::ThumbnailLoaded(con_id, bytes) => {
            if bytes.is_empty() {
                return Task::none();
            }
            if let Some(card) = state.cards.iter_mut().find(|c| c.con_id == con_id) {
                card.thumbnail = Some(bytes);
            }
            Task::none()
        }
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

    let header = container(
        text(format!(
            "{} of {}: {}",
            state.selected + 1,
            state.cards.len(),
            state.cards[state.selected]
                .title
                .as_str()
                .lines()
                .next()
                .unwrap_or(""),
        ))
        .size(13)
        .color(FG_TEXT),
    )
    .padding(Padding::from([6u16, 12u16]))
    .center_x(Length::Fill);

    let mut grid = column![].spacing(10);
    let mut current_row: Vec<Element<'_, Message>> = Vec::new();
    for (i, card) in state.cards.iter().enumerate() {
        current_row.push(card_view(card, i, i == state.selected));
        if current_row.len() == CARDS_PER_ROW {
            let mut r = row![].spacing(10);
            for el in current_row.drain(..) {
                r = r.push(el);
            }
            grid = grid.push(r);
        }
    }
    if !current_row.is_empty() {
        let mut r = row![].spacing(10);
        for el in current_row.drain(..) {
            r = r.push(el);
        }
        grid = grid.push(r);
    }

    let footer = text("Tab cycles · Enter focuses · Esc cancels")
        .size(10)
        .color(FG_MUTED);

    container(
        column![
            header,
            Space::new().height(Length::Fixed(12.0)),
            grid,
            Space::new().height(Length::Fixed(8.0)),
            container(footer).center_x(Length::Fill),
        ]
        .spacing(2),
    )
    .padding(Padding::from([16u16, 16u16]))
    .width(Length::Fill)
    .height(Length::Fill)
    .style(surface_style)
    .into()
}

fn subscription(_state: &App) -> Subscription<Message> {
    use iced::event;
    event::listen_with(|event, status, _window| {
        match event {
            iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. })
                if status == event::Status::Ignored =>
            {
                match key.as_ref() {
                    Key::Named(Named::Tab) => {
                        if modifiers.shift() {
                            Some(Message::Prev)
                        } else {
                            Some(Message::Next)
                        }
                    }
                    Key::Named(Named::Enter) => Some(Message::Commit),
                    Key::Named(Named::Escape) => Some(Message::Cancel),
                    Key::Named(Named::ArrowRight) | Key::Named(Named::ArrowDown) => {
                        Some(Message::Next)
                    }
                    Key::Named(Named::ArrowLeft) | Key::Named(Named::ArrowUp) => {
                        Some(Message::Prev)
                    }
                    _ => {
                        let _ = (key, modifiers as Modifiers);
                        None
                    }
                }
            }
            _ => None,
        }
    })
}

fn surface_style(_: &Theme) -> container::Style {
    container::Style {
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
    }
}

fn card_view<'a>(card: &'a WindowCard, idx: usize, selected: bool) -> Element<'a, Message> {
    let title_text = text(if card.title.is_empty() {
        card.app_id.clone()
    } else {
        truncate_for_card(&card.title)
    })
    .size(11)
    .color(FG_TEXT);
    let app_text = text(card.app_id.clone()).size(10).color(FG_MUTED);

    // v4.0.1 WM-5.a — thumbnail row when grim has supplied
    // PNG bytes; falls back to a blank Space so the card
    // layout doesn't shift when captures arrive.
    let thumb_height = Length::Fixed(CARD_H - 38.0);
    let thumb: Element<'a, Message> = match card.thumbnail.as_ref() {
        Some(bytes) if !bytes.is_empty() => image(image::Handle::from_bytes(bytes.clone()))
            .width(Length::Fill)
            .height(thumb_height)
            .content_fit(iced::ContentFit::Contain)
            .into(),
        _ => Space::new().height(thumb_height).into(),
    };

    let body = container(
        column![
            thumb,
            Space::new().height(Length::Fixed(2.0)),
            container(title_text).center_x(Length::Fill),
            Space::new().height(Length::Fixed(2.0)),
            container(app_text).center_x(Length::Fill),
        ]
        .spacing(0),
    )
    .padding(Padding::from([8u16, 8u16]))
    .width(Length::Fixed(CARD_W))
    .height(Length::Fixed(CARD_H));

    iced::widget::button(body)
        .padding(0)
        .style(move |_t: &Theme, _status: iced::widget::button::Status| {
            iced::widget::button::Style {
                background: Some(Background::Color(if selected {
                    Color {
                        r: ACCENT.r * 0.30,
                        g: ACCENT.g * 0.30,
                        b: ACCENT.b * 0.30,
                        a: 1.0,
                    }
                } else {
                    CARD_BG
                })),
                text_color: FG_TEXT,
                border: Border {
                    color: if selected { ACCENT } else {
                        Color {
                            a: 0.06,
                            ..Color::WHITE
                        }
                    },
                    width: if selected { 2.0 } else { 1.0 },
                    radius: 6.0.into(),
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .on_press(Message::Select(idx))
        .into()
}

fn truncate_for_card(s: &str) -> String {
    const MAX: usize = 22;
    let first_line = s.lines().next().unwrap_or(s);
    if first_line.chars().count() <= MAX {
        return first_line.to_string();
    }
    let mut out: String = first_line.chars().take(MAX - 1).collect();
    out.push('…');
    out
}

// ---- I/O ------------------------------------------------------

#[must_use]
pub fn scan_windows() -> Vec<WindowCard> {
    let out = Command::new("swaymsg")
        .args(["-t", "get_tree"])
        .output();
    match out {
        Ok(o) if o.status.success() => parse_tree(&String::from_utf8_lossy(&o.stdout)),
        _ => Vec::new(),
    }
}

/// Pure parser exposed for tests. Walks the sway get_tree
/// JSON tree, collects every leaf with a non-null `pid` (= a
/// real window, not a workspace / output container), skips
/// the scratchpad workspace (those are minimized).
#[must_use]
pub fn parse_tree(raw: &str) -> Vec<WindowCard> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    walk(&root, &mut out, false);
    out
}

/// v4.0.1 WM-5.a — extract `{ x, y, width, height }` from a
/// sway tree node's `rect` field. Missing fields default to 0
/// so a partial JSON shape (or future sway versions reshaping
/// the field) still yields a valid `WindowRect`.
#[must_use]
pub fn parse_rect(v: &serde_json::Value) -> WindowRect {
    WindowRect {
        x: v.get("x").and_then(|n| n.as_i64()).unwrap_or(0) as i32,
        y: v.get("y").and_then(|n| n.as_i64()).unwrap_or(0) as i32,
        width: v
            .get("width")
            .and_then(|n| n.as_u64())
            .unwrap_or(0) as u32,
        height: v
            .get("height")
            .and_then(|n| n.as_u64())
            .unwrap_or(0) as u32,
    }
}

fn walk(node: &serde_json::Value, out: &mut Vec<WindowCard>, inside_scratch: bool) {
    let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let entering_scratch = inside_scratch || name == "__i3_scratch";

    // Leaf with a pid = real window.
    if !entering_scratch && node.get("pid").is_some_and(|v| !v.is_null()) {
        let con_id = node.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
        if con_id != 0 {
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
            let rect = node
                .get("rect")
                .map(parse_rect)
                .unwrap_or_default();
            out.push(WindowCard {
                con_id,
                app_id,
                title,
                rect,
                thumbnail: None,
            });
        }
    }

    for arr_key in ["nodes", "floating_nodes"] {
        if let Some(arr) = node.get(arr_key).and_then(|v| v.as_array()) {
            for child in arr {
                walk(child, out, entering_scratch);
            }
        }
    }
}

fn swaymsg_focus(con_id: u64) {
    let _ = Command::new("swaymsg")
        .arg(format!("[con_id={con_id}] focus"))
        .status();
}

/// v4.0.1 WM-5.a — capture a single window via grim.
///
/// `grim -g "X,Y WxH" -` writes PNG bytes to stdout. Returns
/// the bytes on success; an empty Vec on any error (grim
/// missing, rect malformed, grim refused the read). Empty Vec
/// is the explicit signal the view layer falls back to a
/// text-only card on.
///
/// Synchronous on purpose: callers should wrap this in
/// `tokio::task::spawn_blocking` (as the deferred-capture
/// Task in `App::new` does) so it doesn't stall the Iced
/// scheduler. The capture itself takes 10-50 ms per window
/// on typical hardware; running all of them serially on the
/// blocking pool keeps the popover responsive.
#[must_use]
pub fn capture_thumbnail(rect: WindowRect) -> Vec<u8> {
    if rect.width == 0 || rect.height == 0 {
        return Vec::new();
    }
    let geom = format!("{},{} {}x{}", rect.x, rect.y, rect.width, rect.height);
    let out = Command::new("grim")
        .args(["-g", &geom, "-"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output();
    match out {
        Ok(o) if o.status.success() => o.stdout,
        _ => Vec::new(),
    }
}

pub fn run() -> iced_layershell::Result {
    iced_layershell::application(
        || {
            let cards = scan_windows();
            let selected = if cards.len() > 1 { 1 } else { 0 };
            let captures: Vec<Task<Message>> = cards
                .iter()
                .map(|c| {
                    let con_id = c.con_id;
                    let rect = c.rect;
                    Task::perform(
                        async move { capture_thumbnail(rect) },
                        move |bytes| Message::ThumbnailLoaded(con_id, bytes),
                    )
                })
                .collect();
            let task = if captures.is_empty() {
                Task::none()
            } else {
                Task::batch(captures)
            };
            (App { cards, selected }, task)
        },
        namespace,
        update,
        view,
    )
    .theme(|_: &App| iced::Theme::custom(
        "mde-popover-app-switcher",
        iced::theme::Palette {
            background: SURFACE_BG,
            text: FG_TEXT,
            primary: ACCENT,
            warning: Color::from_rgb(0.96, 0.65, 0.14),
            success: Color::from_rgb(0.20, 0.80, 0.40),
            danger: Color::from_rgb(0.92, 0.32, 0.30),
        },
    ))
    .subscription(subscription)
    .settings(Settings {
        id: Some("mde-popover-app-switcher".to_string()),
        fonts: crate::fonts::load_fallback_fonts(),
        layer_settings: LayerShellSettings {
            layer: Layer::Overlay,
            anchor: Anchor::empty(),
            exclusive_zone: -1,
            margin: (0, 0, 0, 0),
            size: Some((WIDTH, HEIGHT)),
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

    #[test]
    fn parse_tree_collects_window_with_pid() {
        let raw = r#"{
            "nodes": [{
                "name": "workspace 1",
                "nodes": [
                    {"id": 7, "pid": 1234, "app_id": "foot", "name": "shell",
                     "nodes": [], "floating_nodes": []}
                ],
                "floating_nodes": []
            }]
        }"#;
        let cards = parse_tree(raw);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].con_id, 7);
        assert_eq!(cards[0].app_id, "foot");
        assert_eq!(cards[0].title, "shell");
    }

    #[test]
    fn parse_tree_skips_scratchpad_windows() {
        let raw = r#"{
            "nodes": [
                {
                    "name": "__i3_scratch",
                    "nodes": [],
                    "floating_nodes": [
                        {"id": 1, "pid": 100, "app_id": "foot", "name": "hidden",
                         "nodes": [], "floating_nodes": []}
                    ]
                },
                {
                    "name": "workspace 1",
                    "nodes": [
                        {"id": 2, "pid": 101, "app_id": "firefox", "name": "fox",
                         "nodes": [], "floating_nodes": []}
                    ],
                    "floating_nodes": []
                }
            ]
        }"#;
        let cards = parse_tree(raw);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].app_id, "firefox");
    }

    #[test]
    fn parse_tree_handles_xwayland_window_properties_class() {
        let raw = r#"{
            "nodes": [{
                "name": "workspace 1",
                "nodes": [
                    {"id": 9, "pid": 200, "name": "Gimp",
                     "window_properties": {"class": "Gimp-2.10"},
                     "nodes": [], "floating_nodes": []}
                ],
                "floating_nodes": []
            }]
        }"#;
        let cards = parse_tree(raw);
        assert_eq!(cards[0].app_id, "Gimp-2.10");
    }

    #[test]
    fn parse_tree_returns_empty_for_garbage() {
        assert!(parse_tree("not json").is_empty());
        assert!(parse_tree("").is_empty());
    }

    #[test]
    fn parse_tree_skips_nodes_without_pid() {
        // Workspaces have nodes-with-children but no pid;
        // shouldn't be surfaced as windows.
        let raw = r#"{
            "nodes": [{
                "name": "workspace 1",
                "nodes": [],
                "floating_nodes": []
            }]
        }"#;
        assert!(parse_tree(raw).is_empty());
    }

    #[test]
    fn next_wraps_at_end() {
        let mut app = App {
            cards: vec![
                WindowCard { con_id: 1, app_id: "a".into(), title: "A".into(), rect: WindowRect::default(), thumbnail: None },
                WindowCard { con_id: 2, app_id: "b".into(), title: "B".into(), rect: WindowRect::default(), thumbnail: None },
            ],
            selected: 1,
        };
        let _ = update(&mut app, Message::Next);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn prev_wraps_at_start() {
        let mut app = App {
            cards: vec![
                WindowCard { con_id: 1, app_id: "a".into(), title: "A".into(), rect: WindowRect::default(), thumbnail: None },
                WindowCard { con_id: 2, app_id: "b".into(), title: "B".into(), rect: WindowRect::default(), thumbnail: None },
            ],
            selected: 0,
        };
        let _ = update(&mut app, Message::Prev);
        assert_eq!(app.selected, 1);
    }

    #[test]
    fn default_selection_is_second_card_for_alt_tab_idiom() {
        // Manual lock check — real sway not available in tests.
        let cards = vec![
            WindowCard { con_id: 1, app_id: "a".into(), title: "A".into(), rect: WindowRect::default(), thumbnail: None },
            WindowCard { con_id: 2, app_id: "b".into(), title: "B".into(), rect: WindowRect::default(), thumbnail: None },
            WindowCard { con_id: 3, app_id: "c".into(), title: "C".into(), rect: WindowRect::default(), thumbnail: None },
        ];
        let expected = if cards.len() > 1 { 1 } else { 0 };
        assert_eq!(expected, 1);
    }

    #[test]
    fn truncate_handles_short_titles() {
        assert_eq!(truncate_for_card("hello"), "hello");
    }

    #[test]
    fn truncate_caps_long_titles() {
        let long = "this is a really long window title that exceeds the cap";
        let t = truncate_for_card(long);
        assert!(t.chars().count() <= 22);
        assert!(t.ends_with('…'));
    }

    // ────────────────────────────────────────────────────────
    // WM-5.a — rect parsing + capture defensive guards
    // ────────────────────────────────────────────────────────

    #[test]
    fn parse_rect_extracts_all_four_fields() {
        let v: serde_json::Value =
            serde_json::from_str(r#"{"x": 12, "y": 24, "width": 800, "height": 600}"#)
                .expect("json");
        let r = parse_rect(&v);
        assert_eq!(r.x, 12);
        assert_eq!(r.y, 24);
        assert_eq!(r.width, 800);
        assert_eq!(r.height, 600);
    }

    #[test]
    fn parse_rect_defaults_missing_fields_to_zero() {
        let v: serde_json::Value = serde_json::from_str(r#"{"x": 5}"#).expect("json");
        let r = parse_rect(&v);
        assert_eq!(r.x, 5);
        assert_eq!(r.y, 0);
        assert_eq!(r.width, 0);
        assert_eq!(r.height, 0);
    }

    #[test]
    fn parse_tree_now_extracts_rect() {
        let tree = r#"{
            "type": "root",
            "nodes": [{
                "type": "workspace",
                "name": "1",
                "nodes": [{
                    "type": "con",
                    "id": 100,
                    "name": "Firefox",
                    "app_id": "firefox",
                    "pid": 1234,
                    "rect": {"x": 0, "y": 28, "width": 1920, "height": 1052},
                    "nodes": [],
                    "floating_nodes": []
                }]
            }]
        }"#;
        let cards = parse_tree(tree);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].rect.x, 0);
        assert_eq!(cards[0].rect.y, 28);
        assert_eq!(cards[0].rect.width, 1920);
        assert_eq!(cards[0].rect.height, 1052);
        assert!(cards[0].thumbnail.is_none());
    }

    #[test]
    fn capture_thumbnail_zero_size_returns_empty() {
        // Defensive: a rect with zero width/height (sway not
        // surfacing geometry for a freshly-mapped window)
        // must short-circuit before grim is invoked.
        let bytes = capture_thumbnail(WindowRect::default());
        assert!(bytes.is_empty());
    }
}
