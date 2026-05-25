//! Portal-3 — font + icon theme layer.
//!
//! ## Font stack
//!
//! | Role | Family | System package |
//! |------|--------|---------------|
//! | Primary | `Intel One Mono` | `intel-one-mono-fonts` |
//! | Icon fallback | `Symbols Nerd Font Mono` | `symbols-nerd-font-mono-fonts` |
//!
//! The RPM spec ships `Requires: intel-one-mono-fonts` so that fontconfig
//! resolves `Font::with_name(INTEL_ONE_MONO)` at runtime.  If the font is
//! absent (dev machine without the package) Iced degrades to its built-in
//! font gracefully.
//!
//! ## Carbon icon set
//!
//! `mde_theme::icons` is the canonical Carbon icon enum. Portal surfaces
//! import `mde_theme::icons::{Icon, IconSize}` and call [`icon_glyph`] to
//! get the Unicode fallback for a given semantic icon; Portal-4 onward
//! will switch the call sites to SVG bytes via UX-8.a.
//!
//! ## Nerd Glyph fallback
//!
//! `NERD_SYMBOLS_FONT` names the Symbols Nerd Font Mono family, which
//! provides icon codepoints (U+E000..U+F8FF + extensions) for any Carbon
//! icon that hasn't yet received real SVG bytes.  Iced's font cascade falls
//! through to this family when the primary font lacks a glyph.

use iced::Font;

/// Primary font family — Intel One Mono (monospace display + UI).
/// Deployed by `Requires: intel-one-mono-fonts` in the RPM spec.
pub const INTEL_ONE_MONO: &str = "Intel One Mono";

/// Secondary font family for icon glyphs — Symbols Nerd Font Mono.
/// Provides U+E000..U+F8FF Nerd glyph block for Carbon icon fallback.
/// Deployed by `Requires: symbols-nerd-font-mono-fonts` in the RPM spec.
pub const NERD_SYMBOLS_FONT: &str = "Symbols Nerd Font Mono";

/// Iced `Font` value for Intel One Mono (regular weight).
///
/// Pass to `Settings::default_font`; Iced resolves it via fontconfig.
pub const FONT_INTEL_ONE_MONO: Font = Font::with_name(INTEL_ONE_MONO);

/// Iced `Font` value for Symbols Nerd Font Mono.
///
/// Used by Carbon icon fallback rendering until UX-8.a ships real SVG bytes.
pub const FONT_NERD_SYMBOLS: Font = Font::with_name(NERD_SYMBOLS_FONT);

/// Resolve a Carbon icon to its `ResolvedIcon` (SVG bytes + Unicode fallback).
///
/// Delegates to `mde_theme::mde_icon` — the semantic mapping is
/// owned by `mde-theme` so call sites stay stable across Carbon SVG migrations.
/// Portal-4 nav buttons call this for their 20 px icon glyphs.
pub fn resolve_icon(icon: mde_theme::Icon, size: mde_theme::IconSize) -> mde_theme::ResolvedIcon {
    mde_theme::mde_icon(icon, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_font_name_is_intel_one_mono() {
        assert_eq!(INTEL_ONE_MONO, "Intel One Mono");
    }

    #[test]
    fn nerd_symbols_font_name_is_correct() {
        assert_eq!(NERD_SYMBOLS_FONT, "Symbols Nerd Font Mono");
    }

    #[test]
    fn font_intel_one_mono_constant_has_correct_name() {
        // Font::with_name stores the name as a &str; compare via debug
        // since iced::Font doesn't impl PartialEq on the name string.
        let dbg = format!("{FONT_INTEL_ONE_MONO:?}");
        assert!(dbg.contains("Intel One Mono"), "Font::with_name should carry the family name");
    }

    #[test]
    fn resolve_icon_delegates_to_mde_theme() {
        let resolved = resolve_icon(mde_theme::Icon::Fleet, mde_theme::IconSize::Nav);
        // Resolved icon has a Unicode fallback codepoint (non-empty).
        assert!(!resolved.fallback_glyph.is_empty(), "Carbon icon must have a Unicode fallback");
    }

    #[test]
    fn resolve_icon_carries_correct_size() {
        let resolved = resolve_icon(mde_theme::Icon::Settings, mde_theme::IconSize::Nav);
        assert_eq!(resolved.size, mde_theme::IconSize::Nav);
    }
}
