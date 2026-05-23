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
        payload_size: None,
        payload_transfer_info: None,
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
            payload_size: None,
            payload_transfer_info: None,
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

    // ─────────────────────────────────────────────────────────
    // KDC2-2.14 — ClipboardPlugin (Plugin trait impl)
    // ─────────────────────────────────────────────────────────

    use crate::plugins::{Plugin, PluginContext, PluginKind};

    #[test]
    fn clipboard_plugin_kind_and_handles_match_token() {
        let p = ClipboardPlugin::new();
        assert_eq!(p.kind(), PluginKind::Clipboard);
        assert_eq!(p.handles(), &["kdeconnect.clipboard"]);
    }

    #[test]
    fn clipboard_plugin_queues_inbound_content() {
        let mut plugin = ClipboardPlugin::new();
        let ctx = PluginContext::new("alice", true);
        plugin.process(&clipboard_packet(1, "hello".into()), &ctx);
        let drained = plugin.take_received();
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].content, "hello");
    }

    #[test]
    fn clipboard_plugin_drops_malformed_without_panic() {
        let mut plugin = ClipboardPlugin::new();
        let ctx = PluginContext::new("alice", true);
        let bad = Packet {
            id: 1,
            kind: "kdeconnect.clipboard".to_string(),
            body: serde_json::json!({"not_content": 42}),
            mde_caps: None,
            payload_size: None,
            payload_transfer_info: None,
        };
        plugin.process(&bad, &ctx);
        assert_eq!(plugin.pending_count(), 0);
    }

    // ─────────────────────────────────────────────────────────
    // UC-1 — on_received callback hook
    // ─────────────────────────────────────────────────────────

    #[test]
    fn clipboard_plugin_callback_fires_on_decoded_body() {
        use std::sync::{Arc, Mutex};
        let observed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let observed_cb = Arc::clone(&observed);
        let mut plugin = ClipboardPlugin::with_callback(Box::new(move |body| {
            observed_cb.lock().unwrap().push(body.content.clone());
        }));
        let ctx = PluginContext::new("alice", true);
        plugin.process(&clipboard_packet(1, "hello from phone".into()), &ctx);
        plugin.process(&clipboard_packet(2, "and a second one".into()), &ctx);
        let seen = observed.lock().unwrap();
        assert_eq!(seen.as_slice(),
            &["hello from phone".to_string(), "and a second one".to_string()]);
        // Drain mode still also captures (so host can dedup
        // against recent history without losing the audit trail).
        assert_eq!(plugin.pending_count(), 2);
    }

    #[test]
    fn clipboard_plugin_callback_skipped_for_malformed_body() {
        use std::sync::{Arc, Mutex};
        let observed: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
        let observed_cb = Arc::clone(&observed);
        let mut plugin = ClipboardPlugin::with_callback(Box::new(move |_body| {
            *observed_cb.lock().unwrap() += 1;
        }));
        let ctx = PluginContext::new("alice", true);
        let bad = Packet {
            id: 1,
            kind: "kdeconnect.clipboard".to_string(),
            body: serde_json::json!({"not_content": 42}),
            mde_caps: None,
            payload_size: None,
            payload_transfer_info: None,
        };
        plugin.process(&bad, &ctx);
        assert_eq!(*observed.lock().unwrap(), 0,
            "callback must not fire when body fails to decode");
    }

    #[test]
    fn clipboard_plugin_set_callback_replaces_after_construction() {
        use std::sync::{Arc, Mutex};
        let observed: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
        let observed_cb = Arc::clone(&observed);
        let mut plugin = ClipboardPlugin::new();
        plugin.set_callback(Box::new(move |_body| {
            *observed_cb.lock().unwrap() += 1;
        }));
        let ctx = PluginContext::new("alice", true);
        plugin.process(&clipboard_packet(1, "x".into()), &ctx);
        assert_eq!(*observed.lock().unwrap(), 1);
    }
}

// ────────────────────────────────────────────────────────────────
// KDC2-2.14 — ClipboardPlugin (Plugin trait impl, adapter pattern)
// ────────────────────────────────────────────────────────────────

/// Callback fired on every successfully-decoded inbound
/// clipboard body. The host (`mde-kdc` / `mackesd`) wires a
/// closure that bridges into the QNM mesh-clipboard bucket;
/// the protocol crate stays runtime-agnostic — no tokio, no
/// channel types in the trait signature.
///
/// `Send + Sync` so the plugin remains object-safe when held
/// behind `Box<dyn Plugin>` in the host's `Registry`.
pub type OnReceived = Box<dyn Fn(&ClipboardBody) + Send + Sync>;

/// `Plugin` impl that mirrors inbound clipboard content.
///
/// Adapter pattern (same as `NotificationPlugin`): the protocol
/// crate stays pure. Two consumption modes coexist:
///
/// * **Drain mode** (default, used by tests + the legacy in-
///   memory path): `process()` pushes onto an internal `Vec`;
///   callers drain via `take_received()`.
/// * **Callback mode** (production, wired by UC-7): construct
///   via `with_callback()` — `process()` invokes the closure
///   inline AND still pushes onto the internal Vec, so a host
///   that wires a callback can also still observe the drain
///   (useful for audit + the bridge's deduplication window).
#[derive(Default)]
pub struct ClipboardPlugin {
    received: Vec<ClipboardBody>,
    handles: [&'static str; 1],
    on_received: Option<OnReceived>,
}

impl std::fmt::Debug for ClipboardPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClipboardPlugin")
            .field("received_len", &self.received.len())
            .field("handles", &self.handles)
            .field("on_received", &self.on_received.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

impl ClipboardPlugin {
    /// New empty plugin in drain mode (no callback).
    #[must_use]
    pub fn new() -> Self {
        Self {
            received: Vec::new(),
            handles: ["kdeconnect.clipboard"],
            on_received: None,
        }
    }

    /// New plugin with an inbound-mirror callback. The callback
    /// fires before the body is pushed onto the internal drain
    /// queue, so the host can react synchronously without
    /// polling `take_received()`.
    #[must_use]
    pub fn with_callback(on_received: OnReceived) -> Self {
        Self {
            received: Vec::new(),
            handles: ["kdeconnect.clipboard"],
            on_received: Some(on_received),
        }
    }

    /// Replace the existing callback (or set one on a previously
    /// callback-less plugin). Used when the host wires the
    /// bridge channel after construction.
    pub fn set_callback(&mut self, on_received: OnReceived) {
        self.on_received = Some(on_received);
    }

    /// Drain every received clipboard body.
    #[must_use]
    pub fn take_received(&mut self) -> Vec<ClipboardBody> {
        std::mem::take(&mut self.received)
    }

    /// Items currently queued.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.received.len()
    }
}

impl crate::plugins::Plugin for ClipboardPlugin {
    fn kind(&self) -> crate::plugins::PluginKind {
        crate::plugins::PluginKind::Clipboard
    }

    fn handles(&self) -> &[&'static str] {
        &self.handles
    }

    fn process(
        &mut self,
        packet: &crate::wire::Packet,
        _ctx: &crate::plugins::PluginContext,
    ) -> Vec<crate::wire::Packet> {
        if let Ok(body) = from_packet_body::<ClipboardBody>(packet) {
            if let Some(cb) = &self.on_received {
                cb(&body);
            }
            self.received.push(body);
        }
        Vec::new()
    }
}
