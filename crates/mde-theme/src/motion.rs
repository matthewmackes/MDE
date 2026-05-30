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

/// UX-9 (c) + CR-10 — dialog spec constants.
/// Locked component dimensions, not density-scaled per UX-24.
pub mod dialog {
    /// Maximum dialog width (px). Classic ChromeOS: 480 px.
    pub const MAX_WIDTH: f32 = 480.0;
    /// Backdrop opacity. CR-10 (2026-05-24) overrides UX-9 0.50 →
    /// 0.60 per the Classic ChromeOS 60 % black spec.
    pub const BACKDROP_OPACITY: f32 = 0.60;
    /// Title row height (px). Classic ChromeOS 48 px.
    pub const TITLE_ROW_HEIGHT: f32 = 48.0;
    /// Button row height (px). Classic ChromeOS 64 px.
    pub const BUTTON_ROW_HEIGHT: f32 = 64.0;
    /// Title font size (sp). Classic ChromeOS 18 sp weight 500.
    pub const TITLE_FONT_SIZE: f32 = 18.0;
    /// Horizontal inner padding (px). Classic ChromeOS 16 px.
    pub const H_PAD: f32 = 16.0;
    /// Gap between action buttons (px).
    pub const BUTTON_GAP: f32 = 8.0;
}

/// CR-10 — toast / notification chip constants.
/// Classic ChromeOS spec 2026-05-24.
pub mod toast {
    /// Fixed chip width (px).
    pub const WIDTH: f32 = 320.0;
    /// Auto-dismiss after this many milliseconds.
    pub const DISMISS_MS: u64 = 5000;
    /// Height of the bottom progress strip (px).
    pub const PROGRESS_HEIGHT: f32 = 2.0;
    /// Gap above the Shelf (px).
    pub const POSITION_GAP: f32 = 8.0;
}

/// ANIM-4 — list/stagger + skeleton + selection timing tokens.
/// Cite: motion-language.md §2.4, §2.6, §2.8, §2.9.
/// Locks: Q15 (capped-8 stagger), Q18 (selection slide),
/// Q19 (skeleton shimmer → crossfade).
pub mod list {
    /// Maximum number of items that stagger individually (Q15).
    /// Items at or beyond this index appear at the cap delay so
    /// long lists don't crawl. With step=20ms the spread is 0–140ms.
    pub const STAGGER_CAP: usize = 8;

    /// Per-item stagger step (ms). Item i gets delay
    /// `min(i, STAGGER_CAP - 1) * STAGGER_STEP_MS`.
    pub const STAGGER_STEP_MS: u32 = 20;

    /// Reveal fade-in duration for each staggered list item (ms).
    /// Shorter than the standard 150ms so staggered items feel crisp
    /// even at the tail of the cap.
    pub const STAGGER_REVEAL_MS: u32 = 120;

    /// Selection indicator slide duration (ms). Q18.
    /// Matches motion-language.md §2.6: 150ms ease-out.
    pub const SELECTION_SLIDE_MS: u32 = 150;

    /// Skeleton shimmer oscillation period (ms). Q19.
    /// One full sweep of the shimmer highlight across the placeholder.
    pub const SHIMMER_PERIOD_MS: u64 = 1200;

    /// Duration to crossfade from skeleton shimmer to loaded content
    /// (ms). Q19. Matches the standard 150ms transition.
    pub const SKELETON_CROSSFADE_MS: u32 = 150;
}

/// CR-10 — right-click context menu constants.
/// Classic ChromeOS spec 2026-05-24.
pub mod context_menu {
    /// Minimum menu width (px).
    pub const MIN_WIDTH: f32 = 220.0;
    /// Height of each non-separator row (px).
    pub const ROW_HEIGHT: f32 = 28.0;
    /// Keyboard-shortcut label font size (sp).
    pub const KBD_SIZE: f32 = 11.0;
    /// Primary label font size (sp).
    pub const LABEL_SIZE: f32 = 13.0;
    /// Left padding for the icon column (px).
    pub const ICON_L_PAD: f32 = 12.0;
    /// Left padding between icon and label (px).
    pub const LABEL_L_PAD: f32 = 8.0;
    /// Right padding for the kbd shortcut column (px).
    pub const KBD_R_PAD: f32 = 12.0;
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
    fn dialog_backdrop_is_sixty_percent() {
        // CR-10 Classic ChromeOS spec: 60 % black (was UX-9 50 %).
        assert!((dialog::BACKDROP_OPACITY - 0.60).abs() < f32::EPSILON);
    }

    #[test]
    fn dialog_title_row_is_48px_and_button_row_64px() {
        assert!((dialog::TITLE_ROW_HEIGHT - 48.0).abs() < f32::EPSILON);
        assert!((dialog::BUTTON_ROW_HEIGHT - 64.0).abs() < f32::EPSILON);
    }

    #[test]
    fn toast_width_is_320_and_dismiss_5s() {
        assert!((toast::WIDTH - 320.0).abs() < f32::EPSILON);
        assert_eq!(toast::DISMISS_MS, 5000);
    }

    #[test]
    fn context_menu_min_width_is_220_and_row_28() {
        assert!((context_menu::MIN_WIDTH - 220.0).abs() < f32::EPSILON);
        assert!((context_menu::ROW_HEIGHT - 28.0).abs() < f32::EPSILON);
    }

    #[test]
    fn list_stagger_cap_is_8_and_step_20ms() {
        // Q15 acceptance: capped at 8, 20ms step → 0..140ms spread.
        assert_eq!(list::STAGGER_CAP, 8);
        assert_eq!(list::STAGGER_STEP_MS, 20);
        let last_stagger_ms = (list::STAGGER_CAP as u32 - 1) * list::STAGGER_STEP_MS;
        assert_eq!(last_stagger_ms, 140);
    }

    #[test]
    fn list_selection_slide_matches_motion_language_spec() {
        // motion-language.md §2.6: selection underlay slides 150ms ease-out.
        assert_eq!(list::SELECTION_SLIDE_MS, 150);
    }

    #[test]
    fn list_shimmer_period_is_1200ms() {
        // Q19: shimmer sweeps once per 1200ms.
        assert_eq!(list::SHIMMER_PERIOD_MS, 1200);
    }
}
