//! Phase E.6.1 + E.6.2 — brightness + volume sliders.
//!
//! Pure-fn helpers + subprocess invocations for the drawer's
//! quick-action sliders. The Wayland-native backends are:
//!
//! - **Brightness:** `brightnessctl set N%` (DRM kernel API,
//!   X11+Wayland portable). 7-step granularity preserved from
//!   the 1.x version.
//! - **Volume:** `pactl set-sink-volume @DEFAULT_SINK@ N%` (the
//!   same pactl path the audio applet (E1.2.2) uses, so the
//!   workspace stays one volume-control story).
//!
//! Each helper exposes a pure `percent_step()` function that
//! returns the 7 stops (0/14/28/42/57/71/85/100) so the slider
//! widget can snap. The subprocess wrappers fall through cleanly
//! when the underlying binary is absent (returns `Err`, never
//! panics).

use std::process::Command;

/// 7-step granularity used by the drawer's brightness + volume
/// sliders (matches the 1.x Win10 layout lock).
pub const STOPS: [u8; 8] = [0, 14, 28, 42, 57, 71, 85, 100];

/// Snap an arbitrary 0..=100 percent to the nearest 7-step stop.
#[must_use]
pub fn snap_to_step(percent: u8) -> u8 {
    let pct = percent.min(100);
    *STOPS
        .iter()
        .min_by_key(|&&s| (i32::from(s) - i32::from(pct)).abs())
        .unwrap_or(&0)
}

/// Step index 0..=7 of an arbitrary percent (used by the
/// renderer for the 7-bar segmented display).
#[must_use]
pub fn step_index(percent: u8) -> usize {
    let snapped = snap_to_step(percent);
    STOPS.iter().position(|&s| s == snapped).unwrap_or(0)
}

// ──────────────────────────────────────────────────────────────
// Brightness
// ──────────────────────────────────────────────────────────────

/// Read current brightness via `brightnessctl get|max`. Returns
/// None on any subprocess error.
#[must_use]
pub fn read_brightness_percent() -> Option<u8> {
    let cur = Command::new("brightnessctl").args(["get"]).output().ok()?;
    let max = Command::new("brightnessctl").args(["max"]).output().ok()?;
    let cur: u64 = std::str::from_utf8(&cur.stdout).ok()?.trim().parse().ok()?;
    let max: u64 = std::str::from_utf8(&max.stdout).ok()?.trim().parse().ok()?;
    if max == 0 {
        return None;
    }
    Some(((cur * 100) / max).min(100) as u8)
}

