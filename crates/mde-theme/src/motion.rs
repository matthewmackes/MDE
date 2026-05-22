//! UX-9 — motion + dialog timing tokens.
//!
//! Centralizes every "how long does this take" constant in the
//! design system so animations across the workspace stay
//! coherent. Locks (UX-9 spec):
//!   * sidebar / panel mount transition — 180 ms ease-out
//!   * notification bell pulse — 2 s ease-in-out, max scale 1.15
//!   * tooltip fade-in delay — 120 ms
//!   * dialog mount fade — 180 ms (same easing as panel mount)
//!
//! The actual easing / interpolation lives in the consumer
//! (Iced subscription, GTK CSS, etc.); this module is the
//! durable contract for the *durations* + *parameters*.

use std::time::Duration;

/// Easing curve for a motion token. Consumers translate the
/// enum to their renderer's equivalent (CSS `cubic-bezier`,
/// Iced `iced::animation::Easing`, etc.).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Easing {
    /// Linear interpolation — no easing.
    Linear,
    /// Ease-out — fast start, slow end. Default for entrances
    /// (panels mounting, dialogs appearing).
    EaseOut,
    /// Ease-in — slow start, fast end. Default for exits.
    EaseIn,
    /// Ease-in-out — slow start + slow end. Default for
    /// continuous / looping animations (notification pulse).
    EaseInOut,
}

/// A single motion spec — duration + easing + optional
/// looping flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Motion {
    /// Total animation duration.
    pub duration: Duration,
    /// Easing curve.
    pub easing: Easing,
    /// `true` = animation loops indefinitely (pulse, spinner);
    /// `false` = single-shot (panel mount, dialog enter).
    pub looping: bool,
}

impl Motion {
    /// UX-9 (a) — sidebar panel mount transition. 180 ms
    /// ease-out, opacity 0→1 + translate-Y(4px→0).
    #[must_use]
    pub const fn panel_mount() -> Self {
        Self {
            duration: Duration::from_millis(180),
            easing: Easing::EaseOut,
            looping: false,
        }
    }

    /// UX-9 (c) — dialog mount fade. Same 180 ms ease-out as
    /// panel mount so the system reads as one motion vocabulary.
    #[must_use]
    pub const fn dialog_mount() -> Self {
        Self {
            duration: Duration::from_millis(180),
            easing: Easing::EaseOut,
            looping: false,
        }
    }

    /// UX-9 (b) — notification bell pulse. 2 s ease-in-out,
    /// looping. Max scale 1.15 (see [`PULSE_MAX_SCALE`]).
    #[must_use]
    pub const fn notification_pulse() -> Self {
        Self {
            duration: Duration::from_millis(2000),
            easing: Easing::EaseInOut,
            looping: true,
        }
    }

    /// UX-9 (d) — tooltip fade-in delay. 120 ms.
    #[must_use]
    pub const fn tooltip_fade() -> Self {
        Self {
            duration: Duration::from_millis(120),
            easing: Easing::EaseOut,
            looping: false,
        }
    }
}

/// UX-9 (b) — notification bell pulse maximum scale factor.
/// Component dimension, not density-scaled.
pub const PULSE_MAX_SCALE: f32 = 1.15;

/// UX-9 (a) — panel mount translate-Y start offset (px).
/// Component dimension, not density-scaled.
pub const PANEL_MOUNT_TRANSLATE_Y_PX: f32 = 4.0;

/// UX-9 (c) — dialog spec constants. Locked component
/// dimensions, not density-scaled per UX-24 sub-lock.
pub mod dialog {
    /// Maximum dialog width (px).
    pub const MAX_WIDTH: f32 = 480.0;
    /// Backdrop opacity (0.0 = transparent, 1.0 = opaque black).
    pub const BACKDROP_OPACITY: f32 = 0.50;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_mount_is_180_ms_ease_out() {
        let m = Motion::panel_mount();
        assert_eq!(m.duration, Duration::from_millis(180));
        assert_eq!(m.easing, Easing::EaseOut);
        assert!(!m.looping);
    }

    #[test]
    fn notification_pulse_is_two_seconds_looping() {
        let m = Motion::notification_pulse();
        assert_eq!(m.duration, Duration::from_millis(2000));
        assert!(m.looping);
    }

    #[test]
    fn tooltip_fade_is_120_ms() {
        let m = Motion::tooltip_fade();
        assert_eq!(m.duration, Duration::from_millis(120));
    }

    #[test]
    fn dialog_mount_matches_panel_mount_duration() {
        assert_eq!(
            Motion::dialog_mount().duration,
            Motion::panel_mount().duration
        );
    }

    #[test]
    fn pulse_scale_locked_to_1_15() {
        assert!((PULSE_MAX_SCALE - 1.15).abs() < f32::EPSILON);
    }

    #[test]
    fn dialog_max_width_locked_to_480() {
        assert!((dialog::MAX_WIDTH - 480.0).abs() < f32::EPSILON);
    }

    #[test]
    fn dialog_backdrop_is_fifty_percent() {
        assert!((dialog::BACKDROP_OPACITY - 0.50).abs() < f32::EPSILON);
    }
}
