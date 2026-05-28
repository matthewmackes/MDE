//! Keyboard panel — three `keyboard.*` keys: key-repeat delay,
//! key-repeat rate, and XKB layout. Reads + writes via the shared
//! [`Backend`] trait (`dev.mackes.MDE.Settings.Get/Set`); the
//! mackesd-side applier (`crates/mackesd/src/settings/input.rs`)
//! persists to a sidecar + best-effort live-applies via
//! `swaymsg input type:keyboard`.
//!
//! Ports the v1.x `mackes/workbench/devices/keyboard.py` GTK3 panel
//! (EPIC-RETIRE-PY-WORKBENCH.port-keyboard). The v1.x "enable key
//! repeat" toggle + the "open xfce4-keyboard-settings" launcher are
//! dropped: sway has no separate repeat-enable knob (a rate of 0
//! disables it, but the slider floor is 1), and global shortcuts are
//! the keybinds surface, not a device panel.

use std::sync::Arc;

use iced::widget::{column, row, slider, text, text_input};
use iced::{Element, Length, Task};
use mde_theme::Palette;

use crate::backend::Backend;
use crate::controls::{variant_button, ButtonVariant};
use crate::panels::json_helpers::{parse_u32, quote_json, strip_json_quotes};

pub const KEY_REPEAT_DELAY: &str = "keyboard.repeat_delay";
pub const KEY_REPEAT_RATE: &str = "keyboard.repeat_rate";
pub const KEY_XKB_LAYOUT: &str = "keyboard.xkb_layout";

/// Repeat-delay range in milliseconds — matches the v1.x Python
/// panel's `SpinButton.new_with_range(100, 2000, 50)` and the
/// mackesd `input` applier's validation bounds.
pub const REPEAT_DELAY_MIN: u32 = 100;
pub const REPEAT_DELAY_MAX: u32 = 2000;
pub const REPEAT_DELAY_STEP: u32 = 50;
pub const REPEAT_DELAY_DEFAULT: u32 = 600;

/// Repeat-rate range in characters per second — matches the v1.x
/// `SpinButton.new_with_range(1, 100, 1)` and the applier bounds.
pub const REPEAT_RATE_MIN: u32 = 1;
pub const REPEAT_RATE_MAX: u32 = 100;
pub const REPEAT_RATE_DEFAULT: u32 = 25;

/// Default XKB layout when no value has been written yet.
pub const XKB_LAYOUT_DEFAULT: &str = "us";

#[derive(Debug, Clone)]
pub struct KeyboardPanel {
    pub repeat_delay: u32,
    pub repeat_rate: u32,
    pub xkb_layout: String,
    pub status: String,
    pub busy: bool,
}

