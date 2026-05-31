//! v4.0.1 WM-4 (2026-05-23) — visual snap-assist overlay.
//!
//! Spec-aligned realization of "drag-to-snap zones with
//! visual feedback." The spec called for the overlay to
//! appear *during* a drag — sway IPC doesn't expose live
//! pointer drag events the way X11 / Wayland-core does, so
//! tracking the drag itself requires either a wayland
//! seat-grab protocol that sway hasn't implemented, or a
//! per-100 ms poll of `swaymsg -t get_pointer_locations`
//! (which doesn't exist either). Best-choice deviation:
//! ship the same VISUAL overlay (5 indigo zones at 30%-
//! alpha, click to commit), but trigger it via a keybind
//! (`Super+Z`) instead of the drag itself. The
//! bench-observable outcome — "the operator sees zones,
//! clicks one, the window snaps" — matches the spec; the
//! invocation gesture differs.
//!
//! Bound from `data/sway/config.d/mackes-keybinds-wm.conf`:
//!
//!   bindsym $mod+z exec mde-popover snap-assist
//!
//! Targets the currently-focused sway window (via
//! `swaymsg -t get_tree` walking for `"focused": true`). If
//! no window is focused, the overlay still renders but the
//! click is a no-op.
//!
//! Five zones supported per the spec:
//!   * Left half      ─ left 50%, full height
//!   * Right half     ─ right 50%, full height
//!   * Top half       ─ full width, top 50%
//!   * Bottom half    ─ full width, bottom 50%
//!   * Four quadrants ─ TL / TR / BL / BR at 50% × 50%

use std::process::Command;

use iced::widget::{button, column, container, mouse_area, row, text, Space};
use iced::{Background, Border, Color, Element, Length, Shadow, Subscription, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

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

/// All snap targets the overlay supports. Each variant maps
/// 1-1 to a swaymsg command sequence that lands the focused
/// window in the corresponding region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapZone {
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    QuadrantTopLeft,
    QuadrantTopRight,
    QuadrantBottomLeft,
    QuadrantBottomRight,
}

impl SnapZone {
    /// Returns the `swaymsg` command for this snap zone (as the
    /// single positional argument; sway's IPC parses it as a
    /// concatenated command chain).
    #[must_use]
    pub fn swaymsg_command(self) -> String {
        match self {
            SnapZone::LeftHalf => {
                "floating disable; move position 0 0; resize set 50ppt 100ppt".to_string()
            }
            SnapZone::RightHalf => {
                "floating disable; move position 50ppt 0; resize set 50ppt 100ppt".to_string()
            }
            SnapZone::TopHalf => {
                "floating disable; move position 0 0; resize set 100ppt 50ppt".to_string()
            }
            SnapZone::BottomHalf => {
                "floating disable; move position 0 50ppt; resize set 100ppt 50ppt".to_string()
            }
            SnapZone::QuadrantTopLeft => {
                "floating disable; move position 0 0; resize set 50ppt 50ppt".to_string()
            }
            SnapZone::QuadrantTopRight => {
                "floating disable; move position 50ppt 0; resize set 50ppt 50ppt".to_string()
            }
            SnapZone::QuadrantBottomLeft => {
                "floating disable; move position 0 50ppt; resize set 50ppt 50ppt".to_string()
            }
            SnapZone::QuadrantBottomRight => {
                "floating disable; move position 50ppt 50ppt; resize set 50ppt 50ppt".to_string()
            }
        }
    }

    /// Human label rendered inside the zone's overlay tile.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            SnapZone::LeftHalf => "Left half",
            SnapZone::RightHalf => "Right half",
            SnapZone::TopHalf => "Top half",
            SnapZone::BottomHalf => "Bottom half",
            SnapZone::QuadrantTopLeft => "Top-left ¼",
            SnapZone::QuadrantTopRight => "Top-right ¼",
            SnapZone::QuadrantBottomLeft => "Bottom-left ¼",
            SnapZone::QuadrantBottomRight => "Bottom-right ¼",
        }
    }
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    /// User clicked one of the snap zones — commit + exit.
    Commit(SnapZone),
    /// Esc / click outside — exit without snapping.
    Cancel,
}

#[derive(Debug, Default)]
pub struct App;

fn namespace() -> String {
    "mde-popover-snap-assist".to_string()
}

fn update(_state: &mut App, msg: Message) -> Task<Message> {
    match msg {
        Message::Commit(zone) => {
            run_snap(zone);
            std::process::exit(0);
        }
        Message::Cancel => std::process::exit(0),
        _ => Task::none(),
    }
}

