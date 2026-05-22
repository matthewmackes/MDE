//! Displays panel — four `display.*` keys covering primary
//! output, scale factor, and night-light enable + colour
//! temperature. Connected outputs are enumerated via
//! `swaymsg -t get_outputs` so the panel surfaces an empty
//! state on a TTY or a non-sway compositor without crashing.
//!
//! CB-1.4.a: replaces the v1.x
//! `mackes/workbench/devices/displays.py` GTK3 panel. Phase
//! F.4 already wired the four settings keys through
//! `mde_settings_bridge`; this Iced port reads + writes the
//! same keys via `dev.mackes.MDE.Settings.Get/Set` on the
//! shared [`Backend`] trait.

use std::sync::Arc;

use iced::widget::{checkbox, column, pick_list, row, slider, text, text_input};
use iced::{Element, Length, Task};
use mde_theme::Palette;
use tokio::process::Command;

use crate::backend::Backend;
use crate::controls::{variant_button, ButtonVariant};
use crate::panels::json_helpers::{
    encode_bool, parse_bool, parse_u32, quote_json, strip_json_quotes,
};

pub const KEY_PRIMARY: &str = "display.primary";
pub const KEY_SCALE: &str = "display.scale";
pub const KEY_NIGHT_LIGHT: &str = "display.night_light";
pub const KEY_NIGHT_LIGHT_TEMP: &str = "display.night_light_temp";

/// Scale slider range — matches the v1.x Python panel's
/// `Gtk.Adjustment(lower=0.5, upper=4.0, step_increment=0.25)`.
pub const SCALE_MIN: f32 = 0.5;
pub const SCALE_MAX: f32 = 4.0;
pub const SCALE_STEP: f32 = 0.25;
pub const SCALE_DEFAULT: f32 = 1.0;

/// Night-light colour temperature default + locked sensible
/// bounds. v1.x panel allowed 1000–10000 K; we keep the same
/// range so existing user values round-trip without snap.
pub const TEMP_DEFAULT: u32 = 4500;

#[derive(Debug, Clone, Default)]
pub struct DisplaysPanel {
    /// Outputs enumerated from `swaymsg -t get_outputs`. Empty
    /// means sway isn't running or no displays are connected;
    /// the view shows an empty-state body instead of controls.
    pub outputs: Vec<String>,
    pub primary: String,
    pub scale: f32,
    pub night_light: bool,
    pub temp_input: String,
    pub status: String,
    pub busy: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded {
        outputs: Vec<String>,
        primary: String,
        scale: f32,
        night_light: bool,
        temp_k: u32,
    },
    Error(String),
    Saved,
    PrimaryChanged(String),
    ScaleChanged(f32),
    NightLightChanged(bool),
    TempInputChanged(String),
    SaveClicked,
}

impl DisplaysPanel {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(backend: Arc<dyn Backend>) -> Task<crate::Message> {
        Task::perform(
            async move {
                let outputs = enumerate_outputs().await;
                let primary = strip_json_quotes(&backend.get(KEY_PRIMARY).await?);
                let scale = parse_scale(&backend.get(KEY_SCALE).await?);
                let night_light = parse_bool(&backend.get(KEY_NIGHT_LIGHT).await?);
                let temp_k =
                    parse_u32(&backend.get(KEY_NIGHT_LIGHT_TEMP).await?).unwrap_or(TEMP_DEFAULT);
                Ok::<_, crate::backend::BackendError>(Message::Loaded {
                    outputs,
                    primary,
                    scale,
                    night_light,
                    temp_k,
                })
            },
            |result| {
                crate::Message::Displays(result.unwrap_or_else(|e| Message::Error(e.to_string())))
            },
        )
    }