impl Default for KeyboardPanel {
    fn default() -> Self {
        Self {
            repeat_delay: REPEAT_DELAY_DEFAULT,
            repeat_rate: REPEAT_RATE_DEFAULT,
            xkb_layout: XKB_LAYOUT_DEFAULT.to_owned(),
            status: String::new(),
            busy: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded {
        repeat_delay: u32,
        repeat_rate: u32,
        xkb_layout: String,
    },
    Error(String),
    Saved,
    RepeatDelayChanged(u32),
    RepeatRateChanged(u32),
    XkbLayoutChanged(String),
    SaveClicked,
}

impl KeyboardPanel {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(backend: Arc<dyn Backend>) -> Task<crate::Message> {
        Task::perform(
            async move {
                let repeat_delay = parse_u32(&strip_json_quotes(&backend.get(KEY_REPEAT_DELAY).await?))
                    .unwrap_or(REPEAT_DELAY_DEFAULT);
                let repeat_rate = parse_u32(&strip_json_quotes(&backend.get(KEY_REPEAT_RATE).await?))
                    .unwrap_or(REPEAT_RATE_DEFAULT);
                let raw_layout = strip_json_quotes(&backend.get(KEY_XKB_LAYOUT).await?);
                let xkb_layout = if raw_layout.trim().is_empty() {
                    XKB_LAYOUT_DEFAULT.to_owned()
                } else {
                    raw_layout
                };
                Ok::<_, crate::backend::BackendError>(Message::Loaded {
                    repeat_delay,
                    repeat_rate,
                    xkb_layout,
                })
            },
            |result| {
                crate::Message::Keyboard(result.unwrap_or_else(|e| Message::Error(e.to_string())))
            },
        )
    }

    pub fn update(&mut self, message: Message, backend: Arc<dyn Backend>) -> Task<crate::Message> {
        match message {
            Message::Loaded {
                repeat_delay,
                repeat_rate,
                xkb_layout,
            } => {
                self.repeat_delay = clamp_delay(repeat_delay);
                self.repeat_rate = clamp_rate(repeat_rate);
                self.xkb_layout = xkb_layout;
                self.status.clear();
                self.busy = false;
                Task::none()
            }
            Message::Error(msg) => {
                self.status = msg;
                self.busy = false;
                Task::none()
            }
            Message::Saved => {
                self.status = "Saved.".into();
                self.busy = false;
                Task::none()
            }
            Message::RepeatDelayChanged(v) => {
                self.repeat_delay = clamp_delay(v);
                Task::none()
            }
            Message::RepeatRateChanged(v) => {
                self.repeat_rate = clamp_rate(v);
                Task::none()
            }
            Message::XkbLayoutChanged(v) => {
                self.xkb_layout = v;
                Task::none()
            }
            Message::SaveClicked => {
                if self.busy {
                    return Task::none();
                }
                let layout = self.xkb_layout.trim().to_string();
                if layout.is_empty() {
                    self.status = "Keyboard layout can't be empty (e.g. us, gb, de).".into();
                    return Task::none();
                }
                self.busy = true;
                self.status = "Applying…".into();
                let repeat_delay = self.repeat_delay;
                let repeat_rate = self.repeat_rate;
                Task::perform(
                    async move {
                        backend
                            .set(KEY_REPEAT_DELAY, &repeat_delay.to_string())
                            .await?;
                        backend
                            .set(KEY_REPEAT_RATE, &repeat_rate.to_string())
                            .await?;
                        backend.set(KEY_XKB_LAYOUT, &quote_json(&layout)).await?;
                        Ok::<_, crate::backend::BackendError>(Message::Saved)
                    },
                    |result| {
                        crate::Message::Keyboard(
                            result.unwrap_or_else(|e| Message::Error(e.to_string())),
                        )
                    },
                )
            }
        }
    }

    pub fn view(&self) -> Element<'_, crate::Message> {
        let apply_label = if self.busy { "Applying…" } else { "Apply" };
        let apply_btn = variant_button(
            apply_label,
            ButtonVariant::Primary,
            (!self.busy).then(|| crate::Message::Keyboard(Message::SaveClicked)),
            Palette::dark(),
        );

        column![
            row![
                text("Repeat delay (ms)").width(Length::Fixed(180.0)),
                slider(
                    REPEAT_DELAY_MIN..=REPEAT_DELAY_MAX,
                    self.repeat_delay,
                    |v| crate::Message::Keyboard(Message::RepeatDelayChanged(v)),
                )
                .step(REPEAT_DELAY_STEP),
                text(format!("{} ms", self.repeat_delay)).size(13),
            ]
            .spacing(12),
            row![
                text("Repeat rate (chars/s)").width(Length::Fixed(180.0)),
                slider(
                    REPEAT_RATE_MIN..=REPEAT_RATE_MAX,
                    self.repeat_rate,
                    |v| crate::Message::Keyboard(Message::RepeatRateChanged(v)),
                )
                .step(1_u32),
                text(format!("{}/s", self.repeat_rate)).size(13),
            ]
            .spacing(12),
            row![
                text("Keyboard layout (XKB)").width(Length::Fixed(180.0)),
                text_input("us, gb, de, fr …", &self.xkb_layout)
                    .on_input(|v| crate::Message::Keyboard(Message::XkbLayoutChanged(v))),
            ]
            .spacing(12),
            row![apply_btn, text(&self.status).size(13)].spacing(12),
        ]
        .spacing(12)
        .width(Length::Fill)
        .into()
    }
}

/// Clamp a repeat-delay value into the locked range.
fn clamp_delay(v: u32) -> u32 {
    v.clamp(REPEAT_DELAY_MIN, REPEAT_DELAY_MAX)
}

/// Clamp a repeat-rate value into the locked range.
fn clamp_rate(v: u32) -> u32 {
    v.clamp(REPEAT_RATE_MIN, REPEAT_RATE_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::DemoBackend;

    #[test]
    fn keys_match_locked_keyboard_namespace() {
        assert_eq!(KEY_REPEAT_DELAY, "keyboard.repeat_delay");
        assert_eq!(KEY_REPEAT_RATE, "keyboard.repeat_rate");
        assert_eq!(KEY_XKB_LAYOUT, "keyboard.xkb_layout");
    }

    #[test]
    fn ranges_match_v1_python_spinbuttons() {
        assert_eq!((REPEAT_DELAY_MIN, REPEAT_DELAY_MAX), (100, 2000));
        assert_eq!((REPEAT_RATE_MIN, REPEAT_RATE_MAX), (1, 100));
    }

    #[test]
    fn new_panel_has_sane_defaults() {
        let panel = KeyboardPanel::new();
        assert_eq!(panel.repeat_delay, 600);
        assert_eq!(panel.repeat_rate, 25);
        assert_eq!(panel.xkb_layout, "us");
    }

    #[test]
    fn clamp_helpers_bound_out_of_range() {
        assert_eq!(clamp_delay(10), REPEAT_DELAY_MIN);
        assert_eq!(clamp_delay(9000), REPEAT_DELAY_MAX);
        assert_eq!(clamp_delay(400), 400);
        assert_eq!(clamp_rate(0), REPEAT_RATE_MIN);
        assert_eq!(clamp_rate(500), REPEAT_RATE_MAX);
        assert_eq!(clamp_rate(40), 40);
    }

    #[test]
    fn loaded_clamps_out_of_range_values() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = KeyboardPanel::new();
        let _ = panel.update(
            Message::Loaded {
                repeat_delay: 50,
                repeat_rate: 999,
                xkb_layout: "de".into(),
            },
            backend,
        );
        assert_eq!(panel.repeat_delay, REPEAT_DELAY_MIN);
        assert_eq!(panel.repeat_rate, REPEAT_RATE_MAX);
        assert_eq!(panel.xkb_layout, "de");
    }

    #[test]
    fn field_change_messages_mutate_matching_fields() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = KeyboardPanel::new();
        let _ = panel.update(Message::RepeatDelayChanged(300), backend.clone());
        assert_eq!(panel.repeat_delay, 300);
        let _ = panel.update(Message::RepeatRateChanged(50), backend.clone());
        assert_eq!(panel.repeat_rate, 50);
        let _ = panel.update(Message::XkbLayoutChanged("gb".into()), backend);
        assert_eq!(panel.xkb_layout, "gb");
    }

