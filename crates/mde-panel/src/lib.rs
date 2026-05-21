//! mde-panel — Iced + libcosmic top bar + bottom dock for the
//! Mackes Desktop Environment.
//!
//! Phase E.1 lock (revised 2026-05-21):
//! - Ships **side-by-side** with the legacy GTK3 `mackes-panel`
//!   crate. Both binaries co-exist during the Phase E port; the
//!   spec eventually flips `/usr/bin/mackes-panel` to the
//!   `mde-panel` binary once parity is reached. This avoids
//!   regressing installed v2.0.x boxes mid-port.
//! - Builds on **raw Iced 0.13** with the same feature set as
//!   `mde-workbench` and `mde-files`, so the workspace dep tree
//!   resolves to a single Iced version. `libcosmic` integration
//!   stays optional — it lands at Phase E.1.3 if the
//!   cosmic-theme adapter justifies it; today the `mackes-theme`
//!   crate (E3.1) handles token parsing without cosmic-theme.
//! - **Wayland-first.** Phase E.2 wires
//!   `smithay-client-toolkit`'s wlr-layer-shell-v1 anchor. The
//!   skeleton renders as a standard Iced window in the meantime so
//!   the binary compiles + runs in dev.
//!
//! Source-file modules (`pub mod`) are added per-port in
//! Phase E.4 → E.29. The skeleton itself ships only the app
//! shell + the cross-cutting `Message`/`Pane` types.

#![forbid(unsafe_code)]

use iced::widget::{container, row, text};
use iced::{Alignment, Element, Length, Size, Task, Theme};

pub mod theme;

// ──────────────────────────────────────────────────────────────
// Public layout zones (Phase E lock)
// ──────────────────────────────────────────────────────────────

/// The six named layout zones of the MDE top-bar (1.1.0 Win10 lock).
///
/// Each port (E.4 - E.29) writes its widget into one of these zones;
/// the panel orchestrator owns the spatial composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pane {
    /// Left edge — Start button + admin-menu trigger (right-click).
    Start,
    /// Pinned-app strip immediately right of Start.
    Pinned,
    /// Running-window strip (tasklist hero).
    Tasklist,
    /// SPLIT / LAYOUT / WINDOW sway-IPC chips (E.4.1).
    Cluster,
    /// System tray row (bell + NM + mesh + audio + status).
    Tray,
    /// Date / time pill at far right.
    Clock,
}

impl Pane {
    /// Stable ordering of zones, left → right.
    #[must_use]
    pub const fn ordered() -> [Pane; 6] {
        [
            Pane::Start,
            Pane::Pinned,
            Pane::Tasklist,
            Pane::Cluster,
            Pane::Tray,
            Pane::Clock,
        ]
    }

    /// Display label used in test fixtures + accessibility metadata.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Pane::Start => "Start",
            Pane::Pinned => "Pinned apps",
            Pane::Tasklist => "Running windows",
            Pane::Cluster => "Layout cluster",
            Pane::Tray => "System tray",
            Pane::Clock => "Clock",
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Top-level reducer messages
// ──────────────────────────────────────────────────────────────

/// Reducer messages for the panel application.
///
/// Phase E.1.2 ships the no-op variant set; per-port submessages
/// are added as their tasks land.
#[derive(Debug, Clone)]
pub enum Message {
    /// No-op placeholder — keeps the variant set non-empty so Iced's
    /// pattern matching stays exhaustive.
    Noop,
    /// 1-second tick used by clock + battery + watermark refresh.
    /// Subscription wiring lands at E.17.
    Tick,
}

// ──────────────────────────────────────────────────────────────
// Application state
// ──────────────────────────────────────────────────────────────

/// Panel application state.
///
/// Phase E.1.2 skeleton: empty state container. Per-pane state
/// lands per-port (E.4 - E.29).
#[derive(Debug, Default)]
pub struct App {
    /// Counts how many `Tick` messages have been received — used to
    /// confirm the subscription is wired in tests.
    ticks: u64,
}

impl App {
    /// Launch the panel via the Iced runtime.
    ///
    /// Phase E.2 wraps this with a layer-shell anchor (bottom edge,
    /// 40 px height, exclusive zone).
    pub fn run() -> iced::Result {
        iced::application(Self::title, Self::update, Self::view)
            .theme(Self::theme)
            .window_size(Size::new(1920.0, 40.0))
            .run()
    }

    fn title(&self) -> String {
        "mde-panel".to_string()
    }

    #[allow(clippy::unused_self)]
    fn theme(&self) -> Theme {
        // Phase E.1.3 — load tokens.css if available, fall back to
        // Iced::Theme::Dark when not (dev builds w/o the install
        // tree).
        theme::load_theme()
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Noop => {}
            Message::Tick => {
                self.ticks = self.ticks.saturating_add(1);
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let placeholder = text(format!(
            "mde-panel · Phase E.1 skeleton · ticks={}",
            self.ticks
        ));
        container(
            row![placeholder]
                .spacing(8)
                .align_y(Alignment::Center)
                .padding(8),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_ordering_has_six_distinct_zones() {
        let panes = Pane::ordered();
        assert_eq!(panes.len(), 6);
        for (i, a) in panes.iter().enumerate() {
            for (j, b) in panes.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn pane_labels_match_lock() {
        assert_eq!(Pane::Start.label(), "Start");
        assert_eq!(Pane::Pinned.label(), "Pinned apps");
        assert_eq!(Pane::Tasklist.label(), "Running windows");
        assert_eq!(Pane::Cluster.label(), "Layout cluster");
        assert_eq!(Pane::Tray.label(), "System tray");
        assert_eq!(Pane::Clock.label(), "Clock");
    }

    #[test]
    fn pane_is_copy_and_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for p in Pane::ordered() {
            set.insert(p);
        }
        assert_eq!(set.len(), 6);
    }

    #[test]
    fn app_default_is_initial_state() {
        let app = App::default();
        assert_eq!(app.ticks, 0);
    }

    #[test]
    fn tick_increments_counter() {
        let mut app = App::default();
        let _ = app.update(Message::Tick);
        assert_eq!(app.ticks, 1);
        let _ = app.update(Message::Tick);
        assert_eq!(app.ticks, 2);
    }

    #[test]
    fn noop_is_idempotent() {
        let mut app = App::default();
        app.ticks = 7;
        let _ = app.update(Message::Noop);
        assert_eq!(app.ticks, 7);
    }

    #[test]
    fn tick_counter_saturates_at_u64_max() {
        let mut app = App::default();
        app.ticks = u64::MAX;
        let _ = app.update(Message::Tick);
        assert_eq!(app.ticks, u64::MAX);
    }
}
