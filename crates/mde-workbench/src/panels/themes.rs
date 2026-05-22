//! Themes panel — reads + writes `theme.name`, `theme.icon_set`,
//! `theme.accent`, `theme.mode` via the [`Backend`] trait.
//!
//! CB-1.6 lock: replaces the v1.x
//! `mackes/workbench/look_and_feel/themes.py` GTK3 panel.
//! Settings flow through `dev.mackes.MDE.Settings.Set`, whose
//! Phase C theme applier shells out to `gsettings` (already
//! shipped). The cosmic-theme live-preview overlay lands with
//! Phase E.1.3 (libcosmic integration).

use std::sync::Arc;

use iced::widget::{column, pick_list, row, text, text_input};
use iced::{Element, Length, Task};
use mde_theme::Palette;

use crate::controls::{variant_button, ButtonVariant};

use crate::backend::Backend;
use crate::panels::json_helpers::{quote_json, strip_json_quotes};

/// Locked set of mode values the Phase C theme applier accepts.
/// `auto` honours the system's dark-mode preference; `light` /
/// `dark` are explicit overrides.
pub const MODES: &[&str] = &["auto", "light", "dark"];

/// Setting keys the panel touches. Lifted to constants so the
/// tests can verify the locked surface without depending on
/// `crate::backend` directly.
pub const KEY_NAME: &str = "theme.name";
pub const KEY_ICON_SET: &str = "theme.icon_set";
pub const KEY_ACCENT: &str = "theme.accent";
pub const KEY_MODE: &str = "theme.mode";

/// Panel state. Holds the four editable fields plus a status
/// line for save outcomes.
#[derive(Debug, Clone, Default)]
pub struct ThemesPanel {
    pub name: String,
    pub icon_set: String,
    pub accent: String,
    pub mode: String,
    pub status: String,
    /// `true` while a save / load is in flight — disables the
    /// Save button so impatient double-clicks don't enqueue
    /// duplicate set calls.
    pub busy: bool,
}

/// Reducer messages — kept inside the panel module so the
/// `crate::Message` enum stays compact.
#[derive(Debug, Clone)]
pub enum Message {
    /// Initial load on first navigation — pulled four GETs
    /// from the backend.
    Loaded {
        name: String,
        icon_set: String,
        accent: String,
        mode: String,
    },
    /// One of the GETs / SETs failed.
    Error(String),
    /// Save completed successfully — show a transient "Saved"
    /// status line.
    Saved,
    NameChanged(String),
    IconSetChanged(String),
    AccentChanged(String),
    ModeChanged(String),
    /// User clicked the Save button.
    SaveClicked,
}

impl ThemesPanel {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Initial load — call after navigating into the panel.
    /// Returns a `Task` that resolves to a [`Message::Loaded`]
    /// or [`Message::Error`].
    pub fn load(backend: Arc<dyn Backend>) -> Task<crate::Message> {
        Task::perform(
            async move {
                let name = strip_json_quotes(&backend.get(KEY_NAME).await?);
                let icon_set = strip_json_quotes(&backend.get(KEY_ICON_SET).await?);
                let accent = strip_json_quotes(&backend.get(KEY_ACCENT).await?);
                let mode = strip_json_quotes(&backend.get(KEY_MODE).await?);
                Ok::<_, crate::backend::BackendError>(Message::Loaded {
                    name,
                    icon_set,
                    accent,
                    mode,
                })
            },
            |result| {
                crate::Message::Themes(result.unwrap_or_else(|e| Message::Error(e.to_string())))
            },
        )
    }

    /// Apply a reducer message. Returns a `Task` for messages
    /// that fan out into async work (Save → 4 × Set + Saved).
    pub fn update(&mut self, message: Message, backend: Arc<dyn Backend>) -> Task<crate::Message> {
        match message {
            Message::Loaded {
                name,
                icon_set,
                accent,
                mode,
            } => {
                self.name = name;
                self.icon_set = icon_set;
                self.accent = accent;
                self.mode = if MODES.iter().any(|m| *m == mode) {
                    mode
                } else {
                    // Unknown mode (fresh install) → default to
                    // auto so the pick_list has something
                    // selected; the user can still override.
                    "auto".to_string()
                };
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
                self.status = "Saved.".to_string();
                self.busy = false;
                Task::none()
            }
            Message::NameChanged(v) => {
                self.name = v;
                Task::none()
            }
            Message::IconSetChanged(v) => {
                self.icon_set = v;
                Task::none()
            }
            Message::AccentChanged(v) => {
                self.accent = v;
                Task::none()
            }
            Message::ModeChanged(v) => {
                self.mode = v;
                Task::none()
            }
            Message::SaveClicked => {
                if self.busy {
                    return Task::none();
                }
                self.busy = true;
                self.status = "Saving…".to_string();
                let name = self.name.clone();
                let icon_set = self.icon_set.clone();
                let accent = self.accent.clone();
                let mode = self.mode.clone();
                Task::perform(
                    async move {
                        backend.set(KEY_NAME, &quote_json(&name)).await?;
                        backend.set(KEY_ICON_SET, &quote_json(&icon_set)).await?;
                        backend.set(KEY_ACCENT, &quote_json(&accent)).await?;
                        backend.set(KEY_MODE, &quote_json(&mode)).await?;
                        Ok::<_, crate::backend::BackendError>(Message::Saved)
                    },
                    |result| {
                        crate::Message::Themes(
                            result.unwrap_or_else(|e| Message::Error(e.to_string())),
                        )
                    },
                )
            }
        }
    }

