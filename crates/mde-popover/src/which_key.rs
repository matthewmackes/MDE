//! `mde-popover which-key` — sway binding-mode overlay (Q55).
//!
//! Fullscreen semi-transparent backdrop with a centered card listing all
//! `bindsym` entries for the active sway binding mode. Mode name is read
//! from the `MDE_SWAY_MODE` environment variable (set by mde-portal's Dock
//! when it spawns this surface on `ModeChanged`).
//!
//! Click anywhere or press Esc to dismiss.

use std::path::PathBuf;

use iced::widget::{column, container, mouse_area, row, scrollable, text, Space};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Subscription, Task, Theme};
use iced::widget::container::Style as ContainerStyle;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

// ── Design tokens ─────────────────────────────────────────────────────────────

const CHARCOAL: Color = Color { r: 0.125, g: 0.129, b: 0.141, a: 0.97 };
const FG: Color = Color { r: 0.957, g: 0.957, b: 0.957, a: 1.0 };
const FG_DIM: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.55 };
const FG_LABEL: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.35 };
const ACCENT: Color = Color { r: 0.357, g: 0.416, b: 0.961, a: 1.0 };
const BACKDROP: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.55 };
const CARD_WIDTH: f32 = 440.0;
/// Cap the rendered rows so the card doesn't overflow the screen.
const MAX_ROWS: usize = 24;

// ── Binding struct ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Binding {
    key: String,
    action: String,
}

// ── Sway config parser ────────────────────────────────────────────────────────

fn sway_config_path() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("sway/config")
    } else if let Some(home) = dirs::home_dir() {
        home.join(".config/sway/config")
    } else {
        PathBuf::from("/etc/sway/config")
    }
}

/// Parse `bindsym` entries from inside `mode "<name>" { ... }` blocks.
/// Pure function for testability — does not touch the filesystem.
fn parse_mode_bindings_from_str(config: &str, mode_name: &str) -> Vec<Binding> {
    let needle = format!("mode \"{}\"", mode_name);
    let mut in_block = false;
    let mut depth: usize = 0;
    let mut bindings = Vec::new();

    for line in config.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        if !in_block {
            if trimmed.starts_with(&needle) && trimmed.contains('{') {
                in_block = true;
                depth = 1;
                continue;
            }
        } else {
            for ch in trimmed.chars() {
                match ch {
                    '{' => depth += 1,
                    '}' => depth = depth.saturating_sub(1),
                    _ => {}
                }
            }
            if depth == 0 {
                break;
            }
            if let Some(rest) = trimmed.strip_prefix("bindsym ") {
                // `rest` = "<key_combo> <command...>"
                let mut parts = rest.splitn(2, ' ');
                if let (Some(key), Some(action)) = (parts.next(), parts.next()) {
                    let key = key.trim().to_string();
                    let action = action.trim().to_string();
                    if !key.is_empty() && !action.is_empty() {
                        bindings.push(Binding { key, action });
                    }
                }
            }
        }
    }
    bindings
}

/// Read `~/.config/sway/config` and return bindings for the named mode.
fn parse_mode_bindings(mode_name: &str) -> Vec<Binding> {
    let raw = std::fs::read_to_string(sway_config_path()).unwrap_or_default();
    parse_mode_bindings_from_str(&raw, mode_name)
}

// ── Iced app ──────────────────────────────────────────────────────────────────

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Exit,
}

pub struct App {
    mode_name: String,
    bindings: Vec<Binding>,
}

fn namespace() -> String {
    "mde-popover-which-key".to_string()
}

fn update(_state: &mut App, msg: Message) -> Task<Message> {
    match msg {
        Message::Exit => std::process::exit(0),
        _ => Task::none(),
    }
}

