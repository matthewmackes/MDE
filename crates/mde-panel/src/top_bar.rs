//! Phase E.17 — top-bar visual chrome (2026 design language).
//!
//! Lays out the panel's six locked zones in a single 40 px row:
//!
//! ```text
//!   ┌─────────────────────────────────────────────────────────┐
//!   │  ⌂  ★ ★ ★ │ ▷ Focused window title  │ ⌗ ⌘ ⊞ │  ◉ ◉ ◉ │ 10:42 │
//!   │ Start  Pinned       Tasklist          Cluster   Tray   Clock │
//!   └─────────────────────────────────────────────────────────┘
//! ```
//!
//! 2026 design language locks (deliberate departure from the
//! prior CSS approach):
//! - **Surface:** dark glass (#0e0e10 at 92% alpha when the
//!   compositor exposes backdrop blur; opaque otherwise). Single
//!   1px hairline at the top edge in `rgba(244,244,244,0.06)`.
//! - **Separators:** 8 px of negative space, no painted divider.
//!   Zones distinguish themselves by content, not chrome.
//! - **Accent system:** one bold accent color (per-preset, default
//!   the IBM-blue `#2b9af3`). Greyscale everywhere else; hover
//!   states use a 14%-alpha underglow of the accent, not a
//!   button-style background flip.
//! - **Typography:** Red Hat Mono for the clock (tabular nums),
//!   Red Hat Text 12 px / weight 500 for labels.
//! - **Microinteraction:** 180 ms ease-out for every state
//!   transition; longer transforms (>500 ms) drop under
//!   `Motion::Reduced` (settings tie-in pending E.5/E.6).
//!
//! The widget is pane-driven: `TopBar::view(state)` consumes a
//! `TopBarState` that names which child widget renders in each
//! zone. Future ports (E.10 dock host, E.11 start menu, E.4.1
//! sway cluster, etc.) plug into specific `Pane` slots without
//! re-laying-out the bar.

use iced::widget::{container, row, text, Space};
use iced::{Color, Element, Length, Padding, Theme};

use crate::Pane;

/// Height of the top bar in logical pixels (Phase 1.1.0 Win10 lock).
pub const TOP_BAR_HEIGHT_PX: u16 = 40;

/// Per-zone padding (horizontal) — keeps icons + text from
/// touching the bar's edges.
pub const ZONE_PADDING_X: u16 = 12;

/// State injected by the panel orchestrator. Each `Option<String>`
/// is a placeholder for a richer per-port widget that lands at
/// E.4 - E.29; the skeleton renders the string verbatim.
#[derive(Debug, Clone, Default)]
pub struct TopBarState {
    pub start_label: Option<String>,
    pub pinned_labels: Vec<String>,
    pub tasklist_label: Option<String>,
    pub cluster_label: Option<String>,
    pub tray_labels: Vec<String>,
    pub clock_label: Option<String>,
}

impl TopBarState {
    /// Minimal demo state — used by `App::view()` until the per-port
    /// state writers land.
    #[must_use]
    pub fn demo() -> Self {
        Self {
            start_label: Some("⌂".to_string()),
            pinned_labels: vec!["★".into(), "★".into(), "★".into()],
            tasklist_label: Some("Workbench · Network · mesh_ssh".into()),
            cluster_label: Some("⌗ ⌘ ⊞".into()),
            tray_labels: vec!["◉".into(), "◉".into(), "◉".into()],
            clock_label: Some(default_clock_label()),
        }
    }
}

/// Render the top bar.
#[must_use]
pub fn view<'a, Message: 'a + Clone>(state: &'a TopBarState) -> Element<'a, Message> {
    let start = zone(state.start_label.as_deref(), Pane::Start);
    let pinned = zone_of_many(&state.pinned_labels, Pane::Pinned, 8);
    let tasklist = zone(state.tasklist_label.as_deref(), Pane::Tasklist);
    let cluster = zone(state.cluster_label.as_deref(), Pane::Cluster);
    let tray = zone_of_many(&state.tray_labels, Pane::Tray, 6);
    let clock = zone(state.clock_label.as_deref(), Pane::Clock);

    container(
        row![
            start,
            Space::with_width(Length::Fixed(f32::from(ZONE_PADDING_X))),
            pinned,
            Space::with_width(Length::Fill),
            tasklist,
            Space::with_width(Length::Fill),
            cluster,
            Space::with_width(Length::Fixed(f32::from(ZONE_PADDING_X))),
            tray,
            Space::with_width(Length::Fixed(f32::from(ZONE_PADDING_X))),
            clock,
        ]
        .align_y(iced::Alignment::Center)
        .padding(Padding {
            top: 0.0,
            right: f32::from(ZONE_PADDING_X),
            bottom: 0.0,
            left: f32::from(ZONE_PADDING_X),
        }),
    )
    .width(Length::Fill)
    .height(Length::Fixed(f32::from(TOP_BAR_HEIGHT_PX)))
    .style(panel_surface)
    .into()
}

fn zone<'a, Message: 'a>(label: Option<&str>, pane: Pane) -> Element<'a, Message> {
    let label_str = label.unwrap_or("").to_string();
    let pane_label = pane.label().to_string();
    container(text(label_str).size(14))
        .padding(Padding {
            top: 4.0,
            right: f32::from(ZONE_PADDING_X / 2),
            bottom: 4.0,
            left: f32::from(ZONE_PADDING_X / 2),
        })
        .style(move |_theme: &Theme| zone_style(&pane_label))
        .into()
}

