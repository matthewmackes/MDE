//! Status-cluster pill — top-bar-right applet showing
//! battery + power-profile state.
//!
//! Phase E1.2.10: reads
//! `/sys/class/power_supply/BAT*/{capacity,status}` for
//! battery percent + AC-state, and
//! `powerprofilesctl get` for the active profile.

#![forbid(unsafe_code)]

use std::path::Path;

use mde_applet_api::{AppletId, AppletSlot, HostMessage};

#[must_use]
pub fn manifest() -> mde_applet_api::AppletManifest {
    mde_applet_api::AppletManifest {
        id: AppletId::from_static("status-cluster"),
        binary: "mde-applet-status-cluster".into(),
        slot: AppletSlot::TopBarRight,
        summary: "Battery + power-profile status pill".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

/// One battery's parsed state. The cluster picks the first
/// BAT* it finds — most laptops only have one anyway.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BatteryState {
    /// 0-100 % charge.
    pub capacity: u8,
    /// One of `Charging` / `Discharging` / `Full` /
    /// `Not charging` per the sysfs convention. Empty when
    /// the file is missing.
    pub status: String,
}

/// Walk `/sys/class/power_supply/` for a BAT* subdir; return
/// the first one found. Returns `None` on desktops with no
/// battery.
#[must_use]
pub fn find_battery_dir(root: &Path) -> Option<std::path::PathBuf> {
    let rd = std::fs::read_dir(root).ok()?;
    for entry in rd.flatten() {
        let name = entry.file_name();
        let name_str = name.to_str().unwrap_or("");
        if name_str.starts_with("BAT") {
            return Some(entry.path());
        }
    }
    None
}

/// Parse `capacity` + `status` files out of a battery dir.
/// Returns an empty state on missing files.
#[must_use]
pub fn parse_battery(dir: &Path) -> BatteryState {
    let capacity = std::fs::read_to_string(dir.join("capacity"))
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .unwrap_or(0);
    let status = std::fs::read_to_string(dir.join("status"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    BatteryState { capacity, status }
}

/// Glyph for a battery state. The host paints the actual
/// icon; the text is for fallback + accessibility.
#[must_use]
pub fn battery_glyph(state: &BatteryState) -> &'static str {
    if state.status == "Charging" || state.status == "Full" {
        "\u{26A1}" // lightning bolt
    } else if state.capacity == 0 {
        "?"
    } else if state.capacity < 20 {
        "\u{25AB}" // small empty square = low
    } else if state.capacity < 80 {
        "\u{25FB}" // medium square = mid
    } else {
        "\u{25A0}" // filled square = high
    }
}

/// Render the cluster's display string.
/// Format: `<battery-glyph> <capacity>% · <profile>`.
/// Empty profile section if not on a laptop.
#[must_use]
pub fn format_cluster(battery: Option<&BatteryState>, profile: &str) -> String {
    let mut s = String::new();
    if let Some(b) = battery {
        let glyph = battery_glyph(b);
        s.push_str(glyph);
        s.push(' ');
        s.push_str(&b.capacity.to_string());
        s.push('%');
    }
    let prof = profile.trim();
    if !prof.is_empty() {
        if !s.is_empty() {
            s.push_str(" · ");
        }
        s.push_str(prof);
    }
    if s.is_empty() {
        s.push_str("AC");
    }
    s
}

#[must_use]
pub fn handle_host(msg: &HostMessage) -> bool {
    !matches!(msg, HostMessage::Shutdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_lands_in_top_bar_right_slot() {
        let m = manifest();
        assert_eq!(m.id.as_str(), "status-cluster");
        assert_eq!(m.slot, AppletSlot::TopBarRight);
    }

    #[test]
    fn battery_glyph_lightning_when_charging() {
        let s = BatteryState {
            capacity: 50,
            status: "Charging".into(),
        };
        assert_eq!(battery_glyph(&s), "\u{26A1}");
    }

    #[test]
    fn battery_glyph_lightning_when_full() {
        let s = BatteryState {
            capacity: 100,
            status: "Full".into(),
        };
        assert_eq!(battery_glyph(&s), "\u{26A1}");
    }

    #[test]
    fn battery_glyph_low_under_20() {
        let s = BatteryState {
            capacity: 5,
            status: "Discharging".into(),
        };
        assert_eq!(battery_glyph(&s), "\u{25AB}");
    }

    #[test]
    fn battery_glyph_mid_under_80() {
        let s = BatteryState {
            capacity: 50,
            status: "Discharging".into(),
        };
        assert_eq!(battery_glyph(&s), "\u{25FB}");
    }

    #[test]
    fn battery_glyph_full_above_80() {
        let s = BatteryState {
            capacity: 90,
            status: "Discharging".into(),
        };
        assert_eq!(battery_glyph(&s), "\u{25A0}");
    }

    #[test]
    fn battery_glyph_question_when_zero_and_unknown() {
        let s = BatteryState {
            capacity: 0,
            status: "".into(),
        };
        assert_eq!(battery_glyph(&s), "?");
    }

    #[test]
    fn format_cluster_combines_battery_and_profile() {
        let b = BatteryState {
            capacity: 67,
            status: "Discharging".into(),
        };
        let s = format_cluster(Some(&b), "balanced");
        assert!(s.contains("67%"));
        assert!(s.contains("balanced"));
        assert!(s.contains(" · "));
    }

    #[test]
    fn format_cluster_ac_when_no_battery_or_profile() {
        assert_eq!(format_cluster(None, ""), "AC");
    }

    #[test]
    fn format_cluster_profile_only_on_desktop() {
        let s = format_cluster(None, "performance");
        assert_eq!(s, "performance");
    }

    #[test]
    fn handle_host_short_circuits_shutdown() {
        assert!(!handle_host(&HostMessage::Shutdown));
    }
}
