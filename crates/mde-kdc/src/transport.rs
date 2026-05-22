//! KDC2-3.2 — `KdcHost` Transport impl.
//!
//! Glues the protocol crate (`mde-kdc-proto`) + the pairing
//! store (KDC2-3.7) into the `mackes_transport::Transport`
//! trait. The mesh router (KDC2-1.8 worker) dispatches through
//! this impl exactly the same way it dispatches through
//! DirectUdp / DerpRelay / Https443.
//!
//! ## Stub network layer (current)
//!
//! `probe / open / health` derive their answers from the pairing
//! store: a peer present in `devices.toml` is treated as
//! reachable; absent → `Unreachable`. The actual TLS-wrapped TCP
//! socket lands in KDC2-3.2.a, gated on KDC2-2.8 (rustls helper
//! in the protocol crate). The stub is sufficient for the router
//! integration tests + the D-Bus host scaffold (KDC2-3.3) — both
//! exercise the Transport trait without driving real packets.
//!
//! ## What ships here
//!
//! - `KdcHost::new(pairing)` — construct from a shared
//!   `PairingStore`.
//! - `impl mackes_transport::Transport for KdcHost` —
//!   `kind() == TransportKind::KdcTls`, capabilities from
//!   `TransportCapabilities::kdc_tls_default()`, probe/open
//!   based on `PairingStore::get(peer_id).is_some()`.
//! - `StubConnection { id }` — placeholder `Connection` impl
//!   the open path returns. Real `TlsConnection` replaces it
//!   when KDC2-2.8 lands.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use mackes_transport::{
    transport_capabilities::TransportCapabilities, Capabilities, Connection, HealthState,
    MessageClassSet, Transport, TransportError, TransportKind,
};

use crate::pairing::PairingStore;

/// Concrete `Transport` impl for the KDE Connect wire.
#[derive(Debug)]
pub struct KdcHost {
    pairing: Arc<PairingStore>,
}

impl KdcHost {
    /// Construct from a shared pairing store. The `Arc` lets the
    /// daemon + the D-Bus host hold the store independently.
    #[must_use]
    pub fn new(pairing: Arc<PairingStore>) -> Self {
        Self { pairing }
    }

    /// True when `peer_id` is currently in the pairing store.
    /// Pure synchronous check used by every async method below.
    fn is_paired(&self, peer_id: &str) -> bool {
        self.pairing.get(peer_id).is_some()
    }
}

#[async_trait]
impl Transport for KdcHost {
    fn kind(&self) -> TransportKind {
        TransportKind::KdcTls
    }

    fn capabilities(&self) -> Capabilities {
        // Bridge the kdc_tls_default factory (KDC2-1.5) into
        // the existing `Capabilities` shape the router consumes.
        // `kdc_tls_default` reports payload-shape capabilities
        // (bulk / streaming / mtu / encryption); `Capabilities`
        // reports routing+health capabilities (carries-class
        // set / health window / label). Both coexist per the
        // KDC2-1.5 lock.
        let _payload = TransportCapabilities::kdc_tls_default();
        Capabilities {
            // KDC carries every message class; the protocol's 9
            // plugins cover Control / Clipboard / FileBulk /
            // Notification.
            carries: MessageClassSet::all(),
            // 60 KiB matches mde-kdc-proto's FrameDecoder
            // MAX_FRAME_BYTES sane bound (the per-frame cap is
            // 1 MiB but typical KDC frames are well under 64K).
            max_frame_bytes: Some(64 * 1024),
            // Re-probe cadence: 5 s. KDC's TLS handshake is
            // expensive, but a 5 s window matches the rest of
            // the router's cadence so a peer-side flap gets
            // noticed within one tick of the mesh-router.
            health_window: Duration::from_secs(5),
            // Operator-visible label used in audit + UI rendering.
            label: "kdc-tls".to_string(),
        }
    }

    async fn probe(&self, peer_id: &str) -> HealthState {
        // Stub: paired = Healthy, unpaired = Down. Real probe
        // (TLS-handshake-with-budget) lands with KDC2-2.8.
        if self.is_paired(peer_id) {
            HealthState::Healthy
        } else {
            HealthState::Down
        }
    }

    async fn open(&self, peer_id: &str) -> Result<Box<dyn Connection>, TransportError> {
        if self.is_paired(peer_id) {
            Ok(Box::new(StubConnection {
                id: format!("kdc-stub:{peer_id}"),
            }))
        } else {
            Err(TransportError::Unreachable { code: "not_paired" })
        }
    }

