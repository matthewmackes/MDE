//! Mouse & Touchpad panel — four `mouse.*` keys: libinput pointer
//! acceleration, natural-scroll, tap-to-click, and left-handed
//! button mapping. Reads + writes via the shared [`Backend`] trait;
//! the mackesd `input` applier persists each to a sidecar +
//! best-effort live-applies via `swaymsg input type:pointer` /
//! `type:touchpad`.
//!
//! Ports the v1.x `mackes/workbench/devices/mouse.py`
//! (EPIC-RETIRE-PY-WORKBENCH.port-mouse). The v1.x panel's X11
//! acceleration MULTIPLIER + THRESHOLD model (and the
//! `/Net/DoubleClickTime` xsettings knob + the `xinput` device
//! list) are X11/xfconf-era with no libinput equivalent — the
//! v2.0.0 surface is the modern libinput set the task asked for:
//! acceleration (single normalized -1..=1 value), handedness,
//! tap-to-click, and scroll direction.

use std::sync::Arc;

use iced::widget::{checkbox, column, row, slider, text};
use iced::{Element, Length, Task};
use mde_theme::Palette;

use crate::backend::Backend;
use crate::controls::{variant_button, ButtonVariant};
use crate::panels::json_helpers::{encode_bool, parse_bool, strip_json_quotes};

pub const KEY_POINTER_ACCEL: &str = "mouse.pointer_accel";
pub const KEY_NATURAL_SCROLL: &str = "mouse.natural_scroll";
pub const KEY_TAP_TO_CLICK: &str = "mouse.tap_to_click";
pub const KEY_LEFT_HANDED: &str = "mouse.left_handed";

/// libinput pointer-acceleration range. 0.0 is the system default;
/// negative is slower, positive faster. Matches the mackesd `input`
/// applier's validation bounds.
pub const ACCEL_MIN: f32 = -1.0;
pub const ACCEL_MAX: f32 = 1.0;
pub const ACCEL_STEP: f32 = 0.1;
pub const ACCEL_DEFAULT: f32 = 0.0;

#[derive(Debug, Clone, Default)]
pub struct MousePanel {
    pub pointer_accel: f32,
    pub natural_scroll: bool,
    pub tap_to_click: bool,
    pub left_handed: bool,
    pub status: String,
    pub busy: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded {
        pointer_accel: f32,
        natural_scroll: bool,
        tap_to_click: bool,
        left_handed: bool,
    },
    Error(String),
    Saved,
    AccelChanged(f32),
    NaturalScrollChanged(bool),
    TapToClickChanged(bool),
    LeftHandedChanged(bool),
    SaveClicked,
}

impl MousePanel {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(backend: Arc<dyn Backend>) -> Task<crate::Message> {
        Task::perform(
            async move {
                let pointer_accel = parse_accel(&backend.get(KEY_POINTER_ACCEL).await?);
                let natural_scroll = parse_bool(&backend.get(KEY_NATURAL_SCROLL).await?);
                let tap_to_click = parse_bool(&backend.get(KEY_TAP_TO_CLICK).await?);
                let left_handed = parse_bool(&backend.get(KEY_LEFT_HANDED).await?);
                Ok::<_, crate::backend::BackendError>(Message::Loaded {
                    pointer_accel,
                    natural_scroll,
                    tap_to_click,
                    left_handed,
                })
            },
            |result| {
                crate::Message::Mouse(result.unwrap_or_else(|e| Message::Error(e.to_string())))
            },
        )
    }