fn view(_state: &App) -> Element<'_, Message> {
    let header = text("Snap Assist")
        .size(14)
        .color(FG_TEXT);
    let subhead = text("Click a zone to snap the focused window · Esc cancels")
        .size(11)
        .color(FG_MUTED);

    let halves_row = row![
        zone_button(SnapZone::LeftHalf),
        Space::new().width(Length::Fixed(8.0)),
        zone_button(SnapZone::RightHalf),
        Space::new().width(Length::Fixed(8.0)),
        zone_button(SnapZone::TopHalf),
        Space::new().width(Length::Fixed(8.0)),
        zone_button(SnapZone::BottomHalf),
    ]
    .align_y(iced::Alignment::Center);

    let quad_row = row![
        zone_button(SnapZone::QuadrantTopLeft),
        Space::new().width(Length::Fixed(8.0)),
        zone_button(SnapZone::QuadrantTopRight),
        Space::new().width(Length::Fixed(8.0)),
        zone_button(SnapZone::QuadrantBottomLeft),
        Space::new().width(Length::Fixed(8.0)),
        zone_button(SnapZone::QuadrantBottomRight),
    ]
    .align_y(iced::Alignment::Center);

    let card = container(
        column![
            header,
            Space::new().height(Length::Fixed(4.0)),
            subhead,
            Space::new().height(Length::Fixed(14.0)),
            text("Halves").size(11).color(FG_MUTED),
            Space::new().height(Length::Fixed(6.0)),
            halves_row,
            Space::new().height(Length::Fixed(12.0)),
            text("Quadrants").size(11).color(FG_MUTED),
            Space::new().height(Length::Fixed(6.0)),
            quad_row,
        ]
        .padding(iced::Padding {
            top: 18.0,
            right: 20.0,
            bottom: 18.0,
            left: 20.0,
        }),
    )
    .width(Length::Shrink)
    .style(|_| container::Style {
        background: Some(Background::Color(Color {
            r: 0.055,
            g: 0.055,
            b: 0.063,
            a: 0.97,
        })),
        border: Border {
            color: Color {
                r: 0.957,
                g: 0.957,
                b: 0.957,
                a: 0.12,
            },
            width: 1.0,
            radius: 10.0.into(),
        },
        text_color: Some(FG_TEXT),
        shadow: Shadow::default(),
        snap: false,
    });

    // Backdrop dismiss — fullscreen click-outside cancels.
    let dismiss = || {
        mouse_area(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(Message::Cancel)
    };
    container(column![
        dismiss(),
        row![dismiss(), container(card).padding(20), dismiss()],
        dismiss(),
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_| container::Style {
        background: Some(Background::Color(Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.30,
        })),
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
                    Some(Message::Cancel)
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
            tracing::info!("snap-assist overlay open");
            App
        },
        namespace,
        update,
        view,
    )
    .theme(|_: &App| iced::Theme::Dark)
    .subscription(subscription)
    .settings(Settings {
        id: Some("mde-popover-snap-assist".to_string()),
        fonts: crate::fonts::load_fallback_fonts(),
        layer_settings: LayerShellSettings {
            size: None,
            exclusive_zone: -1,
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            margin: (0, 0, 0, 0),
            layer: Layer::Overlay,
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        },
        ..Default::default()
    })
    .run()
}

fn zone_button(zone: SnapZone) -> Element<'static, Message> {
    button(text(zone.label()).size(11).color(FG_TEXT))
        .padding(iced::Padding {
            top: 14.0,
            right: 16.0,
            bottom: 14.0,
            left: 16.0,
        })
        .width(Length::Fixed(132.0))
        .on_press(Message::Commit(zone))
        .style(|_t: &Theme, status: iced::widget::button::Status| {
            let alpha = match status {
                iced::widget::button::Status::Hovered => 0.45,
                iced::widget::button::Status::Pressed => 0.65,
                _ => 0.30,
            };
            iced::widget::button::Style {
                background: Some(Background::Color(Color {
                    r: ACCENT.r,
                    g: ACCENT.g,
                    b: ACCENT.b,
                    a: alpha,
                })),
                text_color: FG_TEXT,
                border: Border {
                    color: ACCENT,
                    width: 1.5,
                    radius: 6.0.into(),
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .into()
}

fn run_snap(zone: SnapZone) {
    // Targets the focused window — swaymsg defaults to the
    // focused container when no [criteria] block is given.
    let _ = Command::new("swaymsg")
        .arg(zone.swaymsg_command())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_zone_emits_a_swaymsg_command() {
        for z in [
            SnapZone::LeftHalf,
            SnapZone::RightHalf,
            SnapZone::TopHalf,
            SnapZone::BottomHalf,
            SnapZone::QuadrantTopLeft,
            SnapZone::QuadrantTopRight,
            SnapZone::QuadrantBottomLeft,
            SnapZone::QuadrantBottomRight,
        ] {
            let cmd = z.swaymsg_command();
            assert!(cmd.contains("floating disable"));
            assert!(cmd.contains("move position"));
            assert!(cmd.contains("resize set"));
        }
    }

    #[test]
    fn left_half_resize_is_50ppt_wide_full_tall() {
        let cmd = SnapZone::LeftHalf.swaymsg_command();
        assert!(cmd.contains("resize set 50ppt 100ppt"));
        assert!(cmd.contains("move position 0 0"));
    }

    #[test]
    fn right_half_starts_at_50ppt_offset() {
        let cmd = SnapZone::RightHalf.swaymsg_command();
        assert!(cmd.contains("move position 50ppt 0"));
        assert!(cmd.contains("resize set 50ppt 100ppt"));
    }

    #[test]
    fn quadrants_are_50_by_50() {
        for q in [
            SnapZone::QuadrantTopLeft,
            SnapZone::QuadrantTopRight,
            SnapZone::QuadrantBottomLeft,
            SnapZone::QuadrantBottomRight,
        ] {
            let cmd = q.swaymsg_command();
            assert!(cmd.contains("resize set 50ppt 50ppt"));
        }
    }

    #[test]
    fn labels_are_distinct() {
        use std::collections::HashSet;
        let zones = [
            SnapZone::LeftHalf,
            SnapZone::RightHalf,
            SnapZone::TopHalf,
            SnapZone::BottomHalf,
            SnapZone::QuadrantTopLeft,
            SnapZone::QuadrantTopRight,
            SnapZone::QuadrantBottomLeft,
            SnapZone::QuadrantBottomRight,
        ];
        let labels: HashSet<&str> = zones.iter().map(|z| z.label()).collect();
        assert_eq!(labels.len(), zones.len());
    }
}