/// Set brightness as a percent. Returns `Err` if `brightnessctl`
/// is unavailable.
pub fn set_brightness_percent(percent: u8) -> std::io::Result<()> {
    let pct = percent.min(100);
    let status = Command::new("brightnessctl")
        .args(["set", &format!("{pct}%")])
        .status()?;
    if !status.success() {
        return Err(std::io::Error::other(format!(
            "brightnessctl set {pct}% exited with {status}"
        )));
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Volume
// ──────────────────────────────────────────────────────────────

/// Read current default-sink volume via `pactl get-sink-volume`.
/// Returns None on any error.
#[must_use]
pub fn read_volume_percent() -> Option<u8> {
    let out = Command::new("pactl")
        .args(["get-sink-volume", "@DEFAULT_SINK@"])
        .output()
        .ok()?;
    let stdout = std::str::from_utf8(&out.stdout).ok()?;
    parse_pactl_volume(stdout)
}

/// Pure helper — parse `pactl get-sink-volume` output.
/// Format: `Volume: front-left: 65536 / 100% / 0.00 dB, front-right: ...`
/// Returns the average of left + right percentages.
#[must_use]
pub fn parse_pactl_volume(output: &str) -> Option<u8> {
    let mut percentages: Vec<u32> = Vec::new();
    for token in output.split_whitespace() {
        if let Some(stripped) = token.strip_suffix('%') {
            if let Ok(n) = stripped.parse::<u32>() {
                percentages.push(n);
            }
        }
    }
    if percentages.is_empty() {
        return None;
    }
    let avg = percentages.iter().sum::<u32>() / percentages.len() as u32;
    Some(avg.min(100) as u8)
}

/// Set default-sink volume.
pub fn set_volume_percent(percent: u8) -> std::io::Result<()> {
    let pct = percent.min(100);
    let status = Command::new("pactl")
        .args(["set-sink-volume", "@DEFAULT_SINK@", &format!("{pct}%")])
        .status()?;
    if !status.success() {
        return Err(std::io::Error::other(format!(
            "pactl set-sink-volume {pct}% exited with {status}"
        )));
    }
    Ok(())
}

/// Read default-sink mute state. Returns None on any error.
#[must_use]
pub fn read_mute() -> Option<bool> {
    let out = Command::new("pactl")
        .args(["get-sink-mute", "@DEFAULT_SINK@"])
        .output()
        .ok()?;
    let stdout = std::str::from_utf8(&out.stdout).ok()?;
    parse_pactl_mute(stdout)
}

/// Pure helper — parse `pactl get-sink-mute` output.
/// Format: `Mute: yes` or `Mute: no`.
#[must_use]
pub fn parse_pactl_mute(output: &str) -> Option<bool> {
    let lower = output.to_lowercase();
    if lower.contains("mute: yes") || lower.contains("mute: true") {
        Some(true)
    } else if lower.contains("mute: no") || lower.contains("mute: false") {
        Some(false)
    } else {
        None
    }
}

/// Toggle the default-sink mute state.
pub fn toggle_mute() -> std::io::Result<()> {
    let status = Command::new("pactl")
        .args(["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
        .status()?;
    if !status.success() {
        return Err(std::io::Error::other(format!(
            "pactl set-sink-mute toggle exited with {status}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_to_step_returns_a_known_stop() {
        for raw in 0..=100u8 {
            let snapped = snap_to_step(raw);
            assert!(STOPS.contains(&snapped));
        }
    }

    #[test]
    fn snap_zero_returns_zero() {
        assert_eq!(snap_to_step(0), 0);
    }

    #[test]
    fn snap_hundred_returns_hundred() {
        assert_eq!(snap_to_step(100), 100);
    }

    #[test]
    fn snap_clamps_over_hundred() {
        assert_eq!(snap_to_step(255), 100);
    }

    #[test]
    fn snap_picks_nearest() {
        assert_eq!(snap_to_step(50), 57);
        assert_eq!(snap_to_step(20), 14);
        assert_eq!(snap_to_step(30), 28);
    }

    #[test]
    fn step_index_zero_for_zero_percent() {
        assert_eq!(step_index(0), 0);
    }

    #[test]
    fn step_index_seven_for_full() {
        assert_eq!(step_index(100), 7);
    }

    #[test]
    fn step_index_in_range_for_all_inputs() {
        for raw in 0..=100u8 {
            assert!(step_index(raw) < STOPS.len());
        }
    }

    #[test]
    fn parse_pactl_volume_averages_channels() {
        let sample =
            "Volume: front-left: 65536 / 65% / -10.51 dB, front-right: 65536 / 65% / -10.51 dB";
        assert_eq!(parse_pactl_volume(sample), Some(65));
    }

    #[test]
    fn parse_pactl_volume_handles_single_channel() {
        let sample = "Volume: mono: 65536 / 80% / -5.00 dB";
        assert_eq!(parse_pactl_volume(sample), Some(80));
    }

    #[test]
    fn parse_pactl_volume_returns_none_on_empty() {
        assert_eq!(parse_pactl_volume("Sink #0\n\tName: foo\n"), None);
    }

    #[test]
    fn parse_pactl_mute_handles_yes_no() {
        assert_eq!(parse_pactl_mute("Mute: yes"), Some(true));
        assert_eq!(parse_pactl_mute("Mute: no"), Some(false));
    }

    #[test]
    fn parse_pactl_mute_is_case_insensitive() {
        assert_eq!(parse_pactl_mute("MUTE: YES"), Some(true));
        assert_eq!(parse_pactl_mute("Mute: NO"), Some(false));
    }

    #[test]
    fn parse_pactl_mute_returns_none_for_garbage() {
        assert_eq!(parse_pactl_mute("not a mute response"), None);
    }

    #[test]
    fn set_brightness_or_set_volume_fail_gracefully_when_binary_absent() {
        // Don't bother asserting Ok/Err; just verify the function
        // doesn't panic on a system without brightnessctl/pactl.
        let _ = set_brightness_percent(50);
        let _ = set_volume_percent(50);
        let _ = toggle_mute();
    }
}
