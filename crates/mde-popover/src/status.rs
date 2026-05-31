//! `mde-popover status` — status-zone slide-up strip (Portal-9.b).
//!
//! Full-width layer-shell strip, 180 px tall, anchored to the bottom of
//! the screen above the 56 px Dock shelf.  Three horizontally-navigable
//! cards (tabs) covering Volume, Brightness, and Power.
//!
//! Volume  : reads pactl@DEFAULT_SINK; commits with `pactl set-sink-volume`.
//! Brightness: reads `/sys/class/backlight/*/actual_brightness`; commits via
//!             `brightnessctl set <pct>%`.
//! Power   : Lock / Suspend / Reboot / Shutdown action buttons.
//!
//! Dismiss: Esc key or click-outside area.

#![forbid(unsafe_code)]

use iced::widget::{column, container, row, slider, text, Space};
use iced::widget::mouse_area;
use iced::widget::container::Style as ContainerStyle;
use iced::{Background, Border, Color, Element, Length, Padding, Subscription, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

// ── Design tokens ─────────────────────────────────────────────────────────────

const SURFACE: Color = Color { r: 0.102, g: 0.106, b: 0.118, a: 0.97 };
const FG: Color = Color::WHITE;
const FG_DIM: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.55 };
const FG_MUTED: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.30 };
const INDIGO: Color = Color { r: 0.357, g: 0.416, b: 0.961, a: 1.0 };

// Strip sits 56 px above the bottom edge (Dock shelf height).
const DOCK_HEIGHT: f32 = 56.0;
const STRIP_HEIGHT: f32 = 180.0;

// ── Tabs ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Volume,
    Brightness,
    Power,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Tab::Volume => "Volume",
            Tab::Brightness => "Brightness",
            Tab::Power => "Power",
        }
    }
}

// ── System queries ────────────────────────────────────────────────────────────

fn read_volume_pct() -> u8 {
    let Ok(out) = std::process::Command::new("pactl")
        .args(["get-sink-volume", "@DEFAULT_SINK@"])
        .output()
    else {
        return 50;
    };
    let s = String::from_utf8_lossy(&out.stdout);
    for token in s.split_whitespace() {
        if let Some(stripped) = token.strip_suffix('%') {
            if let Ok(v) = stripped.parse::<u8>() {
                return v.min(100);
            }
        }
    }
    50
}

fn read_brightness_pct() -> u8 {
    let Ok(dir) = std::fs::read_dir("/sys/class/backlight") else {
        return 80;
    };
    for entry in dir.flatten() {
        let base = entry.path();
        let Ok(actual) = std::fs::read_to_string(base.join("actual_brightness")) else {
            continue;
        };
        let Ok(max) = std::fs::read_to_string(base.join("max_brightness")) else {
            continue;
        };
        let actual: u32 = actual.trim().parse().unwrap_or(0);
        let max: u32 = max.trim().parse().unwrap_or(1);
        if max == 0 {
            continue;
        }
        return ((actual as f64 / max as f64) * 100.0).round() as u8;
    }
    80
}

// ── Application ───────────────────────────────────────────────────────────────

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    SwitchTab(Tab),
    VolumeSlider(f32),
    VolumeCommit,
    BrightnessSlider(f32),
    BrightnessCommit,
    Lock,
    Suspend,
    Reboot,
    Shutdown,
    Exit,
}

pub struct App {
    active_tab: Tab,
    volume: f32,       // 0..=100
    brightness: f32,   // 0..=100
}

fn namespace() -> String {
    "mde-popover-status".to_string()
}

fn update(state: &mut App, msg: Message) -> Task<Message> {
    match msg {
        Message::SwitchTab(tab) => {
            state.active_tab = tab;
        }
        Message::VolumeSlider(v) => {
            state.volume = v;
        }
        Message::VolumeCommit => {
            let pct = state.volume.round() as u8;
            tracing::info!(pct, "status: set volume");
            let _ = std::process::Command::new("pactl")
                .args(["set-sink-volume", "@DEFAULT_SINK@", &format!("{pct}%")])
                .spawn();
        }
        Message::BrightnessSlider(v) => {
            state.brightness = v;
        }
        Message::BrightnessCommit => {
            let pct = state.brightness.round() as u8;
            tracing::info!(pct, "status: set brightness");
            let _ = std::process::Command::new("brightnessctl")
                .args(["set", &format!("{pct}%")])
                .spawn();
        }
        Message::Lock => {
            let _ = std::process::Command::new("loginctl")
                .arg("lock-session")
                .spawn();
            std::process::exit(0);
        }
        Message::Suspend => {
            let _ = std::process::Command::new("systemctl")
                .arg("suspend")
                .spawn();
            std::process::exit(0);
        }
        Message::Reboot => {
            let _ = std::process::Command::new("systemctl")
                .arg("reboot")
                .spawn();
            std::process::exit(0);
        }
        Message::Shutdown => {
            let _ = std::process::Command::new("systemctl")
                .arg("poweroff")
                .spawn();
            std::process::exit(0);
        }
        Message::Exit => std::process::exit(0),
        _ => {}
    }
    Task::none()
}

