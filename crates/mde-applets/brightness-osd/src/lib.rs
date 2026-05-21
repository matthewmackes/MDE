//! Brightness OSD — transient bottom-center overlay shown
//! on brightness-key press.
//!
//! Phase E2.2: same shape as the volume OSD (E2.1). Paired
//! with a sway-config keybinding that pipes the new
//! brightness percent through stdin.

#![forbid(unsafe_code)]

use mde_applet_api::{AppletId, AppletSlot, HostMessage};

pub const BAR_WIDTH: usize = 20;

#[must_use]
pub fn manifest() -> mde_applet_api::AppletManifest {
    mde_applet_api::AppletManifest {
        id: AppletId::from_static("brightness-osd"),
        binary: "mde-applet-brightness-osd".into(),
        slot: AppletSlot::Overlay,
        summary: "Brightness OSD overlay".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

/// Glyph for a brightness state.
#[must_use]
pub const fn brightness_glyph(pct: u32) -> &'static str {
    if pct < 33 {
        "\u{263C}" // small sun
    } else if pct < 66 {
        "\u{2600}" // medium sun
    } else {
        "\u{1F506}" // bright sun
    }
}

/// Render the OSD bar: `<glyph>  ████████░░░░░░░░░░  <pct>%`.
/// `pct` is clamped to 0..=100.
#[must_use]
pub fn format_osd(pct: u32) -> String {
    let glyph = brightness_glyph(pct);
    let clamped = pct.min(100);
    let filled = ((clamped as f32 / 100.0) * BAR_WIDTH as f32).round() as usize;
    let filled = filled.min(BAR_WIDTH);
    let bar: String = "\u{2588}".repeat(filled) + &"\u{2591}".repeat(BAR_WIDTH - filled);
    format!("{glyph}  {bar}  {clamped}%")
}

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
        assert_eq!(m.id.as_str(), "brightness-osd");
        assert_eq!(m.slot, AppletSlot::Overlay);
    }

    #[test]
    fn bar_width_lock() {
        assert_eq!(BAR_WIDTH, 20);
    }

    #[test]
    fn brightness_glyph_low_mid_high() {
        assert_eq!(brightness_glyph(0), "\u{263C}");
        assert_eq!(brightness_glyph(32), "\u{263C}");
        assert_eq!(brightness_glyph(33), "\u{2600}");
        assert_eq!(brightness_glyph(65), "\u{2600}");
        assert_eq!(brightness_glyph(66), "\u{1F506}");
        assert_eq!(brightness_glyph(100), "\u{1F506}");
    }

    #[test]
    fn format_osd_includes_pct_and_bar() {
        let s = format_osd(50);
        assert!(s.contains("50%"));
        assert!(s.contains("\u{2588}"));
    }

    #[test]
    fn format_osd_clamps_above_100() {
        let s = format_osd(255);
        assert!(s.contains("100%"));
    }

    #[test]
    fn format_osd_zero_renders_empty_bar() {
        let s = format_osd(0);
        assert_eq!(s.matches("\u{2591}").count(), BAR_WIDTH);
    }

    #[test]
    fn handle_host_short_circuits_shutdown() {
        assert!(!handle_host(&HostMessage::Shutdown));
    }
}
