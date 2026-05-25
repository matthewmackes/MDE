//! Portal-2 Iced application — layer-shell Dock, all outputs.
//!
//! Renders a theme-adaptive 56 px bottom strip as the Dock baseline:
//! - Dark mode: Classic ChromeOS charcoal `#202124`
//! - Light mode: Classic ChromeOS off-white `#f1f3f4`
//!
//! `StartMode::AllScreens` makes the compositor spawn one strip per
//! connected output at startup; hotplug is handled by the compositor
//! automatically (wlr-layer-shell NULL-output protocol).
//!
//! Portal-4 onward fills in the nav buttons and interactive segments.

use iced::widget::container;
use iced::{Element, Length, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;

use crate::fonts::FONT_INTEL_ONE_MONO;

/// Crate-private app-id constant visible to the layer-shell compositor.
pub(crate) const APP_ID: &str = "dev.mackes.MDE.Portal";

/// Height of the Dock strip in logical pixels (Portal-2 lock).
pub const DOCK_HEIGHT_PX: u32 = 56;

/// Classic ChromeOS charcoal — `#202124` (dark mode).
const CHARCOAL: iced::Color = iced::Color {
    r: 0.125_f32,
    g: 0.129_f32,
    b: 0.141_f32,
    a: 1.0,
};

/// Classic ChromeOS off-white — `#f1f3f4` (light mode, R4-Q75).
const OFF_WHITE: iced::Color = iced::Color {
    r: 0.945_f32,
    g: 0.953_f32,
    b: 0.957_f32,
    a: 1.0,
};

/// Messages the Dock application handles.
///
/// `#[to_layer_message]` generates the `TryInto<LayershellCustomActions>`
/// impl required by `iced_layershell::Application::run`.
#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    /// No-op placeholder — Portal-4 onward adds domain-specific variants.
    Noop,
}

/// Dock application state (Portal-2).
#[derive(Debug, Default)]
pub struct DockApp;

impl DockApp {
    /// Construct iced_layershell settings for the Dock surface.
    ///
    /// `StartMode::AllScreens` instructs the compositor to spawn one
    /// instance per connected output. Hotplug is handled by the
    /// wlr-layer-shell NULL-output protocol at the compositor level.
    pub fn settings() -> Settings<()> {
        Settings {
            layer_settings: LayerShellSettings {
                size: Some((0, DOCK_HEIGHT_PX)),
                exclusive_zone: DOCK_HEIGHT_PX as i32,
                anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
                layer: Layer::Top,
                keyboard_interactivity: KeyboardInteractivity::None,
                start_mode: StartMode::AllScreens,
                ..Default::default()
            },
            // Portal-3: Intel One Mono as primary font; resolved by
            // fontconfig from the `intel-one-mono-fonts` RPM dep.
            default_font: FONT_INTEL_ONE_MONO,
            ..Default::default()
        }
    }
}

impl iced_layershell::Application for DockApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Task<Message>) {
        (Self, Task::none())
    }

    fn namespace(&self) -> String {
        APP_ID.to_string()
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let bg = if theme == Theme::Dark { CHARCOAL } else { OFF_WHITE };
        container(iced::widget::Space::new(Length::Fill, Length::Fill))
            .style(move |_theme: &Theme| container::Style {
                background: Some(iced::Background::Color(bg)),
                ..Default::default()
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dock_height_is_56() {
        assert_eq!(DOCK_HEIGHT_PX, 56, "Portal-2 lock: Dock is 56 px");
    }

    #[test]
    fn charcoal_matches_chromeos_lock() {
        // Classic ChromeOS visual lock: #202124 = rgb(32, 33, 36)
        // ≈ (0.125, 0.129, 0.141) in [0,1].
        let r = (CHARCOAL.r * 255.0).round() as u8;
        let g = (CHARCOAL.g * 255.0).round() as u8;
        let b = (CHARCOAL.b * 255.0).round() as u8;
        assert_eq!((r, g, b), (32, 33, 36), "#202124 charcoal");
    }

    #[test]
    fn off_white_matches_chromeos_lock() {
        // Classic ChromeOS light: #f1f3f4 = rgb(241, 243, 244)
        // ≈ (0.945, 0.953, 0.957) in [0,1].
        let r = (OFF_WHITE.r * 255.0).round() as u8;
        let g = (OFF_WHITE.g * 255.0).round() as u8;
        let b = (OFF_WHITE.b * 255.0).round() as u8;
        assert_eq!((r, g, b), (241, 243, 244), "#f1f3f4 off-white");
    }

    #[test]
    fn settings_use_all_screens() {
        let settings = DockApp::settings();
        assert!(
            matches!(settings.layer_settings.start_mode, StartMode::AllScreens),
            "Portal-2: Dock must use AllScreens so compositor spawns one per output"
        );
    }

    #[test]
    fn settings_exclusive_zone_equals_dock_height() {
        let s = DockApp::settings();
        assert_eq!(
            s.layer_settings.exclusive_zone, DOCK_HEIGHT_PX as i32,
            "exclusive zone must match dock height so desktop doesn't overlap"
        );
    }

    #[test]
    fn app_id_is_portal_bus_name() {
        assert_eq!(APP_ID, "dev.mackes.MDE.Portal");
    }
}
