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

    /// Dark-theme palette — Classic ChromeOS (pre-2022) tokens
    /// per CR-1 (2026-05-25). Source: `docs/design/
    /// chromeos-classic-spec.md` § Palette (dark mode default).
    pub const fn dark() -> Self {
        Self {
            // Page surface.
            background: Rgba::rgb(0x20, 0x21, 0x24),
            // Cards, popovers, hover surfaces.
            surface: Rgba::rgb(0x2d, 0x2e, 0x30),
            // Same as surface — the spec only carries 3 surface
            // tiers (background / raised / active). The `raised`
            // slot in this struct is the popover/modal tier,
            // which Classic ChromeOS draws at the same elevation
            // as cards. Keep both at #2d2e30 for now; a future
            // CR-* polish task can split them if needed.
            raised: Rgba::rgb(0x2d, 0x2e, 0x30),
            // Active/pressed surface, also the 1px divider color.
            overlay: Rgba::rgb(0x3c, 0x40, 0x43),
            // Q2 indigo — unchanged across the CR-1 swap.
            accent: Rgba::rgb(0x5b, 0x6a, 0xf5),
            // 1 px sharp divider per Classic ChromeOS — same hex
            // as the active-surface token (no alpha blending).
            border: Rgba::rgb(0x3c, 0x40, 0x43),
            // Text primary.
            text: Rgba::rgb(0xe8, 0xea, 0xed),
            // Text muted.
            text_muted: Rgba::rgb(0x9a, 0xa0, 0xa6),
        }
    }

    /// Light-theme palette — Classic ChromeOS pair per CR-1.
    /// Reserved for compile-time consumption; the light retrofit
    /// epic toggles the live binding once it ships. Source:
    /// `docs/design/chromeos-classic-spec.md` § Palette (light).
    pub const fn light() -> Self {
        Self {
            background: Rgba::rgb(0xf7, 0xf7, 0xf7),
            surface: Rgba::rgb(0xff, 0xff, 0xff),
            raised: Rgba::rgb(0xff, 0xff, 0xff),
            overlay: Rgba::rgb(0xe8, 0xea, 0xed),
            // Darker indigo pair for light mode (Classic ChromeOS
            // shifts the accent for AA contrast against white).
            accent: Rgba::rgb(0x40, 0x51, 0xd3),
            // Hard 1 px divider (no alpha) per Classic ChromeOS.
            border: Rgba::rgb(0xda, 0xdc, 0xe0),
            text: Rgba::rgb(0x1d, 0x1d, 0x1f),
            text_muted: Rgba::rgb(0x5f, 0x63, 0x68),
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
    fn accent_differs_between_themes_per_chromeos() {
        // CR-1 (2026-05-25): Classic ChromeOS shifts the accent
        // for light mode (#4051d3) to keep AA contrast against
        // white. Q2's "same accent across themes" rule is
        // grandfathered; live lock is per-theme.
        assert_ne!(Palette::dark().accent, Palette::light().accent);
        // Both stay in the indigo family.
        let d = Palette::dark().accent;
        let l = Palette::light().accent;
        assert!(d.b > d.r && d.b > d.g, "dark accent reads as indigo");
        assert!(l.b > l.r && l.b > l.g, "light accent reads as indigo");
    }

    #[test]
    fn dark_background_matches_chromeos_classic() {
        // CR-1 (2026-05-25): #202124 — Classic ChromeOS page
        // surface. Q3 charcoal #1d1d1f grandfathered.
        let bg = Palette::dark().background;
        assert_eq!((bg.r, bg.g, bg.b), (0x20, 0x21, 0x24));
    }

    #[test]
    fn border_is_hard_1px_divider_per_chromeos() {
        // CR-1: Classic ChromeOS uses hard 1 px dividers in
        // both themes — no alpha hairline. The token resolves
        // to the same value as `overlay` in dark mode (the
        // "surface active" tier).
        let d = Palette::dark();
        assert_eq!(d.border, d.overlay);
        let l = Palette::light();
        assert_eq!((l.border.r, l.border.g, l.border.b), (0xda, 0xdc, 0xe0));
        // Borders are solid (alpha 1.0), not hairline.
        assert!(d.border.a >= 0.95);
        assert!(l.border.a >= 0.95);
    }

    #[test]
    fn hover_tint_uses_accent_at_8pct() {
        let p = Palette::dark();
        let h = p.hover_tint();
        assert_eq!(h.r, p.accent.r);
        assert!((h.a - 0.08).abs() < 0.001);
    }
}
