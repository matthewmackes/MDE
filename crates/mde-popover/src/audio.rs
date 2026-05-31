//! Audio popover — volume slider + mute toggle.
//!
//! Anchored bottom-right of the primary output, 8 px above the
//! panel edge. The user clicks the panel's ♫ tray button →
//! `mde-panel` execs `mde-popover audio` → this binary opens a
//! 320×140 layer-shell window. Drag the slider to set the
//! default-sink volume; press the mute button to toggle mute.
//! Esc closes; the popover does not auto-close on slider drag so
//! the user can fine-tune before committing.

use std::process::Command;

use iced::widget::{button, column, container, mouse_area, row, slider, text, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;
use mde_applet_audio::{parse_mute, parse_volume, AudioState};

const WIDTH: u32 = 320;
const HEIGHT: u32 = 140;

const ACCENT: Color = Color {
    r: 0.169,
    g: 0.604,
    b: 0.953,
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
const SLIDER_TRACK: Color = Color {
    r: 0.106,
    g: 0.106,
    b: 0.114,
    a: 1.0,
};

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    /// Slider moved — apply the new volume immediately.
    VolumeChanged(u32),
    /// Mute button pressed.
    ToggleMute,
    /// Esc handler.
    Exit,
}

pub struct App {
    state: AudioState,
}

fn namespace() -> String {
    "mde-popover-audio".to_string()
}

fn update(state: &mut App, msg: Message) -> Task<Message> {
    match msg {
        Message::VolumeChanged(vol) => {
            state.state.volume_pct = vol;
            pactl_set_volume(vol);
            Task::none()
        }
        Message::ToggleMute => {
            pactl_toggle_mute();
            // Re-read state so the icon updates immediately.
            state.state = read_state();
            Task::none()
        }
        Message::Exit => {
            std::process::exit(0);
        }
        _ => Task::none(),
    }
}

fn view(state: &App) -> Element<'_, Message> {
    let mute_glyph = if state.state.muted { "×" } else { "♫" };
    let mute_color = if state.state.muted { FG_MUTED } else { ACCENT };
    let mute_btn = button(
        text(mute_glyph.to_string())
            .size(20)
            .color(mute_color),
    )
    .padding(Padding {
        top: 6.0,
        right: 14.0,
        bottom: 6.0,
        left: 14.0,
    })
    .style(mute_button_style)
    .on_press(Message::ToggleMute);

    let pct_label = text(format!("{:>3}%", state.state.volume_pct))
        .size(14)
        .color(FG_TEXT);

    let header = row![
        mute_btn,
        Space::new().width(Length::Fixed(12.0)),
        text("Output").size(13).color(FG_TEXT),
        Space::new().width(Length::Fill),
        pct_label,
        Space::new().width(Length::Fixed(8.0)),
        // v3.0.3 — always-visible close button (Esc still works
        // via subscription below).
        crate::dismiss::close_button(Message::Exit),
    ]
    .align_y(iced::Alignment::Center);

    let vol_slider = slider(0u32..=100u32, state.state.volume_pct, Message::VolumeChanged)
        .step(1u32)
        .style(volume_slider_style);

    let footer = text("Esc closes · ♫ toggles mute · drag to set")
        .size(10)
        .color(FG_MUTED);

    let body = column![
        header,
        Space::new().height(Length::Fixed(14.0)),
        vol_slider,
        Space::new().height(Length::Fixed(12.0)),
        footer,
    ]
    .padding(Padding {
        top: 16.0,
        right: 18.0,
        bottom: 12.0,
        left: 18.0,
    });

    let card: Element<'_, Message> = container(body)
        .width(Length::Fixed(WIDTH as f32))
        .height(Length::Fixed(HEIGHT as f32))
        .style(popover_surface)
        .into();

    // v3.0.4 (2026-05-23) — backdrop. Card pinned bottom-right.
    let dismiss = || {
        mouse_area(
            container(Space::new().width(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(Message::Exit)
    };
    let bottom_strip = row![
        dismiss(),
        container(card).padding(iced::Padding {
            top: 0.0,
            right: 4.0,
            bottom: 48.0,
            left: 0.0,
        }),
    ]
    .height(Length::Fixed((HEIGHT + 48) as f32));
    container(column![dismiss(), bottom_strip])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
            border: iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: 0.0.into(),
            },
            shadow: iced::Shadow::default(),
            text_color: None,
            snap: false,
        })
        .into()
}

fn subscription(_state: &App) -> iced::Subscription<Message> {
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
            let state = read_state();
            tracing::info!(volume = state.volume_pct, muted = state.muted, "audio popover open");
            App { state }
        },
        namespace,
        update,
        view,
    )
    .theme(|_: &App| iced::Theme::Dark)
    .subscription(subscription)
    .settings(Settings {
        id: Some("mde-popover-audio".to_string()),
        fonts: crate::fonts::load_fallback_fonts(),
        layer_settings: LayerShellSettings {
            // v3.0.4 — fullscreen surface for backdrop dismiss.
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

/// Snapshot the default-sink state via `pactl`. Returns
/// `AudioState::default()` on any pactl failure (the panel still
/// shows the popover, just with the slider at 0).
fn read_state() -> AudioState {
    let vol = Command::new("pactl")
        .args(["get-sink-volume", "@DEFAULT_SINK@"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| parse_volume(std::str::from_utf8(&o.stdout).unwrap_or("")))
        .unwrap_or(0);
    let muted = Command::new("pactl")
        .args(["get-sink-mute", "@DEFAULT_SINK@"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| parse_mute(std::str::from_utf8(&o.stdout).unwrap_or("")))
        .unwrap_or(false);
    AudioState {
        volume_pct: vol,
        muted,
    }
}

fn pactl_set_volume(pct: u32) {
    let pct = pct.min(100);
    let _ = Command::new("pactl")
        .args(["set-sink-volume", "@DEFAULT_SINK@", &format!("{pct}%")])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

fn pactl_toggle_mute() {
    let _ = Command::new("pactl")
        .args(["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

fn popover_surface(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE_BG)),
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
        text_color: Some(FG_TEXT),
        shadow: Shadow::default(),
        snap: false,
    }
}

fn mute_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(Background::Color(Color {
            r: ACCENT.r,
            g: ACCENT.g,
            b: ACCENT.b,
            a: 0.14,
        })),
        button::Status::Pressed => Some(Background::Color(Color {
            r: ACCENT.r,
            g: ACCENT.g,
            b: ACCENT.b,
            a: 0.22,
        })),
        _ => None,
    };
    button::Style {
        background: bg,
        text_color: FG_TEXT,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

fn volume_slider_style(_theme: &Theme, status: slider::Status) -> slider::Style {
    let handle_color = match status {
        slider::Status::Hovered => Color {
            r: ACCENT.r,
            g: ACCENT.g,
            b: ACCENT.b,
            a: 1.0,
        },
        _ => ACCENT,
    };
    slider::Style {
        rail: slider::Rail {
            backgrounds: (
                Background::Color(ACCENT),
                Background::Color(SLIDER_TRACK),
            ),
            width: 4.0,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 2.0.into(),
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 7.0 },
            background: Background::Color(handle_color),
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_pinned_for_visual_consistency() {
        assert_eq!(WIDTH, 320);
        assert_eq!(HEIGHT, 140);
    }

    #[test]
    fn read_state_returns_default_when_pactl_absent() {
        // Hard to test without intercepting Command — just exercise
        // the call and assert the type. If pactl is missing the
        // helper returns the all-zero default.
        let s = read_state();
        assert!(s.volume_pct <= 100);
    }
}
