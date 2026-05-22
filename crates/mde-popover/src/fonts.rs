//! Shared font-fallback loader for every popover.
//!
//! Reads the system Noto Emoji / Symbola / NotoSansSymbols2 fonts at
//! boot so the popover surfaces have the same glyph coverage as the
//! panel itself. Missing fonts are silently skipped.

/// System font paths the popover host tries to load in order. First
/// hit wins (cosmic-text falls back through the list for any glyph
/// the primary font lacks).
pub const FALLBACK_FONT_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/google-noto-emoji-fonts/NotoEmoji-Regular.ttf",
    "/usr/share/fonts/gdouros-symbola/Symbola.ttf",
    "/usr/share/fonts/google-noto/NotoSansSymbols2-Regular.ttf",
];

/// Read every available fallback font into memory as `Cow::Owned`
/// bytes that Iced's `Settings::fonts` accepts.
#[must_use]
pub fn load_fallback_fonts() -> Vec<std::borrow::Cow<'static, [u8]>> {
    let mut out = Vec::new();
    for path in FALLBACK_FONT_CANDIDATES {
        if let Ok(bytes) = std::fs::read(path) {
            out.push(std::borrow::Cow::Owned(bytes));
        }
    }
    out
}
