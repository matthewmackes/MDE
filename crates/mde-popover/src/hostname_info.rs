//! `mde-popover hostname-info` — node tooltip (Portal-6.c).
//!
//! Fullscreen backdrop layer that positions a small card at the
//! bottom-left corner above the Dock, showing:
//!   • Hostname
//!   • Uptime (d h m)
//!   • Primary IP (first non-loopback address from `hostname -I`)
//!   • Mesh role (static "peer" until GF-6.b wires mackesd D-Bus)
//!
//! Click anywhere outside the card or press Esc to dismiss.

#![forbid(unsafe_code)]

use iced::widget::{column, container, row, text, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;
use iced::widget::mouse_area;
use iced::widget::container::Style as ContainerStyle;

// ── Design tokens (Classic ChromeOS, §3) ─────────────────────────────────────

const CHARCOAL: Color = Color { r: 0.125, g: 0.129, b: 0.141, a: 1.0 };
const FG: Color = Color::WHITE;
const FG_DIM: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.55 };
const FG_LABEL: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.35 };
const CARD_WIDTH: u32 = 248;
const CARD_HEIGHT: u32 = 132;
// Bottom padding — Dock is 56 px; add 4 px gap.
const CARD_BOTTOM: f32 = 60.0;
// Left padding — small inset from screen edge.
const CARD_LEFT: f32 = 8.0;

// ── Data collection ───────────────────────────────────────────────────────────

fn read_hostname() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn read_uptime() -> String {
    let Ok(raw) = std::fs::read_to_string("/proc/uptime") else {
        return "unknown".to_string();
    };
    let secs: f64 = raw
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    let total = secs as u64;
    let d = total / 86_400;
    let h = (total % 86_400) / 3_600;
    let m = (total % 3_600) / 60;
    if d > 0 {
        format!("{d}d {h}h {m}m")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

fn read_primary_ip() -> String {
    // `hostname -I` returns all non-loopback IPs; take the first.
    let out = std::process::Command::new("hostname")
        .arg("-I")
        .output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .split_whitespace()
            .next()
            .unwrap_or("unknown")
            .to_string(),
        Err(_) => "unknown".to_string(),
    }
}

// ── Application ───────────────────────────────────────────────────────────────

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Exit,
}

pub struct App {
    hostname: String,
    uptime: String,
    ip: String,
    mesh_role: String,
}

impl iced_layershell::Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Task<Message>) {
        let hostname = read_hostname();
        let uptime = read_uptime();
        let ip = read_primary_ip();
        let mesh_role = "peer".to_string();
        tracing::info!(hostname = %hostname, ip = %ip, "hostname-info popover open");
        (Self { hostname, uptime, ip, mesh_role }, Task::none())
    }

    fn namespace(&self) -> String {
        "mde-popover-hostname-info".to_string()
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Exit => std::process::exit(0),
            _ => Task::none(),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let row_item = |label: &str, value: &str| -> Element<'static, Message> {
            row![
                text(label.to_string())
                    .size(10)
                    .color(FG_LABEL)
                    .width(Length::Fixed(72.0)),
                text(value.to_string()).size(11).color(FG),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(4)
            .into()
        };

        let card_body = column![
            text(&self.hostname).size(15).color(FG),
            Space::with_height(Length::Fixed(10.0)),
            row_item("uptime", &self.uptime),
            Space::with_height(Length::Fixed(4.0)),
            row_item("ip", &self.ip),
            Space::with_height(Length::Fixed(4.0)),
            row_item("mesh", &self.mesh_role),
            Space::with_height(Length::Fill),
            text("Esc or click outside to close")
                .size(9)
                .color(FG_DIM),
        ]
        .padding(Padding { top: 14.0, right: 16.0, bottom: 10.0, left: 16.0 })
        .spacing(0);

        let card: Element<'_, Message> = container(card_body)
            .width(Length::Fixed(CARD_WIDTH as f32))
            .height(Length::Fixed(CARD_HEIGHT as f32))
            .style(|_: &Theme| ContainerStyle {
                background: Some(Background::Color(CHARCOAL)),
                border: Border {
                    color: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.08 },
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            })
            .into();

        let dismiss = || {
            mouse_area(
                container(Space::with_width(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .on_press(Message::Exit)
        };

        // Bottom strip: [card at left] [dismiss fills right]
        let bottom_strip = row![
            container(card).padding(Padding {
                top: 0.0,
                right: 0.0,
                bottom: CARD_BOTTOM,
                left: CARD_LEFT,
            }),
            dismiss(),
        ]
        .height(Length::Fixed((CARD_HEIGHT as f32) + CARD_BOTTOM));

        container(column![dismiss(), bottom_strip])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_: &Theme| ContainerStyle {
                background: Some(Background::Color(Color::TRANSPARENT)),
                ..Default::default()
            })
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::keyboard::on_key_press(|key, _| {
            use iced::keyboard::{key::Named, Key};
            if matches!(key, Key::Named(Named::Escape)) {
                Some(Message::Exit)
            } else {
                None
            }
        })
    }
}

pub fn run() -> iced_layershell::Result {
    <App as iced_layershell::Application>::run(Settings {
        id: Some("mde-popover-hostname-info".to_string()),
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
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_hostname_returns_non_empty_or_empty_string() {
        let h = read_hostname();
        // Either a real hostname or empty string — never panics.
        assert!(h.len() < 256, "hostname should be short: {h}");
    }

    #[test]
    fn read_uptime_returns_formatted_string() {
        let u = read_uptime();
        // Should contain 'm' (minutes) or 'd' (days) or 'h' (hours).
        assert!(!u.is_empty());
        assert!(
            u.contains('m') || u.contains('h') || u.contains('d') || u == "unknown",
            "uptime format unexpected: {u}"
        );
    }

    #[test]
    fn read_primary_ip_does_not_panic() {
        let ip = read_primary_ip();
        assert!(!ip.is_empty());
    }

    #[test]
    fn card_dimensions_are_positive() {
        assert!(CARD_WIDTH > 0);
        assert!(CARD_HEIGHT > 0);
    }

    #[test]
    fn charcoal_matches_chromeos_design_lock() {
        let r = (CHARCOAL.r * 255.0).round() as u8;
        let g = (CHARCOAL.g * 255.0).round() as u8;
        let b = (CHARCOAL.b * 255.0).round() as u8;
        assert_eq!((r, g, b), (32, 33, 36), "#202124 charcoal lock");
    }
}
