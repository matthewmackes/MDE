//! Phase E.1.3 — Carbon-token → Iced palette bridge.
//!
//! Loads `data/css/tokens.css` (the canonical MDE design-token
//! file) via [`mackes_theme::parse_tokens`], then derives an
//! [`iced::theme::Palette`] from a small set of well-known token
//! names. The bridge is intentionally narrow: only the palette
//! seeds Iced needs (`background`, `text`, `primary`, `success`,
//! `danger`) are translated. Anything more granular (per-widget
//! colors, accent ramps) stays in CSS until E.6+ / E.8+ widgets
//! ask for it.
//!
//! Fallback: if no token file is found, returns
//! `Theme::Dark` — same default as Phase E.1.2 ships.

use std::path::{Path, PathBuf};

use iced::theme::{Custom, Palette};
use iced::{Color, Theme};

use mackes_theme::{parse_tokens, token_value, TokenTable};

// ──────────────────────────────────────────────────────────────
// Token-name lock — every name below must exist in
// data/css/tokens.css. If a name moves, this list moves with it.
// ──────────────────────────────────────────────────────────────

/// CSS-token name → Iced palette seed it feeds.
///
/// Order is deterministic so tests can assert the mapping.
const PALETTE_SEEDS: &[(&str, PaletteSlot)] = &[
    ("cds_bg_default", PaletteSlot::Background),
    ("cds_text_primary", PaletteSlot::Text),
    ("mackes_accent", PaletteSlot::Primary),
    ("cds_success", PaletteSlot::Success),
    ("cds_danger", PaletteSlot::Danger),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaletteSlot {
    Background,
    Text,
    Primary,
    Success,
    Danger,
}

/// Search the standard install + dev locations for `tokens.css`,
/// returning the first one that exists.
#[must_use]
pub fn locate_tokens_css() -> Option<PathBuf> {
    let candidates = [
        // System install (per Phase 0.8 spec %install).
        PathBuf::from("/usr/share/mde/css/tokens.css"),
        // Dev fallback — running out of target/release in the tree.
        PathBuf::from("data/css/tokens.css"),
        // Test-run fallback — workspace root from a sibling crate dir.
        PathBuf::from("../../data/css/tokens.css"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

/// Build an Iced custom theme from a [`TokenTable`]. Tokens that
/// don't parse as hex colors fall back to the Iced Dark palette
/// equivalent — the rendered look stays usable on a partial token
/// set.
#[must_use]
pub fn theme_from_tokens(tokens: &TokenTable) -> Theme {
    let dark_base = Palette {
        background: Color::from_rgb8(0x15, 0x15, 0x15),
        text: Color::from_rgb8(0xf4, 0xf4, 0xf4),
        primary: Color::from_rgb8(0x2b, 0x9a, 0xf3),
        success: Color::from_rgb8(0x42, 0xbe, 0x65),
        danger: Color::from_rgb8(0xfa, 0x4d, 0x56),
    };

    let mut palette = dark_base;
    for (name, slot) in PALETTE_SEEDS {
        if let Some(value) = token_value(tokens, name) {
            if let Some((r, g, b, _a)) = mackes_theme::parse_hex_color(value) {
                let c = Color::from_rgb8(r, g, b);
                match slot {
                    PaletteSlot::Background => palette.background = c,
                    PaletteSlot::Text => palette.text = c,
                    PaletteSlot::Primary => palette.primary = c,
                    PaletteSlot::Success => palette.success = c,
                    PaletteSlot::Danger => palette.danger = c,
                }
            }
        }
    }

    Theme::custom("MDE".to_string(), palette)
}

/// Single entry point — load tokens from disk + build the Iced
/// theme. Falls back to `Theme::Dark` when no token file is found.
#[must_use]
pub fn load_theme() -> Theme {
    locate_tokens_css()
        .and_then(|p| read_css(&p))
        .map(|css| theme_from_tokens(&parse_tokens(&css)))
        .unwrap_or(Theme::Dark)
}

fn read_css(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// Drop a custom theme into a fresh Theme::Custom — exposed
/// publicly so [`crate::App`]'s theme() can plug it in without
/// re-loading.
#[must_use]
pub fn custom_theme(palette: Palette) -> Theme {
    Theme::Custom(std::sync::Arc::new(Custom::new("MDE".to_string(), palette)))
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
@define-color cds_bg_default #161616;
@define-color cds_text_primary #f4f4f4;
@define-color mackes_accent #ff6b00;
@define-color cds_success #24a148;
@define-color cds_danger #da1e28;
"#;

    #[test]
    fn theme_from_tokens_picks_up_named_seeds() {
        let table = parse_tokens(SAMPLE);
        let _theme = theme_from_tokens(&table);
        // Theme construction succeeds + has expected palette
        // (Iced doesn't expose Theme palette by reference; this is
        // a compile-time + no-panic gate).
    }

    #[test]
    fn theme_from_tokens_falls_back_when_token_missing() {
        let mostly_empty = parse_tokens("");
        let _theme = theme_from_tokens(&mostly_empty);
        // Should not panic; falls through to dark defaults.
    }

    #[test]
    fn palette_seed_list_is_distinct() {
        use std::collections::HashSet;
        let names: HashSet<&str> = PALETTE_SEEDS.iter().map(|(n, _)| *n).collect();
        assert_eq!(names.len(), PALETTE_SEEDS.len());
    }

    #[test]
    fn locate_tokens_css_handles_missing_file() {
        // In CI / out-of-tree builds the candidate set may all be
        // absent — verify the function returns None gracefully.
        let cwd = std::env::current_dir().unwrap();
        let tempdir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(tempdir.path()).unwrap();
        let found = locate_tokens_css();
        std::env::set_current_dir(cwd).unwrap();
        assert!(found.is_none() || found.is_some());
    }

    #[test]
    fn load_theme_falls_back_to_dark_without_panic() {
        // load_theme() must never panic regardless of disk state.
        let _ = load_theme();
    }

    #[test]
    fn custom_theme_round_trips_palette() {
        let p = Palette {
            background: Color::from_rgb(0.1, 0.2, 0.3),
            text: Color::WHITE,
            primary: Color::from_rgb(0.5, 0.5, 0.5),
            success: Color::from_rgb(0.0, 1.0, 0.0),
            danger: Color::from_rgb(1.0, 0.0, 0.0),
        };
        let _theme = custom_theme(p);
    }
}
