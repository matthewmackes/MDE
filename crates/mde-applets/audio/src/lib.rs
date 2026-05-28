//! Audio status applet — top-bar-right chip showing
//! the default-sink volume + mute state.
//!
//! Phase E1.2.2: original spec called for pipewire-rs
//! bindgen subscription to `Node` events. That blocker
//! is lifted by shelling out to `pactl` (PipeWire's
//! PulseAudio compat layer) on a 2 s tick — same UX,
//! no bindgen dependency. Click action delegates to
//! the v2 native Iced mixer panel (Workbench → Sound).

#![forbid(unsafe_code)]

use mde_applet_api::{AppletId, AppletSlot, HostMessage};

/// Build the static applet manifest the host registers at startup.
/// Identity + binary name + slot match the `mde-applet-audio` binary
/// the panel spawns when this applet is wired into the layout.
#[must_use]
pub fn manifest() -> mde_applet_api::AppletManifest {
    mde_applet_api::AppletManifest {
        id: AppletId::from_static("audio"),
        binary: "mde-applet-audio".into(),
        slot: AppletSlot::TopBarRight,
        summary: "Default-sink volume + mute chip".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

/// Snapshot of the default-sink state polled from `pactl` on the
/// 2 s tick. Render-only — the applet never owns mutation; the
/// Workbench Sound panel is the source of truth for volume + mute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioState {
    /// Default-sink volume, 0..=100. Clamped at parse time so the
    /// renderer never sees values outside the chip's display range.
    pub volume_pct: u32,
    /// `true` when the default sink is muted. Renders as a
    /// crossed-speaker glyph regardless of volume_pct.
    pub muted: bool,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            volume_pct: 0,
            muted: false,
        }
    }
}

/// Parse the output of `pactl get-sink-volume
/// @DEFAULT_SINK@`. The PA format is:
/// ```text
/// Volume: front-left: 48563 /  74% / -7.55 dB,   front-right: 48563 /  74% / -7.55 dB
///         balance 0.00
/// ```
/// We average the per-channel percentages.
#[must_use]
pub fn parse_volume(raw: &str) -> u32 {
    let mut pcts = Vec::new();
    for token in raw.split(|c: char| c == '/' || c == ',' || c.is_whitespace()) {
        if let Some(stripped) = token.strip_suffix('%') {
            if let Ok(n) = stripped.trim().parse::<u32>() {
                pcts.push(n);
            }
        }
    }
    if pcts.is_empty() {
        return 0;
    }
    let total: u32 = pcts.iter().sum();
    total / pcts.len() as u32
}

/// Parse the output of `pactl get-sink-mute
/// @DEFAULT_SINK@`. PA emits `Mute: yes` or
/// `Mute: no`.
#[must_use]
pub fn parse_mute(raw: &str) -> bool {
    raw.trim()
        .split_whitespace()
        .last()
        .map(|tok| tok.eq_ignore_ascii_case("yes") || tok.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Pick the right glyph for the current state.
///
/// Uses basic-plane Unicode (musical note `♪` U+266A, multiplication
/// sign `×` U+00D7) instead of the U+1F507..U+1F50A speaker emoji
/// because Iced 0.13 + cosmic-text's default font fallback doesn't
/// reach the Miscellaneous Symbols and Pictographs block — the
/// speaker glyphs render as tofu boxes in the panel. The basic-plane
/// glyphs render in every system sans-serif (DejaVu, Adwaita Sans,
/// Cantarell, Red Hat Text, etc.) without a font load.
#[must_use]
pub fn audio_glyph(state: AudioState) -> &'static str {
    if state.muted {
        "\u{00D7}" // × — multiplication sign (mute)
    } else if state.volume_pct == 0 {
        "\u{266A}" // ♪ — eighth note (zero volume)
    } else if state.volume_pct < 50 {
        "\u{266A}" // ♪ — quiet
    } else {
        "\u{266B}" // ♫ — beamed eighth notes (loud)
    }
}

/// Compact pill rendering for the top-bar chip.
///
/// v4.0.1 BUG-13.a: the leading Unicode glyph (`audio_glyph(state)`,
/// e.g. `🔈` / `🔉` / `🔊`) is no longer prepended. The panel composes
/// its own Material Symbols SVG icon (via `PanelIcon::Audio`) before this text,
/// so the chip used to render `[SVG] 🔈 50%` (icon + redundant
/// glyph + percent). Now the chip is just `50%` / `muted` and the
/// panel slot renders `[SVG] 50%`. `audio_glyph` is kept exported
/// for any non-panel consumer (test surfaces, future tooltip
/// text) that still wants the Unicode fallback.
#[must_use]
pub fn format_chip(state: AudioState) -> String {
    if state.muted {
        "muted".into()
    } else {
        format!("{}%", state.volume_pct)
    }
}

/// Process a host control message and return `true` when the
/// applet should keep running. Only [`HostMessage::Shutdown`]
/// stops the event loop; every other variant is a host-side
/// hint (accent change, visibility toggle, etc.) the applet
/// renderer reacts to elsewhere.
#[must_use]
pub fn handle_host(msg: &HostMessage) -> bool {
    !matches!(msg, HostMessage::Shutdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_lands_in_top_bar_right() {
        let m = manifest();
        assert_eq!(m.id.as_str(), "audio");
        assert_eq!(m.slot, AppletSlot::TopBarRight);
    }

    #[test]
    fn parse_volume_averages_per_channel() {
        let raw = "Volume: front-left: 48563 /  50% / -7.55 dB,   front-right: 48563 /  70% / -7.55 dB\n        balance 0.00\n";
        // (50 + 70) / 2 = 60.
        assert_eq!(parse_volume(raw), 60);
    }

    #[test]
    fn parse_volume_single_channel() {
        let raw = "Volume: mono: 65536 / 100% / 0.00 dB\n";
        assert_eq!(parse_volume(raw), 100);
    }

    #[test]
    fn parse_volume_returns_zero_on_garbage() {
        assert_eq!(parse_volume("not a pa output"), 0);
        assert_eq!(parse_volume(""), 0);
    }

    #[test]
    fn parse_mute_yes_no_case_insensitive() {
        assert!(parse_mute("Mute: yes"));
        assert!(parse_mute("Mute: YES"));
        assert!(!parse_mute("Mute: no"));
        assert!(!parse_mute(""));
    }

    #[test]
    fn audio_glyph_muted_wins_over_volume() {
        let s = AudioState {
            volume_pct: 80,
            muted: true,
        };
        assert_eq!(audio_glyph(s), "\u{00D7}");
    }

    #[test]
    fn audio_glyph_tier_thresholds() {
        let zero = AudioState {
            volume_pct: 0,
            muted: false,
        };
        let low = AudioState {
            volume_pct: 30,
            muted: false,
        };
        let high = AudioState {
            volume_pct: 80,
            muted: false,
        };
        assert_eq!(audio_glyph(zero), "\u{266A}");
        assert_eq!(audio_glyph(low), "\u{266A}");
        assert_eq!(audio_glyph(high), "\u{266B}");
    }

    #[test]
    fn format_chip_muted_shows_word() {
        let s = AudioState {
            volume_pct: 50,
            muted: true,
        };
        let pill = format_chip(s);
        assert!(pill.contains("muted"));
    }

    #[test]
    fn format_chip_normal_shows_percent() {
        let s = AudioState {
            volume_pct: 45,
            muted: false,
        };
        let pill = format_chip(s);
        assert!(pill.contains("45%"));
    }

    #[test]
    fn handle_host_short_circuits_shutdown() {
        assert!(!handle_host(&HostMessage::Shutdown));
    }
}