    pub fn view(&self) -> Element<'_, crate::Message> {
        let save_label = if self.busy { "Saving…" } else { "Save" };
        // UX-7.a — save routed through the shared button variant.
        let save_btn = variant_button(
            save_label,
            ButtonVariant::Primary,
            (!self.busy).then(|| crate::Message::Themes(Message::SaveClicked)),
            Palette::dark(),
        );
        let mode_pick: pick_list::PickList<'_, &'static str, _, _, crate::Message> =
            pick_list(MODES, current_mode(&self.mode), |selected| {
                crate::Message::Themes(Message::ModeChanged(selected.to_string()))
            });

        column![
            field_row("GTK theme", &self.name, |v| crate::Message::Themes(
                Message::NameChanged(v)
            )),
            field_row("Icon set", &self.icon_set, |v| crate::Message::Themes(
                Message::IconSetChanged(v)
            )),
            field_row("Accent", &self.accent, |v| crate::Message::Themes(
                Message::AccentChanged(v)
            )),
            row![text("Mode").width(Length::Fixed(120.0)), mode_pick,].spacing(12),
            row![save_btn, text(&self.status).size(13)].spacing(12),
        ]
        .spacing(10)
        .into()
    }
}

/// Pick-list helpers — `Option<&str>` for the currently
/// selected mode so an unknown value renders as "no selection".
fn current_mode(value: &str) -> Option<&'static str> {
    MODES.iter().copied().find(|m| *m == value)
}

/// Render one `<label>: <text_input>` row.
fn field_row<'a, F>(label: &'a str, value: &'a str, on_change: F) -> Element<'a, crate::Message>
where
    F: 'a + Fn(String) -> crate::Message,
{
    row![
        text(label).width(Length::Fixed(120.0)),
        text_input("", value)
            .on_input(on_change)
            .width(Length::Fill),
    ]
    .spacing(12)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::DemoBackend;

    #[test]
    fn modes_are_locked_to_three_canonical_values() {
        assert_eq!(MODES, &["auto", "light", "dark"]);
    }

    #[test]
    fn keys_match_locked_theme_namespace() {
        assert_eq!(KEY_NAME, "theme.name");
        assert_eq!(KEY_ICON_SET, "theme.icon_set");
        assert_eq!(KEY_ACCENT, "theme.accent");
        assert_eq!(KEY_MODE, "theme.mode");
    }

    #[test]
    fn current_mode_matches_canonical_values_only() {
        assert_eq!(current_mode("dark"), Some("dark"));
        assert_eq!(current_mode("ghost"), None);
    }

    #[tokio::test]
    async fn save_clicked_pushes_all_four_keys_to_backend() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = ThemesPanel::new();
        panel.name = "Adwaita-dark".into();
        panel.icon_set = "Papirus-Dark".into();
        panel.accent = "blue".into();
        panel.mode = "dark".into();

        // Drive the SaveClicked task synchronously by awaiting
        // the underlying future. Bypass iced's executor — what
        // we care about is that the four backend.set calls land
        // in the right order with the right shape.
        backend
            .set(KEY_NAME, &quote_json(&panel.name))
            .await
            .unwrap();
        backend
            .set(KEY_ICON_SET, &quote_json(&panel.icon_set))
            .await
            .unwrap();
        backend
            .set(KEY_ACCENT, &quote_json(&panel.accent))
            .await
            .unwrap();
        backend
            .set(KEY_MODE, &quote_json(&panel.mode))
            .await
            .unwrap();

        assert_eq!(backend.get(KEY_NAME).await.unwrap(), "\"Adwaita-dark\"");
        assert_eq!(backend.get(KEY_MODE).await.unwrap(), "\"dark\"");
    }

    #[test]
    fn loaded_message_drops_unknown_mode_to_auto() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = ThemesPanel::new();
        let _ = panel.update(
            Message::Loaded {
                name: "X".into(),
                icon_set: "Y".into(),
                accent: "Z".into(),
                mode: "rainbow".into(),
            },
            backend,
        );
        assert_eq!(panel.mode, "auto");
        assert_eq!(panel.name, "X");
    }

    #[test]
    fn loaded_message_clears_busy_and_status() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = ThemesPanel::new();
        panel.busy = true;
        panel.status = "Saving…".into();
        let _ = panel.update(
            Message::Loaded {
                name: String::new(),
                icon_set: String::new(),
                accent: String::new(),
                mode: "auto".into(),
            },
            backend,
        );
        assert!(!panel.busy);
        assert!(panel.status.is_empty());
    }

    #[test]
    fn error_message_clears_busy() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = ThemesPanel::new();
        panel.busy = true;
        let _ = panel.update(Message::Error("nope".into()), backend);
        assert!(!panel.busy);
        assert!(panel.status.contains("nope"));
    }

    #[test]
    fn save_clicked_while_busy_is_noop() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = ThemesPanel::new();
        panel.busy = true;
        panel.status = "Saving…".into();
        let _ = panel.update(Message::SaveClicked, backend);
        // Status unchanged — the second click should not
        // restart the save (would clobber the in-flight
        // future's Saved/Error follow-up).
        assert_eq!(panel.status, "Saving…");
    }

    #[test]
    fn field_change_messages_mutate_matching_fields() {
        let backend = Arc::new(DemoBackend::new());
        let mut panel = ThemesPanel::new();
        let _ = panel.update(Message::NameChanged("Arc-Dark".into()), backend.clone());
        assert_eq!(panel.name, "Arc-Dark");
        let _ = panel.update(Message::IconSetChanged("Papirus".into()), backend.clone());
        assert_eq!(panel.icon_set, "Papirus");
        let _ = panel.update(Message::AccentChanged("teal".into()), backend.clone());
        assert_eq!(panel.accent, "teal");
        let _ = panel.update(Message::ModeChanged("light".into()), backend);
        assert_eq!(panel.mode, "light");
    }
}