fn view(state: &App) -> Element<'_, Message> {
    // ── Tab bar ───────────────────────────────────────────────────────────
    let tab_bar = row![
        tab_btn(Tab::Volume, state.active_tab),
        tab_btn(Tab::Brightness, state.active_tab),
        tab_btn(Tab::Power, state.active_tab),
    ]
    .spacing(2)
    .padding(Padding { top: 6.0, right: 12.0, bottom: 4.0, left: 12.0 });

    // ── Card content ──────────────────────────────────────────────────────
    let content: Element<'_, Message> = match state.active_tab {
        Tab::Volume => {
            let pct = state.volume.round() as u8;
            column![
                row![
                    text("Volume").size(12).color(FG_DIM),
                    Space::new().width(Length::Fill),
                    text(format!("{pct}%")).size(12).color(FG),
                ]
                .align_y(iced::Alignment::Center),
                slider(0.0..=100.0, state.volume, Message::VolumeSlider)
                    .on_release(Message::VolumeCommit)
                    .step(1.0),
                Space::new().height(Length::Fixed(8.0)),
                row![
                    vol_quick_btn("🔇 Mute", 0.0),
                    Space::new().width(Length::Fill),
                    vol_quick_btn("25%", 25.0),
                    Space::new().width(Length::Fixed(4.0)),
                    vol_quick_btn("50%", 50.0),
                    Space::new().width(Length::Fixed(4.0)),
                    vol_quick_btn("75%", 75.0),
                    Space::new().width(Length::Fixed(4.0)),
                    vol_quick_btn("100%", 100.0),
                ]
                .align_y(iced::Alignment::Center),
            ]
            .spacing(8)
            .padding(Padding { top: 4.0, right: 16.0, bottom: 8.0, left: 16.0 })
            .into()
        }
        Tab::Brightness => {
            let pct = state.brightness.round() as u8;
            column![
                row![
                    text("Brightness").size(12).color(FG_DIM),
                    Space::new().width(Length::Fill),
                    text(format!("{pct}%")).size(12).color(FG),
                ]
                .align_y(iced::Alignment::Center),
                slider(0.0..=100.0, state.brightness, Message::BrightnessSlider)
                    .on_release(Message::BrightnessCommit)
                    .step(1.0),
                Space::new().height(Length::Fixed(8.0)),
                row![
                    bri_quick_btn("10%", 10.0),
                    Space::new().width(Length::Fixed(4.0)),
                    bri_quick_btn("30%", 30.0),
                    Space::new().width(Length::Fixed(4.0)),
                    bri_quick_btn("60%", 60.0),
                    Space::new().width(Length::Fixed(4.0)),
                    bri_quick_btn("80%", 80.0),
                    Space::new().width(Length::Fill),
                    bri_quick_btn("100%", 100.0),
                ]
                .align_y(iced::Alignment::Center),
            ]
            .spacing(8)
            .padding(Padding { top: 4.0, right: 16.0, bottom: 8.0, left: 16.0 })
            .into()
        }
        Tab::Power => {
            row![
                power_btn("Lock", Message::Lock, COLOR_SAGE),
                Space::new().width(Length::Fill),
                power_btn("Suspend", Message::Suspend, FG_DIM),
                Space::new().width(Length::Fixed(8.0)),
                power_btn("Reboot", Message::Reboot, COLOR_AMBER),
                Space::new().width(Length::Fixed(8.0)),
                power_btn("Shutdown", Message::Shutdown, COLOR_RED),
            ]
            .align_y(iced::Alignment::Center)
            .padding(Padding::from([12, 16]))
            .into()
        }
    };

    let strip_card = column![tab_bar, content]
        .width(Length::Fill)
        .height(Length::Fixed(STRIP_HEIGHT));

    let strip: Element<'_, Message> = container(strip_card)
        .width(Length::Fill)
        .height(Length::Fixed(STRIP_HEIGHT))
        .style(|_: &Theme| ContainerStyle {
            background: Some(Background::Color(SURFACE)),
            border: Border {
                color: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.06 },
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into();

    // ── Backdrop dismiss ──────────────────────────────────────────────────
    let backdrop = mouse_area(
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(Message::Exit);

    // Stack: [backdrop fills top] [strip anchored at bottom above Dock]
    container(
        column![
            backdrop,
            container(strip).padding(Padding {
                top: 0.0,
                right: 0.0,
                bottom: DOCK_HEIGHT,
                left: 0.0,
            }),
        ]
        .width(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_: &Theme| ContainerStyle {
        background: Some(Background::Color(Color::TRANSPARENT)),
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

// ── Widget helpers ────────────────────────────────────────────────────────────

const COLOR_SAGE: Color = Color { r: 0.33, g: 0.70, b: 0.54, a: 1.0 };
const COLOR_AMBER: Color = Color { r: 0.95, g: 0.67, b: 0.22, a: 1.0 };
const COLOR_RED: Color = Color { r: 0.86, g: 0.21, b: 0.16, a: 1.0 };

fn tab_btn(tab: Tab, active: Tab) -> Element<'static, Message> {
    let is_active = tab == active;
    let color = if is_active { FG } else { FG_MUTED };
    let bg = if is_active {
        Background::Color(Color { r: 1.0, g: 1.0, b: 1.0, a: 0.08 })
    } else {
        Background::Color(Color::TRANSPARENT)
    };
    mouse_area(
        container(text(tab.label().to_string()).size(11).color(color))
            .padding(Padding::from([4, 10]))
            .style(move |_: &Theme| ContainerStyle {
                background: Some(bg),
                border: Border {
                    color: if is_active { INDIGO } else { Color::TRANSPARENT },
                    width: if is_active { 0.0 } else { 0.0 },
                    radius: 3.0.into(),
                },
                ..Default::default()
            }),
    )
    .on_press(Message::SwitchTab(tab))
    .into()
}

fn vol_quick_btn(label: &'static str, target: f32) -> Element<'static, Message> {
    mouse_area(
        container(text(label.to_string()).size(10).color(FG_DIM))
            .padding(Padding::from([3, 7]))
            .style(|_: &Theme| ContainerStyle {
                background: Some(Background::Color(Color {
                    r: 1.0, g: 1.0, b: 1.0, a: 0.05,
                })),
                border: Border { radius: 3.0.into(), ..Default::default() },
                ..Default::default()
            }),
    )
    .on_press(Message::VolumeSlider(target))
    .into()
}

fn bri_quick_btn(label: &'static str, target: f32) -> Element<'static, Message> {
    mouse_area(
        container(text(label.to_string()).size(10).color(FG_DIM))
            .padding(Padding::from([3, 7]))
            .style(|_: &Theme| ContainerStyle {
                background: Some(Background::Color(Color {
                    r: 1.0, g: 1.0, b: 1.0, a: 0.05,
                })),
                border: Border { radius: 3.0.into(), ..Default::default() },
                ..Default::default()
            }),
    )
    .on_press(Message::BrightnessSlider(target))
    .into()
}

fn power_btn(label: &'static str, msg: Message, color: Color) -> Element<'static, Message> {
    mouse_area(
        container(text(label.to_string()).size(13).color(color))
            .padding(Padding::from([10, 20]))
            .style(|_: &Theme| ContainerStyle {
                background: Some(Background::Color(Color {
                    r: 1.0, g: 1.0, b: 1.0, a: 0.06,
                })),
                border: Border {
                    color: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.12 },
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }),
    )
    .on_press(msg)
    .into()
}

pub fn run() -> iced_layershell::Result {
    iced_layershell::application(
        || {
            let volume = read_volume_pct() as f32;
            let brightness = read_brightness_pct() as f32;
            tracing::info!(volume, brightness, "status popover open");
            App { active_tab: Tab::Volume, volume, brightness }
        },
        namespace,
        update,
        view,
    )
    .theme(|_: &App| iced::Theme::Dark)
    .subscription(subscription)
    .settings(Settings {
        id: Some("mde-popover-status".to_string()),
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_volume_pct_does_not_panic() {
        let v = read_volume_pct();
        assert!(v <= 100, "volume pct must be 0–100, got {v}");
    }

    #[test]
    fn read_brightness_pct_does_not_panic() {
        let b = read_brightness_pct();
        assert!(b <= 100, "brightness pct must be 0–100, got {b}");
    }

    #[test]
    fn tab_label_volume() {
        assert_eq!(Tab::Volume.label(), "Volume");
    }

    #[test]
    fn tab_label_brightness() {
        assert_eq!(Tab::Brightness.label(), "Brightness");
    }

    #[test]
    fn tab_label_power() {
        assert_eq!(Tab::Power.label(), "Power");
    }

    #[test]
    fn strip_height_is_above_dock() {
        assert!(STRIP_HEIGHT > DOCK_HEIGHT, "strip must clear the dock shelf");
    }

    #[test]
    fn default_tab_is_volume() {
        assert_eq!(Tab::default(), Tab::Volume);
    }
}