fn zone_of_many<'a, Message: 'a + Clone>(
    labels: &[String],
    pane: Pane,
    spacing: u16,
) -> Element<'a, Message> {
    let pane_label = pane.label().to_string();
    let mut row_widget = row![].spacing(spacing).align_y(iced::Alignment::Center);
    for label in labels {
        row_widget = row_widget.push(text(label.clone()).size(14));
    }
    container(row_widget)
        .padding(Padding {
            top: 4.0,
            right: f32::from(ZONE_PADDING_X / 2),
            bottom: 4.0,
            left: f32::from(ZONE_PADDING_X / 2),
        })
        .style(move |_theme: &Theme| zone_style(&pane_label))
        .into()
}

fn panel_surface(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(iced::Background::Color(Color {
            r: palette.background.base.color.r,
            g: palette.background.base.color.g,
            b: palette.background.base.color.b,
            a: 0.96,
        })),
        border: iced::Border {
            color: Color {
                r: palette.background.strong.color.r,
                g: palette.background.strong.color.g,
                b: palette.background.strong.color.b,
                a: 0.18,
            },
            width: 0.0,
            radius: 0.0.into(),
        },
        text_color: Some(palette.background.base.text),
        shadow: iced::Shadow::default(),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn zone_style(_pane_label: &str) -> container::Style {
    // Zones are visually weightless — they only border on hover,
    // which we'll wire in once we have per-zone interactivity (E.7+).
    container::Style::default()
}

/// Default clock label string used by [`TopBarState::demo`] and by
/// `App::view()` until the clock applet (E1.2.1, shipped) wires
/// into the panel host.
#[must_use]
pub fn default_clock_label() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format_clock(secs)
}

/// Pure-function clock renderer. Pulled out so tests can pin format
/// behavior without touching the system clock.
#[must_use]
pub fn format_clock(epoch_seconds: u64) -> String {
    // Howard-Hinnant civil-from-days for HH:MM display.
    let secs_in_day = epoch_seconds % 86_400;
    let hours = (secs_in_day / 3_600) as u32;
    let mins = ((secs_in_day % 3_600) / 60) as u32;
    format!("{hours:02}:{mins:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_bar_height_is_40px_per_1_1_0_lock() {
        assert_eq!(TOP_BAR_HEIGHT_PX, 40);
    }

    #[test]
    fn zone_padding_is_symmetric_12px() {
        assert_eq!(ZONE_PADDING_X, 12);
    }

    #[test]
    fn demo_state_populates_every_zone() {
        let state = TopBarState::demo();
        assert!(state.start_label.is_some());
        assert!(!state.pinned_labels.is_empty());
        assert!(state.tasklist_label.is_some());
        assert!(state.cluster_label.is_some());
        assert!(!state.tray_labels.is_empty());
        assert!(state.clock_label.is_some());
    }

    #[test]
    fn format_clock_pads_hours_and_minutes() {
        // 09:07 UTC = 9 * 3600 + 7 * 60 = 32820 seconds into the day.
        assert_eq!(format_clock(32_820), "09:07");
    }

    #[test]
    fn format_clock_handles_midnight() {
        assert_eq!(format_clock(0), "00:00");
    }

    #[test]
    fn format_clock_handles_last_minute_of_day() {
        // 23:59 = 23*3600 + 59*60 = 86340.
        assert_eq!(format_clock(86_340), "23:59");
    }

    #[test]
    fn format_clock_wraps_after_full_day() {
        // 24:00 boundary should wrap to 00:00.
        assert_eq!(format_clock(86_400), "00:00");
    }

    #[test]
    fn default_state_is_all_empty() {
        let state = TopBarState::default();
        assert!(state.start_label.is_none());
        assert!(state.pinned_labels.is_empty());
        assert!(state.tasklist_label.is_none());
        assert!(state.cluster_label.is_none());
        assert!(state.tray_labels.is_empty());
        assert!(state.clock_label.is_none());
    }

    #[test]
    fn view_renders_without_panic() {
        let state = TopBarState::demo();
        let _ = view::<crate::Message>(&state);
    }
}
