//! NetworkManager status chip — top-bar-right applet.
//!
//! Phase E1.2.3: reads the connectivity column from `nmcli
//! -t -f STATE,CONNECTIVITY g` (general status) and renders
//! a one-line chip: `<glyph> <active-connection-name>` or
//! "Disconnected" when nothing is active.

#![forbid(unsafe_code)]

use mde_applet_api::{AppletId, AppletSlot, HostMessage};

#[must_use]
pub fn manifest() -> mde_applet_api::AppletManifest {
    mde_applet_api::AppletManifest {
        id: AppletId::from_static("network"),
        binary: "mde-applet-network".into(),
        slot: AppletSlot::TopBarRight,
        summary: "NetworkManager active-connection chip".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

/// One active connection row from `nmcli -t -f
/// NAME,TYPE,DEVICE,STATE connection show --active`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveConnection {
    pub name: String,
    pub kind: String,
}

/// Parse the first active wifi/ethernet connection out of
/// nmcli's colon-separated active-connection list.
/// Returns `None` when no connection is active.
///
/// v4.0.1 BUG-9: `nmcli connection show --active` reports the
/// connection.type column using the IEEE/NM technical names —
/// `802-11-wireless` for Wi-Fi and `802-3-ethernet` for wired —
/// NOT the short names like `wifi` or `ethernet`. The original
/// whitelist (`wifi`/`802-3-ethernet`/`ethernet`) silently dropped
/// every active Wi-Fi connection so the chip showed "Disconnected"
/// even when wlp2s0 was associated. The aliases stay in the
/// whitelist so older NM versions / `nmcli device` paths that emit
/// the short names continue to match.
#[must_use]
pub fn parse_active(raw: &str) -> Option<ActiveConnection> {
    for line in raw.lines() {
        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() < 4 {
            continue;
        }
        if parts[3] != "activated" {
            continue;
        }
        let kind = parts[1];
        if !is_real_iface_kind(kind) {
            continue;
        }
        return Some(ActiveConnection {
            name: parts[0].to_string(),
            kind: kind.to_string(),
        });
    }
    None
}

/// Connection-type allow-list for what counts as "the network the
/// user sees in the chip". Excludes loopback, vpn, bridge, etc. —
/// those don't represent the upstream-internet path.
#[must_use]
pub const fn is_real_iface_kind(kind: &str) -> bool {
    matches!(
        kind.as_bytes(),
        b"wifi"
            | b"802-11-wireless"
            | b"802-3-ethernet"
            | b"ethernet"
    )
}

/// Glyph for a connection type. The host paints the actual
/// icon; the text is for fallback + accessibility.
#[must_use]
pub const fn type_glyph(kind: &str) -> &'static str {
    match kind.as_bytes() {
        // v4.0.1 BUG-9: include `802-11-wireless` alongside `wifi`.
        b"wifi" | b"802-11-wireless" => "\u{25EF}", // large circle = wifi-ish glyph
        b"802-3-ethernet" | b"ethernet" => "\u{2261}", // ≡ = ethernet
        _ => "?",
    }
}

/// Render the chip's display string. Disconnected →
/// "Disconnected".
#[must_use]
pub fn format_chip(conn: Option<&ActiveConnection>) -> String {
    match conn {
        Some(c) => format!("{} {}", type_glyph(&c.kind), c.name),
        None => "Disconnected".to_string(),
    }
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
        assert_eq!(m.id.as_str(), "network");
        assert_eq!(m.slot, AppletSlot::TopBarRight);
    }

    #[test]
    fn parse_active_extracts_first_wifi_row() {
        let raw = "home-wifi:wifi:wlan0:activated\nwired:ethernet:eno1:activated\n";
        let c = parse_active(raw).unwrap();
        assert_eq!(c.name, "home-wifi");
        assert_eq!(c.kind, "wifi");
    }

    #[test]
    fn parse_active_extracts_ethernet_when_no_wifi() {
        let raw = "wired:802-3-ethernet:eno1:activated\n";
        let c = parse_active(raw).unwrap();
        assert_eq!(c.kind, "802-3-ethernet");
    }

    #[test]
    fn parse_active_skips_inactive_rows() {
        let raw = "home-wifi:wifi:wlan0:deactivated\nwork-vpn:vpn:tun0:activated\n";
        // VPN doesn't count for the chip — both rows are
        // skipped.
        assert!(parse_active(raw).is_none());
    }

    #[test]
    fn parse_active_returns_none_on_empty() {
        assert!(parse_active("").is_none());
    }

    #[test]
    fn type_glyph_maps_wifi_and_ethernet() {
        assert_eq!(type_glyph("wifi"), "\u{25EF}");
        // v4.0.1 BUG-9: nmcli's IEEE name for Wi-Fi.
        assert_eq!(type_glyph("802-11-wireless"), "\u{25EF}");
        assert_eq!(type_glyph("ethernet"), "\u{2261}");
        assert_eq!(type_glyph("802-3-ethernet"), "\u{2261}");
        assert_eq!(type_glyph("vpn"), "?");
    }

    #[test]
    fn parse_active_extracts_802_11_wireless() {
        // v4.0.1 BUG-9 — operator's `nmcli connection show --active`
        // emits "802-11-wireless" for the Wi-Fi connection-type
        // column; the original whitelist dropped this and the chip
        // showed "Disconnected" even when wlp2s0 was associated.
        let raw = "FRANKS-REDHOTS:802-11-wireless:wlp2s0:activated\nlo:loopback:lo:activated\n";
        let c = parse_active(raw).unwrap();
        assert_eq!(c.name, "FRANKS-REDHOTS");
        assert_eq!(c.kind, "802-11-wireless");
    }

    #[test]
    fn format_chip_disconnected_when_none() {
        assert_eq!(format_chip(None), "Disconnected");
    }

    #[test]
    fn format_chip_combines_glyph_and_name() {
        let c = ActiveConnection {
            name: "home-wifi".into(),
            kind: "wifi".into(),
        };
        let chip = format_chip(Some(&c));
        assert!(chip.contains("home-wifi"));
        assert!(chip.contains("\u{25EF}"));
    }

    #[test]
    fn handle_host_short_circuits_shutdown() {
        assert!(!handle_host(&HostMessage::Shutdown));
    }
}
