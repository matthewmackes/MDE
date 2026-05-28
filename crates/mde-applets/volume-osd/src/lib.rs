//! Volume OSD — transient bottom-center overlay shown on
//! volume-key press.
//!
//! Phase E2.1: paired with a keybinding in sway config
//! that pipes the new volume percent through stdin.
//! Renders a horizontal progress bar that fades after
//! 1500 ms (timing handled host-side; this applet just
//! emits the rendered frame).

#![forbid(unsafe_code)]

use mde_applet_api::{AppletId, AppletSlot, HostMessage};

/// OSD bar width in characters (matches the v1.x panel's
/// drawer-osd render width).
pub const BAR_WIDTH: usize = 20;

/// Build the static applet manifest the host registers at
/// startup. Slot = Overlay because the OSD renders on the
/// wlr-layer-shell overlay layer in response to volume-key
/// events.
#[must_use]
pub fn manifest() -> mde_applet_api::AppletManifest {
    mde_applet_api::AppletManifest {
        id: AppletId::from_static("volume-osd"),
        binary: "mde-applet-volume-osd".into(),
        slot: AppletSlot::Overlay,
        summary: "Volume OSD overlay".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

/// Glyph for a volume state.
#[must_use]
pub fn volume_glyph(pct: u32, muted: bool) -> &'static str {
    if muted {
        "\u{1F507}" // muted speaker
    } else if pct == 0 {
        "\u{1F508}" // speaker (no waves)
    } else if pct < 50 {
        "\u{1F509}" // speaker with one wave
    } else {
        "\u{1F50A}" // speaker with three waves
    }
}

/// Render the OSD bar: `<glyph>  ████████░░░░░░░░░░  <pct>%`.
/// `pct` is clamped to 0..=150 (the PA range matching the
/// sound-panel slider).
#[must_use]
pub fn format_osd(pct: u32, muted: bool) -> String {
    let glyph = volume_glyph(pct, muted);
    let clamped = pct.min(150);
    // Scale to BAR_WIDTH cells.
    let filled = ((clamped as f32 / 150.0) * BAR_WIDTH as f32).round() as usize;
    let filled = filled.min(BAR_WIDTH);
    let bar: String = "\u{2588}".repeat(filled) + &"\u{2591}".repeat(BAR_WIDTH - filled);
    if muted {
        format!("{glyph}  {bar}  muted")
    } else {
        format!("{glyph}  {bar}  {clamped}%")
    }
}

/// Process a host control message and return `true` when the
/// applet should keep running. Only [`HostMessage::Shutdown`]
/// stops the event loop; every other variant is a host-side
/// hint the renderer reacts to elsewhere.
#[must_use]
pub fn handle_host(msg: &HostMessage) -> bool {
    !matches!(msg, HostMessage::Shutdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_lands_in_overlay_slot() {
        let m = manifest();
        assert_eq!(m.id.as_str(), "volume-osd");
        assert_eq!(m.slot, AppletSlot::Overlay);
    }

    #[test]
    fn bar_width_lock() {
        assert_eq!(BAR_WIDTH, 20);
    }

    #[test]
    fn volume_glyph_muted_overrides_pct() {
        assert_eq!(volume_glyph(50, true), "\u{1F507}");
        assert_eq!(volume_glyph(0, true), "\u{1F507}");
    }

    #[test]
    fn volume_glyph_zero_is_speaker_no_waves() {
        assert_eq!(volume_glyph(0, false), "\u{1F508}");
    }

    #[test]
    fn volume_glyph_low_under_50() {
        assert_eq!(volume_glyph(20, false), "\u{1F509}");
        assert_eq!(volume_glyph(49, false), "\u{1F509}");
    }

    #[test]
    fn volume_glyph_high_50_and_up() {
        assert_eq!(volume_glyph(50, false), "\u{1F50A}");
        assert_eq!(volume_glyph(100, false), "\u{1F50A}");
        assert_eq!(volume_glyph(150, false), "\u{1F50A}");
    }

    #[test]
    fn format_osd_renders_pct_in_text() {
        let s = format_osd(50, false);
        assert!(s.contains("50%"));
        assert!(s.contains("\u{2588}"));
    }

    #[test]
    fn format_osd_muted_uses_muted_label() {
        let s = format_osd(50, true);
        assert!(s.contains("muted"));
        assert!(!s.contains("50%"));
    }

    #[test]
    fn format_osd_clamps_above_150_to_150() {
        let s = format_osd(200, false);
        assert!(s.contains("150%"));
    }

    #[test]
    fn format_osd_zero_renders_empty_bar() {
        let s = format_osd(0, false);
        // 20 empty cells.
        assert_eq!(s.matches("\u{2591}").count(), BAR_WIDTH);
    }

    #[test]
    fn handle_host_short_circuits_shutdown() {
        assert!(!handle_host(&HostMessage::Shutdown));
    }
}
