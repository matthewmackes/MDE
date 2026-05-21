//! # MDE Design System
//!
//! The Rust-native design token surface for Mackes Desktop
//! Environment. Lock authority: `docs/design/visual-identity.md`
//! and `docs/PROJECT_WORKLIST.md` § UX Design Locks (50-Q survey,
//! 2026-05-21).
//!
//! ## Surface
//!
//! - [`color::Rgba`] — primitive RGBA color (no Iced runtime dep
//!   in the default build).
//! - [`palette`] — named color tokens for dark + light themes.
//! - [`spacing`] — the 12-step modular spacing scale (NFU-1).
//! - [`typography`] — type-scale sizes + font-stack constants.
//! - [`radii`] — corner-radius tokens.
//! - [`shadows`] — elevation shadow specs.
//! - [`Theme`] — Dark / Light enum.
//! - [`Density`] — Compact / Comfortable / Spacious enum
//!   (UX-15). UX-24 sub-lock: density scales spacing tokens only,
//!   never component dimensions.
//! - [`Tokens`] — resolved token set for a given (theme, density)
//!   pair. The single struct every consumer reads.
//!
//! ## Iced interop
//!
//! Behind the `iced` feature flag, this crate adds conversion
//! helpers (`Rgba::into_iced_color()`, `FontSize::px()`, etc.) so
//! Iced views can consume tokens directly. Without the feature
//! the crate is dependency-free and unit-testable in isolation.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod accessibility;
pub mod color;
pub mod components;
pub mod density;
pub mod palette;
pub mod prefs;
pub mod radii;
pub mod shadows;
pub mod spacing;
pub mod theme;
pub mod typography;

pub use accessibility::A11y;
pub use color::Rgba;
pub use components::EmptyState;
pub use density::Density;
pub use palette::Palette;
pub use prefs::Preferences;
pub use radii::Radii;
pub use shadows::Shadow;
pub use spacing::Space;
pub use theme::{Theme, Tokens};
pub use typography::{FontSize, FontWeight, LetterSpacing, TypeRole};

/// Convenience: resolved tokens for the most common case
/// (dark theme + comfortable density). Use in tests, demos, and
/// any surface where the user hasn't expressed a preference yet.
pub fn default_tokens() -> Tokens {
    Tokens::resolve(Theme::Dark, Density::Comfortable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_resolves_without_panic() {
        let t = default_tokens();
        assert_eq!(t.theme, Theme::Dark);
        assert_eq!(t.density, Density::Comfortable);
    }

    #[test]
    fn density_modes_resolve_distinct_scaled_spacings() {
        let c = Tokens::resolve(Theme::Dark, Density::Compact);
        let m = Tokens::resolve(Theme::Dark, Density::Comfortable);
        let s = Tokens::resolve(Theme::Dark, Density::Spacious);
        // UX-24: density scales spacings; component dimensions
        // (nav row, button height) are NOT density-scaled.
        assert!(c.space.md < m.space.md);
        assert!(m.space.md < s.space.md);
    }

    #[test]
    fn both_themes_resolve_with_full_palette() {
        let d = Tokens::resolve(Theme::Dark, Density::Comfortable);
        let l = Tokens::resolve(Theme::Light, Density::Comfortable);
        // Accent stays the same across themes (Q2 lock).
        assert_eq!(d.palette.accent, l.palette.accent);
        // Background diverges between themes.
        assert_ne!(d.palette.background, l.palette.background);
    }
}
