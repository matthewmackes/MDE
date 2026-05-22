//! KDC2-2.5 clipboard plugin — `kdeconnect.clipboard` packet body.
//!
//! Stock KDE Connect's clipboard plugin sends a packet of kind
//! `kdeconnect.clipboard` with a single body field `content`
//! (UTF-8 string). KDC2 ships the matching body type plus the
//! generic [`from_packet_body`] downcast helper that other plugins
//! reuse.
//!
//! Wire compatibility note: upstream sometimes also emits
//! `kdeconnect.clipboard.connect` — the same body shape, but only
//! sent on connection-handshake to push the current clipboard
//! contents at the new peer. The body is identical so the same
//! [`ClipboardBody`] type covers both packet kinds.

use serde::{Deserialize, Serialize};

use crate::wire::Packet;

/// `kdeconnect.clipboard` (+ `.connect`) packet body. UTF-8 text
/// payload, no length cap on the wire — receivers enforce their
/// own size limit before applying.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardBody {
    /// The clipboard content. UTF-8 only; binary payloads use
    /// the `share.request` plugin (file transfer).
    pub content: String,
}

/// Generic downcast helper: extract a typed body `B` from a
/// [`Packet`]. Used by every plugin's `on_packet` implementation
/// to interpret the wire's `serde_json::Value` body without
/// reimplementing the same JSON re-serialize → deserialize dance
/// every time.
///
/// The function pattern (rather than a `Packet::body_as::<B>()`
/// method) keeps the wire module pluginsuncoupled — see the
/// crate-level doc on the `protocol → router → daemon → surface`
/// layering rule.
pub fn from_packet_body<B>(packet: &Packet) -> Result<B, serde_json::Error>
where
    B: for<'de> Deserialize<'de>,
{
    serde_json::from_value(packet.body.clone())
}

/// Build a `kdeconnect.clipboard` packet from clipboard text.
/// Used by the host integration (KDC2-3) when the user copies
/// text on a local MDE peer.
///
/// `id_ms` is the millisecond Unix timestamp the receiver uses
/// for deduplication; callers should pass `chrono::Utc::now()
/// .timestamp_millis()` (or equivalent) so paired devices can
/// dedup dual-sent copies via the mesh router.
#[must_use]
pub fn clipboard_packet(id_ms: i64, content: String) -> Packet {
    Packet {
        id: id_ms,
        kind: "kdeconnect.clipboard".to_string(),
        body: serde_json::json!({"content": content}),
        mde_caps: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_packet_serializes_with_upstream_field_names() {
        let p = clipboard_packet(123, "hello".to_string());
        let s = serde_json::to_string(&p).unwrap();
        // Wire compatibility: upstream Android client deserializes
        // `content` verbatim.
        assert!(s.contains(r#""content":"hello""#));
        assert!(s.contains(r#""type":"kdeconnect.clipboard""#));
        assert!(s.contains(r#""id":123"#));
    }

    #[test]
    fn from_packet_body_extracts_clipboard_payload() {
        let p = clipboard_packet(1, "extracted".to_string());
        let body: ClipboardBody = from_packet_body(&p).unwrap();
        assert_eq!(body.content, "extracted");
    }

    #[test]
    fn from_packet_body_round_trips_via_wire() {
        // Encode → decode through serde_json::to_string + from_str
        // (simulating a real send/recv hop) then downcast.
        let p = clipboard_packet(42, "round-trip".to_string());
        let wire = serde_json::to_string(&p).unwrap();
        let decoded: Packet = serde_json::from_str(&wire).unwrap();
        let body: ClipboardBody = from_packet_body(&decoded).unwrap();
        assert_eq!(body.content, "round-trip");
    }

    #[test]
    fn from_packet_body_rejects_mismatched_shape() {
        // Body that's the wrong shape (missing `content`) surfaces
        // a serde error, not a panic. Plugins use this to detect
        // a malformed peer + drop the packet.
        let p = Packet {
            id: 1,
            kind: "kdeconnect.clipboard".to_string(),
            body: serde_json::json!({"wrong_field": 42}),
            mde_caps: None,
        };
        let result: Result<ClipboardBody, _> = from_packet_body(&p);
        assert!(result.is_err());
    }

    #[test]
    fn clipboard_body_round_trips_through_json() {
        let b = ClipboardBody {
            content: "with newlines\n and tabs\t and unicode 🦀".to_string(),
        };
        let s = serde_json::to_string(&b).unwrap();
        let back: ClipboardBody = serde_json::from_str(&s).unwrap();
        assert_eq!(back, b);
    }

    #[test]
    fn clipboard_packet_id_lands_in_dedup_field() {
        // The `id` is the dedup key — two packets with the same
        // id from the same peer are the same logical clipboard
        // event (mesh-router dual-send relies on this).
        let p1 = clipboard_packet(7, "x".to_string());
        let p2 = clipboard_packet(7, "x".to_string());
        assert_eq!(p1.id, p2.id);
        assert_eq!(p1.body, p2.body);
    }
}