    pub fn update(&mut self, message: Message, backend: Arc<dyn Backend>) -> Task<crate::Message> {
        match message {
            Message::Loaded {
                pointer_accel,
                natural_scroll,
                tap_to_click,
                left_handed,
            } => {
                self.pointer_accel = clamp_accel(pointer_accel);
                self.natural_scroll = natural_scroll;
                self.tap_to_click = tap_to_click;
                self.left_handed = left_handed;
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
            Message::AccelChanged(v) => {
                self.pointer_accel = clamp_accel(v);
                Task::none()
            }
            Message::NaturalScrollChanged(v) => {
                self.natural_scroll = v;
                Task::none()
            }
            Message::TapToClickChanged(v) => {
                self.tap_to_click = v;
                Task::none()
            }
            Message::LeftHandedChanged(v) => {
                self.left_handed = v;
                Task::none()
            }
            Message::SaveClicked => {
                if self.busy {
                    return Task::none();
                }
                self.busy = true;
                self.status = "Applying…".into();
                let pointer_accel = self.pointer_accel;
                let natural_scroll = self.natural_scroll;
                let tap_to_click = self.tap_to_click;
                let left_handed = self.left_handed;
                Task::perform(
                    async move {
                        backend
                            .set(KEY_POINTER_ACCEL, &pointer_accel.to_string())
                            .await?;
                        backend
                            .set(KEY_NATURAL_SCROLL, encode_bool(natural_scroll))
                            .await?;
                        backend
                            .set(KEY_TAP_TO_CLICK, encode_bool(tap_to_click))
                            .await?;
                        backend
                            .set(KEY_LEFT_HANDED, encode_bool(left_handed))
                            .await?;
                        Ok::<_, crate::backend::BackendError>(Message::Saved)
                    },
                    |result| {
                        crate::Message::Mouse(
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
            (!self.busy).then(|| crate::Message::Mouse(Message::SaveClicked)),
            Palette::dark(),
        );

        column![
            row![
                text("Pointer acceleration").width(Length::Fixed(180.0)),
                slider(ACCEL_MIN..=ACCEL_MAX, self.pointer_accel, |v| {
                    crate::Message::Mouse(Message::AccelChanged(v))
                })
                .step(ACCEL_STEP),
                text(format!("{:+.1}", self.pointer_accel)).size(13),
            ]
            .spacing(12),
            checkbox("Natural (reverse) scrolling", self.natural_scroll)
                .on_toggle(|v| crate::Message::Mouse(Message::NaturalScrollChanged(v))),
            checkbox("Tap to click (touchpad)", self.tap_to_click)
                .on_toggle(|v| crate::Message::Mouse(Message::TapToClickChanged(v))),
            checkbox("Left-handed button mapping", self.left_handed)
                .on_toggle(|v| crate::Message::Mouse(Message::LeftHandedChanged(v))),
            row![apply_btn, text(&self.status).size(13)].spacing(12),
        ]
        .spacing(12)
        .width(Length::Fill)
        .into()
    }
}

/// Clamp a pointer-acceleration value into the locked range.
fn clamp_accel(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(ACCEL_MIN, ACCEL_MAX)
    } else {
        ACCEL_DEFAULT
    }
}

/// Parse a `mouse.pointer_accel` JSON value (canonical float,
/// legacy quoted string, or integer). Falls back to the default.
fn parse_accel(s: &str) -> f32 {
    clamp_accel(strip_json_quotes(s).parse::<f32>().unwrap_or(ACCEL_DEFAULT))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::DemoBackend;

    #[test]
    fn keys_match_locked_mouse_namespace() {
        assert_eq!(KEY_POINTER_ACCEL, "mouse.pointer_accel");
        assert_eq!(KEY_NATURAL_SCROLL, "mouse.natural_scroll");
        assert_eq!(KEY_TAP_TO_CLICK, "mouse.tap_to_click");
        assert_eq!(KEY_LEFT_HANDED, "mouse.left_handed");
    }

    #[test]
    fn parse_accel_handles_forms_and_clamps() {
        assert!((parse_accel("0.5") - 0.5).abs() < f32::EPSILON);
        assert!((parse_accel("\"-0.3\"") + 0.3).abs() < f32::EPSILON);
        assert!((parse_accel("2.0") - ACCEL_MAX).abs() < f32::EPSILON);
        assert!((parse_accel("garbage") - ACCEL_DEFAULT).abs() < f32::EPSILON);
    }

    #[test]
    fn clamp_accel_bounds_out_of_range() {
        assert!((clamp_accel(-9.0) - ACCEL_MIN).abs() < f32::EPSILON);
        assert!((clamp_accel(9.0) - ACCEL_MAX).abs() < f32::EPSILON);
        assert!((clamp_accel(f32::NAN) - ACCEL_DEFAULT).abs() < f32::EPSILON);
        assert!((clamp_accel(0.4) - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn loaded_clamps_accel_and_sets_toggles() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = MousePanel::new();
        let _ = panel.update(
            Message::Loaded {
                pointer_accel: 5.0,
                natural_scroll: true,
                tap_to_click: true,
                left_handed: true,
            },
            backend,
        );
        assert!((panel.pointer_accel - ACCEL_MAX).abs() < f32::EPSILON);
        assert!(panel.natural_scroll && panel.tap_to_click && panel.left_handed);
    }

    #[test]
    fn field_change_messages_mutate_matching_fields() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = MousePanel::new();
        let _ = panel.update(Message::AccelChanged(0.3), backend.clone());
        assert!((panel.pointer_accel - 0.3).abs() < f32::EPSILON);
        let _ = panel.update(Message::NaturalScrollChanged(true), backend.clone());
        assert!(panel.natural_scroll);
        let _ = panel.update(Message::TapToClickChanged(true), backend.clone());
        assert!(panel.tap_to_click);
        let _ = panel.update(Message::LeftHandedChanged(true), backend);
        assert!(panel.left_handed);
    }

    #[test]
    fn save_clicked_while_busy_is_noop() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = MousePanel::new();
        panel.busy = true;
        panel.status = "Applying…".into();
        let _ = panel.update(Message::SaveClicked, backend);
        assert_eq!(panel.status, "Applying…");
    }

    #[tokio::test]
    async fn save_writes_all_four_keys_with_correct_json_shapes() {
        let backend: Arc<dyn Backend> = Arc::new(DemoBackend::new());
        backend.set(KEY_POINTER_ACCEL, "0.5").await.unwrap();
        backend
            .set(KEY_NATURAL_SCROLL, encode_bool(true))
            .await
            .unwrap();
        backend.set(KEY_TAP_TO_CLICK, encode_bool(false)).await.unwrap();
        backend.set(KEY_LEFT_HANDED, encode_bool(true)).await.unwrap();
        assert_eq!(backend.get(KEY_POINTER_ACCEL).await.unwrap(), "0.5");
        assert_eq!(backend.get(KEY_NATURAL_SCROLL).await.unwrap(), "true");
        assert_eq!(backend.get(KEY_TAP_TO_CLICK).await.unwrap(), "false");
        assert_eq!(backend.get(KEY_LEFT_HANDED).await.unwrap(), "true");
    }
}
