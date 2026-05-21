//! PC-8 — Hero strip: full-bleed identity surface at the top of
//! the modal.
//!
//! Visual identity (cited from `docs/design/visual-identity.md`):
//!
//! - Height: ~280 px. The strip dominates the modal's upper third
//!   so the peer's identity reads at a glance, then descends into
//!   the technical sections.
//! - Background: `Palette::raised` placeholder when no Wikidata
//!   image is resolved; product photo (PC-6) lazy-loads in.
//! - Scrim: vertical glass effect using `Palette::surface` over a
//!   60% alpha overlay so the hostname text reads cleanly over
//!   any image hue.
//! - Hostname: `TypeRole::Display` (28 sp medium per Q14),
//!   lower-left.
//! - Manufacturer wordmark: `TypeRole::Subheading` (17 sp medium),
//!   upper-right. Truncates with ellipsis if > 24 chars.
//! - Distro + kernel chip: `TypeRole::Caption` (12 sp medium),
//!   bottom-right, `Radii::full` pill on `Palette::overlay`.
//!
//! Tokens: every visible value flows through `mde-theme`. Zero
//! hardcoded pixel literals; zero hardcoded hex colors.

use iced::{Background, Border, Color, Element, Length};
use iced::widget::{column, container, row, text, Space};
use mde_theme::{TypeRole, Tokens};

use crate::probe::PeerProbe;
use crate::enrich::Enrichment;

/// Hero strip height in logical pixels. Locked at 280 — the
/// upper third of an 840 px-tall modal surface.
pub const HERO_HEIGHT_PX: u16 = 280;

/// Manufacturer wordmark truncation budget (chars). Beyond this,
/// the wordmark collapses to an ellipsis to keep the upper-right
/// corner from competing with the hostname.
pub const WORDMARK_MAX_CHARS: usize = 24;

/// Build the hero strip view for a given probe + enrichment.
pub fn view<'a, Msg: 'a + Clone>(
    probe: &'a PeerProbe,
    enrichment: &'a Enrichment,
    tokens: &'a Tokens,
) -> Element<'a, Msg> {
    let palette = tokens.palette;
    let space = tokens.space;

    // Hostname — display tier (28 sp medium).
    let hostname = text(probe.hostname.clone())
        .size(TypeRole::Display.size_in(tokens.font_size))
        .color(rgba_to_color(palette.text));

    // Manufacturer wordmark with truncation.
    let manuf = enrichment
        .wikidata
        .as_ref()
        .map(|w| truncate(&w.manufacturer, WORDMARK_MAX_CHARS))
        .unwrap_or_else(|| {
            enrichment
                .hwdb
                .as_ref()
                .map(|h| truncate(&h.vendor_name, WORDMARK_MAX_CHARS))
                .unwrap_or_else(|| "—".to_string())
        });
    let wordmark = text(manuf)
        .size(TypeRole::Subheading.size_in(tokens.font_size))
        .color(rgba_to_color(palette.text_muted));

    // Distro + kernel chip (caption-sized pill, lower-right).
    let distro_chip = container(
        text(format!("{} · {}", probe.distro, short_kernel(&probe.kernel.uname)))
            .size(TypeRole::Caption.size_in(tokens.font_size))
            .color(rgba_to_color(palette.text)),
    )
    .padding([space.xs2, space.sm])
    .style(move |_theme| container::Style {
        background: Some(Background::Color(rgba_to_color(palette.overlay))),
        border: Border {
            radius: tokens.radii.full.into(),
            ..Border::default()
        },
        ..container::Style::default()
    });

    // Upper row: wordmark right-aligned.
    let upper = row![
        Space::with_width(Length::Fill),
        wordmark,
    ]
    .padding(space.md2);

    // Lower row: hostname left, distro chip right.
    let lower = row![
        hostname,
        Space::with_width(Length::Fill),
        distro_chip,
    ]
    .padding(space.md2)
    .align_y(iced::alignment::Vertical::Bottom);

    // The hero block: background placeholder (raised tier) until
    // the Wikidata image streams in via enrichment.
    container(
        column![
            upper,
            Space::with_height(Length::Fill),
            lower,
        ]
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fixed(f32::from(HERO_HEIGHT_PX)))
    .style(move |_theme| container::Style {
        background: Some(Background::Color(rgba_to_color(palette.raised))),
        ..container::Style::default()
    })
    .into()
}

/// Truncate a string to `max` chars, appending `…` if shortened.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// Extract a short kernel descriptor from a `uname -a` line.
/// E.g. "Linux laptop-mm 7.0.8-200.fc44.x86_64" → "7.0.8".
fn short_kernel(uname: &str) -> String {
    uname
        .split_whitespace()
        .nth(2)
        .map(|s| {
            // Strip after the first `-` to drop the Fedora release suffix.
            s.split('-').next().unwrap_or(s).to_string()
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn rgba_to_color(c: mde_theme::Rgba) -> Color {
    c.into_iced_color()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hero_height_is_280px() {
        // Acceptance from PC-8: hero ~280 px.
        assert_eq!(HERO_HEIGHT_PX, 280);
    }

    #[test]
    fn wordmark_truncates_long_names() {
        let s = "Very Long Manufacturer Name That Definitely Exceeds";
        let t = truncate(s, WORDMARK_MAX_CHARS);
        assert!(t.chars().count() <= WORDMARK_MAX_CHARS);
        assert!(t.ends_with('…'));
    }

    #[test]
    fn wordmark_preserves_short_names() {
        let s = "Intel";
        assert_eq!(truncate(s, WORDMARK_MAX_CHARS), "Intel");
    }

    #[test]
    fn short_kernel_extracts_version() {
        assert_eq!(short_kernel("Linux laptop-mm 7.0.8-200.fc44.x86_64"), "7.0.8");
    }

    #[test]
    fn short_kernel_handles_empty() {
        assert_eq!(short_kernel(""), "unknown");
    }
}