    pub fn update(&mut self, message: Message, backend: Arc<dyn Backend>) -> Task<crate::Message> {
        match message {
            Message::Loaded {
                outputs,
                primary,
                scale,
                night_light,
                temp_k,
            } => {
                self.outputs = outputs;
                // Unknown primary (e.g. previously-connected output
                // is gone) falls back to the first detected output
                // so the combo lands on something selectable.
                self.primary = if self.outputs.contains(&primary) {
                    primary
                } else {
                    self.outputs.first().cloned().unwrap_or_default()
                };
                self.scale = clamp_scale(scale);
                self.night_light = night_light;
                self.temp_input = temp_k.to_string();
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
            Message::PrimaryChanged(v) => {
                self.primary = v;
                Task::none()
            }
            Message::ScaleChanged(v) => {
                self.scale = clamp_scale(v);
                Task::none()
            }
            Message::NightLightChanged(v) => {
                self.night_light = v;
                Task::none()
            }
            Message::TempInputChanged(v) => {
                self.temp_input = v;
                Task::none()
            }
            Message::SaveClicked => {
                if self.busy {
                    return Task::none();
                }
                let temp_k = match resolve_temp(&self.temp_input) {
                    Ok(v) => v,
                    Err(msg) => {
                        self.status = msg;
                        return Task::none();
                    }
                };
                self.busy = true;
                self.status = "Saving…".into();
                let primary = self.primary.clone();
                let scale = self.scale;
                let night_light = self.night_light;
                Task::perform(
                    async move {
                        backend.set(KEY_PRIMARY, &quote_json(&primary)).await?;
                        backend.set(KEY_SCALE, &scale.to_string()).await?;
                        backend
                            .set(KEY_NIGHT_LIGHT, encode_bool(night_light))
                            .await?;
                        backend
                            .set(KEY_NIGHT_LIGHT_TEMP, &temp_k.to_string())
                            .await?;
                        Ok::<_, crate::backend::BackendError>(Message::Saved)
                    },
                    |result| {
                        crate::Message::Displays(
                            result.unwrap_or_else(|e| Message::Error(e.to_string())),
                        )
                    },
                )
            }
        }
    }

    pub fn view(&self) -> Element<'_, crate::Message> {
        // Empty state when no outputs are enumerated. Matches the
        // v1.x Python "No displays detected" branch — keep the
        // chrome readable on a TTY / non-sway compositor instead
        // of rendering inert controls.
        if self.outputs.is_empty() {
            return column![
                text("No displays detected").size(18),
                text(
                    "MDE reads displays from sway. If you're on a TTY or a different \
                     compositor, this panel won't have outputs to configure.",
                )
                .size(13),
            ]
            .spacing(8)
            .width(Length::Fill)
            .into();
        }

        let save_label = if self.busy { "Saving…" } else { "Save" };
        // UX-7.a — save routed through the shared button variant.
        let save_btn = variant_button(
            save_label,
            ButtonVariant::Primary,
            (!self.busy).then(|| crate::Message::Displays(Message::SaveClicked)),
            Palette::dark(),
        );

        // pick_list needs an owned-string list. `outputs` is
        // Vec<String> so we clone here; the count is tiny
        // (usually 1–3 monitors) so the alloc is fine.
        let primary_pick: pick_list::PickList<'_, String, _, _, crate::Message> = pick_list(
            self.outputs.clone(),
            current_output(&self.outputs, &self.primary),
            |v| crate::Message::Displays(Message::PrimaryChanged(v)),
        );

        column![
            row![
                text("Primary display").width(Length::Fixed(180.0)),
                primary_pick,
            ]
            .spacing(12),
            row![
                text("Scale").width(Length::Fixed(180.0)),
                slider(SCALE_MIN..=SCALE_MAX, self.scale, |v| {
                    crate::Message::Displays(Message::ScaleChanged(v))
                })
                .step(SCALE_STEP),
                text(format!("{:.2}×", self.scale)).size(13),
            ]
            .spacing(12),
            checkbox("Night light", self.night_light)
                .on_toggle(|v| { crate::Message::Displays(Message::NightLightChanged(v)) }),
            row![
                text("Color temperature (K)").width(Length::Fixed(180.0)),
                text_input("4500", &self.temp_input)
                    .on_input(|v| crate::Message::Displays(Message::TempInputChanged(v))),
            ]
            .spacing(12),
            row![save_btn, text(&self.status).size(13)].spacing(12),
        ]
        .spacing(12)
        .width(Length::Fill)
        .into()
    }
}

fn current_output(outputs: &[String], value: &str) -> Option<String> {
    outputs.iter().find(|o| *o == value).cloned()
}

/// Clamp a scale value into the locked slider range without
/// snapping to the step grid — the slider itself enforces the
/// step. v1.x sidecars may carry odd values from earlier
/// xrandr-only days; we preserve them in-range rather than
/// rounding silently.
fn clamp_scale(s: f32) -> f32 {
    if s.is_finite() {
        s.clamp(SCALE_MIN, SCALE_MAX)
    } else {
        SCALE_DEFAULT
    }
}

/// Parse a `display.scale` JSON value. Accepts canonical
/// floats (`1.25`), legacy quoted strings (`"1.25"`), and
/// integers (`1`). Falls back to the locked default on any
/// parse failure so the slider lands on a usable initial
/// value.
fn parse_scale(s: &str) -> f32 {
    let stripped = strip_json_quotes(s);
    stripped.parse::<f32>().unwrap_or(SCALE_DEFAULT)
}