    async fn health(&self, peer_id: &str) -> HealthState {
        // Stub: same shape as probe. Once the TLS path lands
        // (KDC2-2.8), health() will read live observation
        // history rather than the static pairing-state lookup.
        self.probe(peer_id).await
    }
}

/// Placeholder Connection returned by `KdcHost::open` until
/// KDC2-2.8 ships the real TLS-wrapped socket. The router holds
/// it across sends + drops it when the peer session ends.
#[derive(Debug)]
struct StubConnection {
    id: String,
}

impl Connection for StubConnection {
    fn id(&self) -> &str {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pairing::PairedDevice;
    use std::sync::Arc;
    use tempfile::tempdir;

    fn make_host_with_peer(peer_id: &str) -> KdcHost {
        let tmp = tempdir().unwrap();
        let store = PairingStore::open_or_init(tmp.path()).unwrap();
        store
            .upsert(PairedDevice {
                id: peer_id.into(),
                name: peer_id.into(),
                kind: "phone".into(),
                fingerprint: "AB:CD".into(),
                public_key_b64: "AA==".into(),
                capabilities: vec!["kdeconnect.clipboard".into()],
                paired_at: 1_700_000_000,
                last_seen_at: 1_700_000_500,
            })
            .unwrap();
        // Leak the tempdir guard so the store survives — the
        // tests don't write more files after the host is
        // constructed, so this is fine.
        std::mem::forget(tmp);
        KdcHost::new(Arc::new(store))
    }

    fn make_empty_host() -> KdcHost {
        let tmp = tempdir().unwrap();
        let store = PairingStore::open_or_init(tmp.path()).unwrap();
        std::mem::forget(tmp);
        KdcHost::new(Arc::new(store))
    }

    fn block_on<F: std::future::Future>(fut: F) -> F::Output {
        // Tiny spin executor — KdcHost futures don't yield
        // (all logic is synchronous behind async-trait), so
        // a one-shot polling loop terminates immediately.
        use std::pin::Pin;
        use std::sync::Arc;
        use std::task::{Context, Poll, Wake, Waker};
        struct NoopWaker;
        impl Wake for NoopWaker {
            fn wake(self: Arc<Self>) {}
        }
        let mut fut: Pin<Box<F>> = Box::pin(fut);
        let waker: Waker = Arc::new(NoopWaker).into();
        let mut cx = Context::from_waker(&waker);
        loop {
            if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }

    #[test]
    fn kind_is_kdc_tls() {
        let h = make_empty_host();
        assert_eq!(h.kind(), TransportKind::KdcTls);
    }

    #[test]
    fn capabilities_carry_every_message_class() {
        let h = make_empty_host();
        let caps = h.capabilities();
        assert!(caps.carries.control);
        assert!(caps.carries.clipboard);
        assert!(caps.carries.file_bulk);
        assert!(caps.carries.notification);
        assert_eq!(caps.label, "kdc-tls");
    }

    #[test]
    fn probe_paired_peer_is_healthy() {
        let h = make_host_with_peer("alice");
        let state = block_on(h.probe("alice"));
        assert_eq!(state, HealthState::Healthy);
    }

    #[test]
    fn probe_unpaired_peer_is_down() {
        let h = make_empty_host();
        let state = block_on(h.probe("nobody"));
        assert_eq!(state, HealthState::Down);
    }

    #[test]
    fn open_paired_peer_returns_connection_with_id() {
        let h = make_host_with_peer("alice");
        let conn = block_on(h.open("alice")).expect("paired peer opens");
        assert_eq!(conn.id(), "kdc-stub:alice");
    }

    #[test]
    fn open_unpaired_peer_returns_unreachable() {
        let h = make_empty_host();
        let err = block_on(h.open("nobody")).expect_err("unpaired must fail");
        match err {
            TransportError::Unreachable { code } => {
                assert_eq!(code, "not_paired");
            }
            other => panic!("expected Unreachable, got {other:?}"),
        }
    }

    #[test]
    fn health_matches_probe() {
        let h = make_host_with_peer("alice");
        assert_eq!(block_on(h.health("alice")), HealthState::Healthy);
        assert_eq!(block_on(h.health("nobody")), HealthState::Down);
    }

    #[test]
    fn is_object_safe_via_transport_trait() {
        // The mesh-router (KDC2-1.8) holds `Vec<Arc<dyn
        // Transport>>` — KdcHost must coerce into the trait
        // object cleanly.
        let h = make_empty_host();
        let _trait_obj: Arc<dyn Transport> = Arc::new(h);
    }
}