fn view(state: &App) -> Element<'_, Message> {
    let heading = if state.mode_name.is_empty() {
        "Mode bindings".to_string()
    } else {
        format!("MODE: {}", state.mode_name.to_ascii_uppercase())
    };

    let header = container(
        text(heading).size(12).color(ACCENT),
    )
    .padding(Padding { top: 0.0, right: 0.0, bottom: 6.0, left: 0.0 });

    let body: Element<'_, Message> = if state.bindings.is_empty() {
        text("No bindings found for this mode.")
            .size(12)
            .color(FG_DIM)
            .into()
    } else {
        let mut rows = column![].spacing(4);
        for binding in state.bindings.iter().take(MAX_ROWS) {
            let key_cell = container(
                text(binding.key.clone()).size(12).color(FG),
            )
            .width(Length::Fixed(150.0));

            let sep = container(text("→").size(12).color(FG_LABEL))
                .width(Length::Fixed(20.0));

            let action_cell = container(
                text(binding.action.clone()).size(12).color(FG_DIM),
            )
            .width(Length::Fill);

            rows = rows.push(
                row![key_cell, sep, action_cell].align_y(Alignment::Center),
            );
        }
        if state.bindings.len() > MAX_ROWS {
            let extra = state.bindings.len() - MAX_ROWS;
            rows = rows.push(
                text(format!("… {extra} more"))
                    .size(11)
                    .color(FG_DIM),
            );
        }
        scrollable(rows).height(Length::Shrink).into()
    };

    let dismiss_hint = container(
        text("Esc or click outside to close").size(10).color(FG_LABEL),
    )
    .padding(Padding { top: 8.0, right: 0.0, bottom: 0.0, left: 0.0 });

    let card = container(
        column![header, body, dismiss_hint].padding(Padding::from([14, 16])),
    )
    .width(Length::Fixed(CARD_WIDTH))
    .style(|_: &Theme| ContainerStyle {
        background: Some(Background::Color(CHARCOAL)),
        border: Border {
            color: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.10 },
            width: 1.0,
            radius: 8.0.into(),
        },
        text_color: Some(FG),
        shadow: iced::Shadow::default(),
        snap: false,
    });

    let centered = column![
        Space::new().height(Length::Fill),
        container(card)
            .width(Length::Fill)
            .align_x(Alignment::Center),
        Space::new().height(Length::Fill),
    ];

    mouse_area(
        container(centered)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_: &Theme| ContainerStyle {
                background: Some(Background::Color(BACKDROP)),
                ..Default::default()
            }),
    )
    .on_press(Message::Exit)
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

pub fn run() -> iced_layershell::Result {
    iced_layershell::application(
        || {
            let mode_name = std::env::var("MDE_SWAY_MODE").unwrap_or_default();
            let bindings = if mode_name.is_empty() {
                Vec::new()
            } else {
                parse_mode_bindings(&mode_name)
            };
            App { mode_name, bindings }
        },
        namespace,
        update,
        view,
    )
    .theme(|_: &App| iced::Theme::Dark)
    .subscription(subscription)
    .settings(Settings {
        id: Some("mde-popover-which-key".to_string()),
        fonts: crate::fonts::load_fallback_fonts(),
        layer_settings: LayerShellSettings {
            size: None,
            exclusive_zone: -1,
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            margin: (0, 0, 0, 0),
            layer: Layer::Overlay,
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

    const SAMPLE_CONFIG: &str = r#"
# MDE sway config
bindsym $mod+r mode "resize"

mode "resize" {
    bindsym h resize shrink width 10 px
    bindsym j resize grow height 10 px
    bindsym k resize shrink height 10 px
    bindsym l resize grow width 10 px
    bindsym Return mode "default"
    bindsym Escape mode "default"
}
"#;

    #[test]
    fn parse_resize_mode_extracts_six_bindings() {
        let bindings = parse_mode_bindings_from_str(SAMPLE_CONFIG, "resize");
        assert_eq!(bindings.len(), 6);
    }

    #[test]
    fn parse_first_binding_key_and_action() {
        let bindings = parse_mode_bindings_from_str(SAMPLE_CONFIG, "resize");
        assert_eq!(bindings[0].key, "h");
        assert!(bindings[0].action.contains("shrink width"));
    }

    #[test]
    fn parse_mode_bindings_return_mode_entry() {
        let bindings = parse_mode_bindings_from_str(SAMPLE_CONFIG, "resize");
        let ret = bindings.iter().find(|b| b.key == "Return").unwrap();
        assert!(ret.action.contains("default"));
    }

    #[test]
    fn parse_unknown_mode_returns_empty() {
        let bindings = parse_mode_bindings_from_str(SAMPLE_CONFIG, "nonexistent");
        assert!(bindings.is_empty());
    }

    #[test]
    fn parse_empty_config_returns_empty() {
        let bindings = parse_mode_bindings_from_str("", "resize");
        assert!(bindings.is_empty());
    }

    #[test]
    fn parse_ignores_comment_lines() {
        let config = "# bindsym h resize shrink width 10 px\nmode \"test\" {\n    bindsym x test-action\n}\n";
        let bindings = parse_mode_bindings_from_str(config, "test");
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].key, "x");
    }

    #[test]
    fn card_width_constant_pinned() {
        assert!((CARD_WIDTH - 440.0).abs() < f32::EPSILON);
    }
}
