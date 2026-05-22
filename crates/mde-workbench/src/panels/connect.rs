//! KDC2-5.4 / 5.5 / 5.6 / 5.7 — Workbench "Connect" peer card.
//!
//! Replaces the v13.0 GTK3 KDE Connect panel. Renders one card
//! per paired device (read from the `dev.mackes.MDE.Connect`
//! D-Bus interface) with four conditional sections:
//!
//!   * **Phone** (5.4) — battery glyph + Ring + Find + MPRIS
//!     transport controls. Shown when `peer.kind == "phone"`.
//!
//!   * **Messaging** (5.5) — SMS thread list + composer. Shown
//!     when the peer's `capabilities` advertises
//!     `kdeconnect.sms.messages` (iOS doesn't, Android does).
//!
//!   * **Share** (5.6) — drop-file target wired to
//!     `dev.mackes.MDE.Connect1.SendFile`. Shown when the peer
//!     advertises `kdeconnect.share.request`.
//!
//!   * **Common chrome** (5.7) — Clipboard / Notification mirror
//!     toggles + the Pair / Unpair button. Always visible.
//!
//! This module ships the pure-model layer + the section
//! visibility logic + text-rendering helpers. The Iced view
//! integration into the crate-level Message router lives in a
//! follow-up commit because routing a new panel through
//! `crate::Message` + `app.rs::update` touches several files;
//! the pure model + tests are the load-bearing piece.

#![allow(dead_code)] // The Iced view wiring lands in the boot-integration follow-up.

use serde::{Deserialize, Serialize};

/// One paired device — wire-equivalent to the
/// `dev.mackes.MDE.Connect1.DeviceInfo` struct in mde-kdc.
/// Reproduced here as a flat type so the panel doesn't take a
/// direct dep on mde-kdc (which would drag in tokio + zbus).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectPeer {
    /// Stable device id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// `phone` / `tablet` / `desktop` / `unknown`.
    pub kind: String,
    /// SHA-256 fingerprint, `AB:CD:EF:...` format.
    pub fingerprint: String,
    /// Plugin tokens advertised by the device. Drives the
    /// section-visibility predicates.
    pub capabilities: Vec<String>,
    /// Pair-time (unix epoch seconds).
    pub paired_at: i64,
    /// Most-recent reachability observation (0 = never).
    pub last_seen_at: i64,
    /// Battery percentage (0..=100); `None` when the device
    /// hasn't reported yet OR the battery plugin is disabled.
    pub battery_pct: Option<u8>,
    /// MPRIS now-playing title, if any.
    pub now_playing: Option<String>,
}

/// Identifies one section of the peer card so tests + the view
/// can reason about visibility uniformly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectSection {
    /// KDC2-5.4 — phone-specific controls.
    Phone,
    /// KDC2-5.5 — SMS thread list + composer.
    Messaging,
    /// KDC2-5.6 — file drop target.
    Share,
    /// KDC2-5.7 — common chrome (clipboard, notifications
    /// mirror, pair toggle). Always visible.
    CommonChrome,
}

/// Section-visibility predicate. The view renders only the
/// sections this returns `true` for.
#[must_use]
pub fn section_visible_for(section: ConnectSection, peer: &ConnectPeer) -> bool {
    match section {
        ConnectSection::CommonChrome => true,
        ConnectSection::Phone => peer.kind == "phone",
        ConnectSection::Messaging => peer
            .capabilities
            .iter()
            .any(|c| c == "kdeconnect.sms.messages"),
        ConnectSection::Share => peer
            .capabilities
            .iter()
            .any(|c| c == "kdeconnect.share.request"),
    }
}

/// KDC2-5.4 — phone section text fragment. Pure helper that
/// the Iced view feeds into a `text()` widget.
#[must_use]
pub fn render_phone_section(peer: &ConnectPeer) -> String {
    let battery = match peer.battery_pct {
        Some(pct) => format!("Battery: {pct}%"),
        None => "Battery: —".to_string(),
    };
    let now_playing = peer
        .now_playing
        .as_deref()
        .map(|t| format!("Now playing: {t}"))
        .unwrap_or_else(|| "Now playing: (nothing)".to_string());
    format!("{battery}\n{now_playing}\n[Ring] [Find]")
}

/// KDC2-5.5 — messaging section text fragment.
#[must_use]
pub fn render_messaging_section(peer: &ConnectPeer) -> String {
    if !section_visible_for(ConnectSection::Messaging, peer) {
        return String::new();
    }
    "Threads: (none yet — pulls from `kdeconnect.sms.messages`)\n[New message]".to_string()
}

/// KDC2-5.6 — share section text fragment.
#[must_use]
pub fn render_share_section(peer: &ConnectPeer) -> String {
    if !section_visible_for(ConnectSection::Share, peer) {
        return String::new();
    }
    "Drop a file here → SendFile(device_id, path)".to_string()
}

/// KDC2-5.7 — common chrome text fragment.
#[must_use]
pub fn render_common_chrome(peer: &ConnectPeer) -> String {
    let last = if peer.last_seen_at == 0 {
        "Never reached".to_string()
    } else {
        format!("Last seen: {}", peer.last_seen_at)
    };
    format!(
        "Fingerprint: {fp}\n{last}\n[Mirror clipboard] [Mirror notifications] [Unpair]",
        fp = peer.fingerprint,
    )
}

