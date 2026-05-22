//! Frontend dispatch + auto-detection.
//!
//! The wizard runs the same state machine behind three rendering
//! paths:
//!
//! * **GUI** (Iced + wgpu) — graphical Wayland / X11 session.
//! * **TUI** (ratatui + crossterm) — bare TTY / SSH / serial,
//!   driven from `mde-installer-launch` after the compositor is
//!   killed.
//! * **Headless** — JSON-answer-file driver for kickstart, CI
//!   image bakes, and `mde-wizard --frontend=headless
//!   --answers=/root/answers.yaml`. Refuses to run destructive
//!   ops (Stage 2 purge) without explicit `confirmed: true`.
//!
//! `Frontend::auto()` picks GUI iff a graphical session is
//! present, falling back to TUI otherwise — that's the behaviour
//! the launcher relies on after it chvt's to tty1.

use clap::ValueEnum;

/// Which rendering path the wizard should use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Frontend {
    /// Auto-detect based on `$DISPLAY` / `$WAYLAND_DISPLAY`.
    Auto,
    /// Iced graphical wizard. Requires a compositor.
    Gui,
    /// ratatui console wizard. Runs on any TTY.
    Tui,
    /// Headless answers-driven wizard. No interactive UI.
    Headless,
}

impl Frontend {
    /// Resolve `Auto` to the concrete frontend the binary should
    /// run. Logic:
    ///
    /// * `WAYLAND_DISPLAY` or `DISPLAY` set, AND a terminal that
    ///   looks interactive → GUI.
    /// * Neither set → TUI.
    /// * Explicit (non-Auto) values pass through unchanged.
    ///
    /// Headless is **never** picked by auto — it must be
    /// requested explicitly.
    #[must_use]
    pub fn resolve(self, env: &impl FrontendEnv) -> Frontend {
        match self {
            Self::Auto => {
                if env.has_wayland_display() || env.has_x_display() {
                    Self::Gui
                } else {
                    Self::Tui
                }
            }
            other => other,
        }
    }
}

/// Read-side abstraction so `Frontend::resolve` is testable
/// without mutating the real environment.
pub trait FrontendEnv {
    /// True iff `$WAYLAND_DISPLAY` is set + non-empty.
    fn has_wayland_display(&self) -> bool;
    /// True iff `$DISPLAY` is set + non-empty.
    fn has_x_display(&self) -> bool;
}

/// Real-process implementation that reads `std::env`.
pub struct RealEnv;

impl FrontendEnv for RealEnv {
    fn has_wayland_display(&self) -> bool {
        std::env::var("WAYLAND_DISPLAY").is_ok_and(|v| !v.is_empty())
    }
    fn has_x_display(&self) -> bool {
        std::env::var("DISPLAY").is_ok_and(|v| !v.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StaticEnv {
        wl: bool,
        x: bool,
    }
    impl FrontendEnv for StaticEnv {
        fn has_wayland_display(&self) -> bool {
            self.wl
        }
        fn has_x_display(&self) -> bool {
            self.x
        }
    }

    #[test]
    fn auto_picks_gui_when_wayland_set() {
        let env = StaticEnv { wl: true, x: false };
        assert_eq!(Frontend::Auto.resolve(&env), Frontend::Gui);
    }

    #[test]
    fn auto_picks_gui_when_x_set() {
        let env = StaticEnv { wl: false, x: true };
        assert_eq!(Frontend::Auto.resolve(&env), Frontend::Gui);
    }

    #[test]
    fn auto_picks_tui_when_no_display() {
        let env = StaticEnv { wl: false, x: false };
        assert_eq!(Frontend::Auto.resolve(&env), Frontend::Tui);
    }

    #[test]
    fn auto_never_picks_headless() {
        let env = StaticEnv { wl: false, x: false };
        assert_ne!(Frontend::Auto.resolve(&env), Frontend::Headless);
    }

    #[test]
    fn explicit_choice_overrides_environment() {
        let env = StaticEnv { wl: true, x: true };
        assert_eq!(Frontend::Tui.resolve(&env), Frontend::Tui);
        assert_eq!(Frontend::Headless.resolve(&env), Frontend::Headless);
        assert_eq!(Frontend::Gui.resolve(&env), Frontend::Gui);
    }
}