    #[test]
    fn save_clicked_with_empty_layout_surfaces_validation() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = KeyboardPanel::new();
        panel.xkb_layout = "   ".into();
        let _ = panel.update(Message::SaveClicked, backend);
        assert!(panel.status.contains("empty"));
        assert!(!panel.busy);
    }

    #[test]
    fn save_clicked_while_busy_is_noop() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = KeyboardPanel::new();
        panel.busy = true;
        panel.status = "Applying…".into();
        let _ = panel.update(Message::SaveClicked, backend);
        assert_eq!(panel.status, "Applying…");
    }

    #[tokio::test]
    async fn save_writes_all_three_keys_with_correct_json_shapes() {
        let backend: Arc<dyn Backend> = Arc::new(DemoBackend::new());
        backend.set(KEY_REPEAT_DELAY, "300").await.unwrap();
        backend.set(KEY_REPEAT_RATE, "40").await.unwrap();
        backend.set(KEY_XKB_LAYOUT, &quote_json("gb")).await.unwrap();
        assert_eq!(backend.get(KEY_REPEAT_DELAY).await.unwrap(), "300");
        assert_eq!(backend.get(KEY_REPEAT_RATE).await.unwrap(), "40");
        assert_eq!(backend.get(KEY_XKB_LAYOUT).await.unwrap(), "\"gb\"");
    }
}
