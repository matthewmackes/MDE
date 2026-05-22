//! KDC2-2 plugins — per-feature plugin trait + the canonical
//! plugin registry (ping, clipboard, share, notification,
//! findmyphone, battery, mpris, sms, telephony).
//!
//! KDE Connect's wire format multiplexes nine plugins through a
//! single TLS session, distinguished by the `Packet.kind` string
//! (`kdeconnect.<plugin>`). KDC2 keeps the same nine plugins for
//! wire compatibility with stock clients. Extending with MDE-only
//! plugins is a v2.2+ deferred feature — the trait + registry
//! below are the seam.
//!
//! Per-plugin body types live in submodules. KDC2-2.5 lands
//! `clipboard` first (smallest body shape); KDC2-2.6..2.9 land
//! the remaining seven.

pub mod battery;
pub mod clipboard;
pub mod notification;
pub mod ping;
pub mod share;
pub mod sms;

pub use battery::{battery_packet, BatteryBody};
pub use clipboard::{clipboard_packet, from_packet_body, ClipboardBody};
pub use notification::{notification_packet, NotificationBody};
pub use ping::{ping_packet, PingBody};
pub use share::{file_share_packet, url_share_packet, ShareBody, ShareKind};
pub use sms::{sms_messages_packet, SmsMessage, SmsMessagesBody};

use std::fmt;

/// The canonical set of KDE Connect plugin types v2.1 KDC2 ships
/// at wire-compat parity with upstream.
///
/// The serde token (snake_case via Display) matches the `Packet
/// .kind` suffix (`kdeconnect.<token>`). Adding a new plugin
/// means a new variant here + a `PluginRegistry::default()`
/// update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginKind {
    /// Connection liveness check.
    Ping,
    /// Clipboard sync.
    Clipboard,
    /// File transfer.
    Share,
    /// Mirror notifications from one peer to another.
    Notification,
    /// Trigger phone ring / find-my-device.
    FindMyPhone,
    /// Mirror phone/laptop battery state.
    Battery,
    /// MPRIS media-player control.
    Mpris,
    /// SMS read/send (Android only).
    Sms,
    /// Phone-call state mirror.
    Telephony,
}

impl PluginKind {
    /// Every plugin KDC2 ships at v2.1 parity with upstream.
    /// Iteration order matters: it's the **default registration
    /// order** the host integration walks at startup, so handshake
    /// `incomingCapabilities` / `outgoingCapabilities` lists land
    /// in a deterministic shape (some KDC clients are sensitive to
    /// list order during pairing).
    #[must_use]
    pub const fn all() -> [PluginKind; 9] {
        [
            PluginKind::Ping,
            PluginKind::Clipboard,
            PluginKind::Share,
            PluginKind::Notification,
            PluginKind::FindMyPhone,
            PluginKind::Battery,
            PluginKind::Mpris,
            PluginKind::Sms,
            PluginKind::Telephony,
        ]
    }

    /// Wire token sans the `kdeconnect.` prefix.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            PluginKind::Ping => "ping",
            PluginKind::Clipboard => "clipboard",
            PluginKind::Share => "share.request",
            PluginKind::Notification => "notification",
            PluginKind::FindMyPhone => "findmyphone.request",
            PluginKind::Battery => "battery",
            PluginKind::Mpris => "mpris",
            PluginKind::Sms => "sms.messages",
            PluginKind::Telephony => "telephony",
        }
    }

    /// Full `Packet.kind` string for this plugin (`kdeconnect.<token>`).
    /// Used by the wire decoder to dispatch incoming packets to the
    /// right `Plugin::on_packet` handler.
    #[must_use]
    pub fn packet_kind(self) -> String {
        format!("kdeconnect.{}", self.token())
    }
}

impl fmt::Display for PluginKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// Object-safe trait every plugin implements. Lives in this crate
/// so plugin registry walks can dispatch through a `Box<dyn
/// Plugin>` regardless of which crate owns the actual impl.
///
/// KDC2-2.1 ships only the trait shape. Plugin implementations
/// land per-plugin starting at KDC2-2.5 (clipboard first, since
/// it's the smallest body shape).
pub trait Plugin: Send + Sync + std::fmt::Debug {
    /// Which plugin variant is this implementation for. The
    /// registry uses this to route incoming packets without
    /// downcasting.
    fn kind(&self) -> PluginKind;

    /// `kdeconnect.identity.incomingCapabilities` value this
    /// plugin contributes (typically `[self.kind().packet_kind()]`).
    /// Some plugins (like `share.request`) carry multiple kinds —
    /// the trait returns a slice so they can list every one.
    fn incoming_kinds(&self) -> &[&'static str];

    /// `kdeconnect.identity.outgoingCapabilities` value this
    /// plugin contributes.
    fn outgoing_kinds(&self) -> &[&'static str];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_kind_count_matches_upstream_kdc() {
        // v2.1 KDC2 lock: parity with upstream KDE Connect's 9
        // canonical plugins. Adding a 10th means a new variant +
        // a survey lock + a memory note.
        assert_eq!(PluginKind::all().len(), 9);
    }

    #[test]
    fn plugin_kind_packet_kind_includes_kdeconnect_prefix() {
        for k in PluginKind::all() {
            let s = k.packet_kind();
            assert!(
                s.starts_with("kdeconnect."),
                "plugin {k:?} packet kind {s:?} missing kdeconnect. prefix",
            );
        }
    }

    #[test]
    fn share_plugin_uses_request_suffix() {
        // Upstream's share plugin's kind is `kdeconnect.share.request`,
        // NOT `kdeconnect.share`. A drop of `.request` would silently
        // break file transfer with stock clients.
        assert_eq!(PluginKind::Share.packet_kind(), "kdeconnect.share.request");
    }

    #[test]
    fn findmyphone_plugin_uses_request_suffix() {
        // Same upstream quirk as Share — the trigger packet is
        // `kdeconnect.findmyphone.request`.
        assert_eq!(
            PluginKind::FindMyPhone.packet_kind(),
            "kdeconnect.findmyphone.request",
        );
    }

    #[test]
    fn sms_plugin_uses_messages_suffix() {
        // The Android KDE Connect SMS plugin emits
        // `kdeconnect.sms.messages` (plural).
        assert_eq!(PluginKind::Sms.packet_kind(), "kdeconnect.sms.messages");
    }

    #[test]
    fn plugin_kind_tokens_are_unique() {
        // Two plugins sharing the same token would silently merge
        // in the registry. Hard-lock uniqueness.
        let mut tokens: Vec<&'static str> = PluginKind::all().iter().map(|k| k.token()).collect();
        tokens.sort_unstable();
        tokens.dedup();
        assert_eq!(tokens.len(), 9);
    }
}