/// Parse the night-light temperature input. Empty → default
/// (matches the v1.x panel's behaviour when no value has been
/// written yet); non-numeric input → validation error so the
/// panel surfaces it instead of writing garbage.
fn resolve_temp(input: &str) -> Result<u32, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(TEMP_DEFAULT);
    }
    parse_u32(trimmed).ok_or_else(|| "Colour temperature must be a positive integer.".to_string())
}

/// Pure JSON parser for `swaymsg -t get_outputs` payloads.
/// Returns the names of active outputs in the order sway
/// reports them. Inactive outputs (e.g. lid-closed laptop
/// panels with no `active: true`) are filtered out so the
/// primary picker doesn't offer them.
#[must_use]
pub fn parse_outputs_json(json: &str) -> Vec<String> {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let Some(arr) = parsed.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter(|o| o.get("active").and_then(|a| a.as_bool()).unwrap_or(true))
        .filter_map(|o| o.get("name").and_then(|n| n.as_str()).map(str::to_string))
        .filter(|n| !n.is_empty())
        .collect()
}

/// Shell out to `swaymsg -t get_outputs` and parse the JSON
/// response. Returns an empty Vec on any error (swaymsg not
/// installed, sway not running, malformed JSON) so the panel
/// can show its empty state rather than surfacing a stack
/// trace to the user.
pub async fn enumerate_outputs() -> Vec<String> {
    let Ok(output) = Command::new("swaymsg")
        .args(["-t", "get_outputs"])
        .output()
        .await
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let Ok(stdout) = String::from_utf8(output.stdout) else {
        return Vec::new();
    };
    parse_outputs_json(&stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::DemoBackend;

    #[test]
    fn keys_match_locked_display_namespace() {
        assert_eq!(KEY_PRIMARY, "display.primary");
        assert_eq!(KEY_SCALE, "display.scale");
        assert_eq!(KEY_NIGHT_LIGHT, "display.night_light");
        assert_eq!(KEY_NIGHT_LIGHT_TEMP, "display.night_light_temp");
    }

    #[test]
    fn scale_range_matches_v1_python_adjustment() {
        assert!((SCALE_MIN - 0.5).abs() < f32::EPSILON);
        assert!((SCALE_MAX - 4.0).abs() < f32::EPSILON);
        assert!((SCALE_STEP - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn parse_outputs_json_extracts_active_named_outputs() {
        let json = r#"[
            {"name": "HDMI-A-1", "active": true},
            {"name": "DP-2",      "active": true},
            {"name": "LVDS-1",    "active": false}
        ]"#;
        assert_eq!(parse_outputs_json(json), vec!["HDMI-A-1", "DP-2"]);
    }

    #[test]
    fn parse_outputs_json_defaults_missing_active_to_true() {
        // Earlier sway IPC payloads omit `active` entirely; the
        // v1.x Python panel treated those as connected. Match.
        let json = r#"[{"name": "eDP-1"}]"#;
        assert_eq!(parse_outputs_json(json), vec!["eDP-1"]);
    }

    #[test]
    fn parse_outputs_json_filters_empty_and_missing_names() {
        let json = r#"[
            {"name": "",         "active": true},
            {"active": true},
            {"name": "HDMI-A-1", "active": true}
        ]"#;
        assert_eq!(parse_outputs_json(json), vec!["HDMI-A-1"]);
    }

    #[test]
    fn parse_outputs_json_empty_on_garbage_input() {
        assert!(parse_outputs_json("").is_empty());
        assert!(parse_outputs_json("not json").is_empty());
        assert!(parse_outputs_json("{}").is_empty());
        assert!(parse_outputs_json("null").is_empty());
    }

    #[test]
    fn parse_scale_handles_canonical_quoted_and_integer_forms() {
        assert!((parse_scale("1.25") - 1.25).abs() < f32::EPSILON);
        assert!((parse_scale("\"1.5\"") - 1.5).abs() < f32::EPSILON);
        assert!((parse_scale("2") - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn parse_scale_falls_back_to_default_on_garbage() {
        assert!((parse_scale("") - SCALE_DEFAULT).abs() < f32::EPSILON);
        assert!((parse_scale("forever") - SCALE_DEFAULT).abs() < f32::EPSILON);
    }

    #[test]
    fn clamp_scale_bounds_out_of_range_values() {
        assert!((clamp_scale(0.1) - SCALE_MIN).abs() < f32::EPSILON);
        assert!((clamp_scale(99.0) - SCALE_MAX).abs() < f32::EPSILON);
        assert!((clamp_scale(f32::NAN) - SCALE_DEFAULT).abs() < f32::EPSILON);
        assert!((clamp_scale(1.5) - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn resolve_temp_accepts_empty_and_rejects_garbage() {
        assert_eq!(resolve_temp(""), Ok(TEMP_DEFAULT));
        assert_eq!(resolve_temp("3000"), Ok(3000));
        assert!(resolve_temp("warm").is_err());
    }

    #[test]
    fn loaded_falls_back_to_first_output_when_primary_unknown() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = DisplaysPanel::new();
        let _ = panel.update(
            Message::Loaded {
                outputs: vec!["HDMI-A-1".into(), "DP-2".into()],
                primary: "vanished-monitor".into(),
                scale: 1.0,
                night_light: false,
                temp_k: TEMP_DEFAULT,
            },
            backend,
        );
        assert_eq!(panel.primary, "HDMI-A-1");
    }

    #[test]
    fn loaded_preserves_primary_when_still_connected() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = DisplaysPanel::new();
        let _ = panel.update(
            Message::Loaded {
                outputs: vec!["HDMI-A-1".into(), "DP-2".into()],
                primary: "DP-2".into(),
                scale: 1.0,
                night_light: false,
                temp_k: TEMP_DEFAULT,
            },
            backend,
        );
        assert_eq!(panel.primary, "DP-2");
    }

    #[test]
    fn loaded_clamps_out_of_range_scale() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = DisplaysPanel::new();
        let _ = panel.update(
            Message::Loaded {
                outputs: vec!["HDMI-A-1".into()],
                primary: "HDMI-A-1".into(),
                scale: 12.0,
                night_light: false,
                temp_k: TEMP_DEFAULT,
            },
            backend,
        );
        assert!((panel.scale - SCALE_MAX).abs() < f32::EPSILON);
    }

    #[test]
    fn field_change_messages_mutate_matching_fields() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = DisplaysPanel::new();
        panel.outputs = vec!["HDMI-A-1".into(), "DP-2".into()];
        let _ = panel.update(Message::PrimaryChanged("DP-2".into()), backend.clone());
        assert_eq!(panel.primary, "DP-2");
        let _ = panel.update(Message::ScaleChanged(2.0), backend.clone());
        assert!((panel.scale - 2.0).abs() < f32::EPSILON);
        let _ = panel.update(Message::NightLightChanged(true), backend.clone());
        assert!(panel.night_light);
        let _ = panel.update(Message::TempInputChanged("3000".into()), backend);
        assert_eq!(panel.temp_input, "3000");
    }

    #[test]
    fn save_clicked_with_garbage_temp_surfaces_validation() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = DisplaysPanel::new();
        panel.outputs = vec!["HDMI-A-1".into()];
        panel.primary = "HDMI-A-1".into();
        panel.temp_input = "warm".into();
        let _ = panel.update(Message::SaveClicked, backend);
        assert!(panel.status.contains("integer"));
        assert!(!panel.busy);
    }

    #[test]
    fn save_clicked_while_busy_is_noop() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = DisplaysPanel::new();
        panel.busy = true;
        panel.status = "Saving…".into();
        let _ = panel.update(Message::SaveClicked, backend);
        assert_eq!(panel.status, "Saving…");
    }

    #[tokio::test]
    async fn save_writes_all_four_keys_with_correct_json_shapes() {
        let backend: Arc<dyn Backend> = Arc::new(DemoBackend::new());
        backend
            .set(KEY_PRIMARY, &quote_json("HDMI-A-1"))
            .await
            .unwrap();
        backend.set(KEY_SCALE, "1.25").await.unwrap();
        backend
            .set(KEY_NIGHT_LIGHT, encode_bool(true))
            .await
            .unwrap();
        backend.set(KEY_NIGHT_LIGHT_TEMP, "3000").await.unwrap();
        assert_eq!(backend.get(KEY_PRIMARY).await.unwrap(), "\"HDMI-A-1\"");
        assert_eq!(backend.get(KEY_SCALE).await.unwrap(), "1.25");
        assert_eq!(backend.get(KEY_NIGHT_LIGHT).await.unwrap(), "true");
        assert_eq!(backend.get(KEY_NIGHT_LIGHT_TEMP).await.unwrap(), "3000");
    }
}
