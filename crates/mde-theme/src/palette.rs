//! Color palette tokens. Locks: Q2 (accent), Q3 (charcoal),
//! Q4 (4 elevation tiers), Q5 (light theme ships in v2.2),
//! Q7 (adaptive borders). See `docs/design/visual-identity.md`
//! § 2 for the rationale and the full table.

use crate::color::Rgba;
use crate::theme::Theme;

/// A complete palette for one theme. All eight tokens are
/// guaranteed populated. Color picks come from the lock survey;
/// adjust at survey time, not at call sites.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// Lowest surface in the elevation stack. Dark: `#1d1d1f`
    /// (Q3 Apple-charcoal). Light: `#f5f5f7`.
    pub background: Rgba,
    /// Standard surface — cards, panels, sidebars.
    pub surface: Rgba,
    /// Raised surface — modals, popovers, command palette.
    pub raised: Rgba,
    /// Overlay surface — tooltips, dropdown menus.
    pub overlay: Rgba,
    /// Single accent — indigo `#5b6af5` (Q2). Same in both
    /// themes by design (single restrained accent).
    pub accent: Rgba,
    /// Hairline border in dark mode; 1 px solid border in light
    /// mode (Q7 adaptive).
    pub border: Rgba,
    /// Primary text color. Dark: near-white. Light: near-black.
    pub text: Rgba,
    /// Muted / secondary text color.
    pub text_muted: Rgba,
}

impl Palette {
    /// Resolve the palette for a given theme.
    pub const fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self::dark(),
            Theme::Light => Self::light(),
        }
    }

    /// Dark-theme palette (default in v1.x; one of two in v2.2).
    pub const fn dark() -> Self {
        Self {
            background: Rgba::rgb(0x1d, 0x1d, 0x1f),
            surface: Rgba::rgb(0x2a, 0x2a, 0x2c),
            raised: Rgba::rgb(0x38, 0x38, 0x3a),
            overlay: Rgba::rgb(0x48, 0x48, 0x4a),
            accent: Rgba::rgb(0x5b, 0x6a, 0xf5),
            // Hairline @ ~8% white per § 2.
            border: Rgba::rgba(0xff, 0xff, 0xff, 0.08),
            // ~92% white — clears WCAG AAA against background.
            text: Rgba::rgba(0xff, 0xff, 0xff, 0.92),
            text_muted: Rgba::rgba(0xff, 0xff, 0xff, 0.55),
        }
    }

    /// Light-theme palette. Ships co-equal with dark in v2.2
    /// (Q5 + FU-2 full parity).
    pub const fn light() -> Self {
        Self {
            background: Rgba::rgb(0xf5, 0xf5, 0xf7),
            surface: Rgba::rgb(0xff, 0xff, 0xff),
            raised: Rgba::rgb(0xf0, 0xf0, 0xf2),
            overlay: Rgba::rgb(0xe5, 0xe5, 0xe7),
            accent: Rgba::rgb(0x5b, 0x6a, 0xf5),
            // 1 px solid border @ ~12% black per § 2.
            border: Rgba::rgba(0x00, 0x00, 0x00, 0.12),
            text: Rgba::rgba(0x00, 0x00, 0x00, 0.88),
            text_muted: Rgba::rgba(0x00, 0x00, 0x00, 0.55),
        }
    }

    /// Translucent indigo wash used for hover states (Q8).
    /// Returns the accent at 8% opacity.
    pub fn hover_tint(&self) -> Rgba {
        self.accent.with_alpha(0.08)
    }

    /// Active (mouse-down) state — accent at 12% opacity.
    pub fn active_tint(&self) -> Rgba {
        self.accent.with_alpha(0.12)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accent_matches_q2_lock() {
        let p = Palette::dark();
        assert_eq!(p.accent.r, 0x5b);
        assert_eq!(p.accent.g, 0x6a);
        assert_eq!(p.accent.b, 0xf5);
    }

    #[test]
    fn accent_is_identical_in_both_themes() {
        // Q2: single restrained accent, same across themes.
        assert_eq!(Palette::dark().accent, Palette::light().accent);
    }

    #[test]
    fn dark_background_matches_q3_charcoal() {
        let bg = Palette::dark().background;
        assert_eq!((bg.r, bg.g, bg.b), (0x1d, 0x1d, 0x1f));
    }

    #[test]
    fn border_is_adaptive_per_q7() {
        // Hairline (alpha ≈ 0.08) in dark; near-solid in light.
        assert!(Palette::dark().border.a < 0.2);
        assert!(Palette::light().border.a >= 0.10);
    }

    #[test]
    fn hover_tint_uses_accent_at_8pct() {
        let p = Palette::dark();
        let h = p.hover_tint();
        assert_eq!(h.r, p.accent.r);
        assert!((h.a - 0.08).abs() < 0.001);
    }
}