/// Top-level card renderer: returns the section list (in render
/// order) the view should display for this peer + their text
/// fragments. The Iced view turns these into widgets.
#[must_use]
pub fn render_card(peer: &ConnectPeer) -> Vec<(ConnectSection, String)> {
    let mut out = Vec::new();
    if section_visible_for(ConnectSection::Phone, peer) {
        out.push((ConnectSection::Phone, render_phone_section(peer)));
    }
    if section_visible_for(ConnectSection::Messaging, peer) {
        out.push((ConnectSection::Messaging, render_messaging_section(peer)));
    }
    if section_visible_for(ConnectSection::Share, peer) {
        out.push((ConnectSection::Share, render_share_section(peer)));
    }
    // Common chrome always visible, at the bottom.
    out.push((ConnectSection::CommonChrome, render_common_chrome(peer)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_peer(kind: &str, caps: &[&str]) -> ConnectPeer {
        ConnectPeer {
            id: "abc-123".into(),
            name: "Pixel 8".into(),
            kind: kind.into(),
            fingerprint: "AB:CD:EF".into(),
            capabilities: caps.iter().map(|s| s.to_string()).collect(),
            paired_at: 1_700_000_000,
            last_seen_at: 1_700_001_000,
            battery_pct: Some(72),
            now_playing: Some("track-name".into()),
        }
    }

    #[test]
    fn common_chrome_always_visible() {
        let phone = make_peer("phone", &[]);
        let desk = make_peer("desktop", &[]);
        assert!(section_visible_for(ConnectSection::CommonChrome, &phone));
        assert!(section_visible_for(ConnectSection::CommonChrome, &desk));
    }

    #[test]
    fn phone_section_only_visible_for_phones() {
        assert!(section_visible_for(
            ConnectSection::Phone,
            &make_peer("phone", &[]),
        ));
        assert!(!section_visible_for(
            ConnectSection::Phone,
            &make_peer("desktop", &[]),
        ));
        assert!(!section_visible_for(
            ConnectSection::Phone,
            &make_peer("tablet", &[]),
        ));
    }

    #[test]
    fn messaging_section_gated_on_sms_messages_capability() {
        let with = make_peer("phone", &["kdeconnect.sms.messages"]);
        let without = make_peer("phone", &["kdeconnect.clipboard"]);
        assert!(section_visible_for(ConnectSection::Messaging, &with));
        assert!(!section_visible_for(ConnectSection::Messaging, &without));
    }

    #[test]
    fn share_section_gated_on_share_request_capability() {
        let with = make_peer("phone", &["kdeconnect.share.request"]);
        let without = make_peer("phone", &[]);
        assert!(section_visible_for(ConnectSection::Share, &with));
        assert!(!section_visible_for(ConnectSection::Share, &without));
    }

    #[test]
    fn phone_section_includes_battery_when_known() {
        let peer = make_peer("phone", &[]);
        let txt = render_phone_section(&peer);
        assert!(txt.contains("Battery: 72%"));
        assert!(txt.contains("[Ring]"));
        assert!(txt.contains("[Find]"));
    }

    #[test]
    fn phone_section_shows_em_dash_when_battery_unknown() {
        let mut peer = make_peer("phone", &[]);
        peer.battery_pct = None;
        let txt = render_phone_section(&peer);
        assert!(txt.contains("Battery: —"));
    }

    #[test]
    fn phone_section_shows_nothing_when_no_now_playing() {
        let mut peer = make_peer("phone", &[]);
        peer.now_playing = None;
        let txt = render_phone_section(&peer);
        assert!(txt.contains("(nothing)"));
    }

    #[test]
    fn messaging_section_renders_only_when_visible() {
        let with = make_peer("phone", &["kdeconnect.sms.messages"]);
        let without = make_peer("phone", &[]);
        assert!(!render_messaging_section(&with).is_empty());
        assert!(render_messaging_section(&without).is_empty());
    }

    #[test]
    fn share_section_renders_only_when_visible() {
        let with = make_peer("phone", &["kdeconnect.share.request"]);
        let without = make_peer("phone", &[]);
        assert!(!render_share_section(&with).is_empty());
        assert!(render_share_section(&without).is_empty());
    }

    #[test]
    fn common_chrome_shows_never_reached_for_fresh_pair() {
        let mut peer = make_peer("phone", &[]);
        peer.last_seen_at = 0;
        let txt = render_common_chrome(&peer);
        assert!(txt.contains("Never reached"));
        assert!(txt.contains("AB:CD:EF"));
        assert!(txt.contains("[Unpair]"));
    }

    #[test]
    fn render_card_emits_sections_in_phone_messaging_share_chrome_order() {
        // A fully-featured phone returns Phone + Messaging +
        // Share + CommonChrome in that order.
        let peer = make_peer(
            "phone",
            &["kdeconnect.sms.messages", "kdeconnect.share.request"],
        );
        let sections: Vec<ConnectSection> =
            render_card(&peer).into_iter().map(|(s, _)| s).collect();
        assert_eq!(
            sections,
            vec![
                ConnectSection::Phone,
                ConnectSection::Messaging,
                ConnectSection::Share,
                ConnectSection::CommonChrome,
            ],
        );
    }

    #[test]
    fn render_card_for_desktop_omits_phone_messaging_share() {
        // A paired desktop peer has no phone/messaging/share
        // sections; only CommonChrome surfaces.
        let peer = make_peer("desktop", &["kdeconnect.clipboard"]);
        let sections: Vec<ConnectSection> =
            render_card(&peer).into_iter().map(|(s, _)| s).collect();
        assert_eq!(sections, vec![ConnectSection::CommonChrome]);
    }
}
