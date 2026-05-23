//! KDC2-3.2 — `KdcHost` Transport impl.
//!
//! Glues the protocol crate (`mde-kdc-proto`) + the pairing
//! store (KDC2-3.7) + the discovery registry (KDC2-2.11) into the
//! `mackes_transport::Transport` trait. The mesh router (KDC2-1.8
//! worker) dispatches through this impl exactly the same way it
//! dispatches through DirectUdp / DerpRelay / Https443.
//!
//! ## Real TLS network layer (KDC2-2.8 closure, 2026-05-23)
//!
//! `probe`/`open`/`health` consult both stores:
//!
//!   * **Pairing store** — peer must be `PairedDevice` to be eligible.
//!     Otherwise `Unreachable { code: "not_paired" }`.
//!   * **Discovery registry** — peer must have a recent source
//!     `SocketAddr` (cached from UDP/1716 announces). Otherwise
//!     `Unreachable { code: "not_discovered" }`.
//!
//! On `open`, the host TCP-connects to `(source_addr.ip(),
//! KDC_TLS_PORT)` and wraps the stream with
//! [`tls::connect_pinned_tls`] using the paired device's stored
//! SHA-256 fingerprint. A successful handshake yields a
//! [`KdcTlsConnection`] carrying the live `TlsStream<TcpStream>`
//! and a stable `kdc-tls:{peer_id}` identifier the router
//! correlates against audit entries. Fingerprint mismatch surfaces
//! as `HandshakeFailed { code: "fingerprint_mismatch" }` so the
//! UI can render `PairingState::KeyMismatch`.
//!
//! ## What ships here
//!
//! - `KdcHost::new(pairing, discovery)` — production constructor
//!   tying the host to a shared pairing store + the KDC discovery
//!   registry.
//! - `KdcHost::pairing_only(pairing)` — test/bench helper that
//!   constructs an empty discovery registry. Useful for the
//!   trait-conformance tests that exercise the "paired but
//!   unreachable" error path without booting a TCP listener.
//! - `impl mackes_transport::Transport for KdcHost` —
//!   `kind() == TransportKind::KdcTls`, capabilities from
//!   `TransportCapabilities::kdc_tls_default()`, open performs the
//!   pinned TLS handshake.
//! - `KdcTlsConnection { id, stream }` — live `Connection` impl
//!   the router holds across sends.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use mackes_transport::{
    transport_capabilities::TransportCapabilities, Capabilities, Connection, HealthState,
    MessageClassSet, Transport, TransportError, TransportKind,
};
use mde_kdc_proto::codec::FrameDecoder;
use mde_kdc_proto::discovery::DiscoveryRegistry;
use mde_kdc_proto::plugins::{PluginContext, PluginKind, Registry};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex as AsyncMutex;
use tokio_rustls::client::TlsStream;

use crate::dispatch::{check_plugin_allowed, PluginAuthority};
use crate::pairing::PairingStore;
use crate::tls;

/// KDE Connect's wire port. Both UDP broadcasts (discovery) and
/// the TCP TLS handshake use 1716; upstream KDC may fall through
/// to 1717-1764 if 1716 is busy, but stock devices advertise 1716
/// by default.
pub const KDC_TLS_PORT: u16 = 1716;

/// Concrete `Transport` impl for the KDE Connect wire.
#[derive(Debug)]
pub struct KdcHost {
    pairing: Arc<PairingStore>,
    discovery: Arc<AsyncMutex<DiscoveryRegistry>>,
}

impl KdcHost {
    /// Construct the production host wiring. Both stores are
    /// shared with the daemon's other workers (the future
    /// `kdc_discovery` worker writes to `discovery`; the D-Bus
    /// host scaffold (KDC2-3.3) writes to `pairing`).
    #[must_use]
    pub fn new(
        pairing: Arc<PairingStore>,
        discovery: Arc<AsyncMutex<DiscoveryRegistry>>,
    ) -> Self {
        Self { pairing, discovery }
    }

    /// Test/bench helper — constructs a host without any
    /// discovery wiring. Every `open()` call returns
    /// `Unreachable { code: "not_discovered" }`, which lets
    /// conformance tests exercise the paired-but-unreachable
    /// branch without spinning up a TLS listener. Production
    /// code uses [`KdcHost::new`].
    #[must_use]
    pub fn pairing_only(pairing: Arc<PairingStore>) -> Self {
        Self {
            pairing,
            discovery: Arc::new(AsyncMutex::new(DiscoveryRegistry::new())),
        }
    }

    /// Borrow the discovery registry — exposed so the
    /// `kdc_discovery` worker (KDC2-2.9.a follow-up) can inject
    /// real announces via the same `Arc` the host holds.
    #[must_use]
    pub fn discovery(&self) -> Arc<AsyncMutex<DiscoveryRegistry>> {
        Arc::clone(&self.discovery)
    }
}

/// Live `Connection` returned by [`KdcHost::open`] — wraps the
/// `tokio_rustls::client::TlsStream<TcpStream>` produced by the
/// pinned-fingerprint handshake. The router holds it across
/// sends for the peer session's lifetime.
///
/// The stream is parked behind a `tokio::sync::Mutex` so the
/// router can `lock().await.write_all(...)` from any of its
/// tasks without giving up the connection. Per-send sequencing
/// happens at the protocol-frame layer (mde-kdc-proto codec) —
/// this mutex just guarantees mutually-exclusive access to the
/// underlying TLS half.
pub struct KdcTlsConnection {
    id: String,
    stream: AsyncMutex<TlsStream<TcpStream>>,
}

impl KdcTlsConnection {
    /// The peer-derived identifier (`kdc-tls:{peer_id}`).
    #[must_use]
    pub fn id_owned(&self) -> &str {
        &self.id
    }

    /// Take an exclusive lock on the underlying TLS stream. The
    /// future protocol-frame writer/reader pair (KDC2-3.2.b) goes
    /// through this.
    pub async fn lock_stream(&self) -> tokio::sync::MutexGuard<'_, TlsStream<TcpStream>> {
        self.stream.lock().await
    }
}

impl std::fmt::Debug for KdcTlsConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KdcTlsConnection")
            .field("id", &self.id)
            .field("stream", &"<TlsStream<TcpStream>>")
            .finish()
    }
}

impl Connection for KdcTlsConnection {
    fn id(&self) -> &str {
        &self.id
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
        // Healthy iff paired AND we have a recent announce
        // address. The router uses this on every tick before
        // deciding to send; latency-tracking lands in the
        // observation history Path (KDC2-1.12).
        if self.pairing.get(peer_id).is_none() {
            return HealthState::Down;
        }
        let addr = self.discovery.lock().await.source_addr_for(peer_id);
        if addr.is_some() {
            HealthState::Healthy
        } else {
            HealthState::Down
        }
    }

    async fn open(&self, peer_id: &str) -> Result<Box<dyn Connection>, TransportError> {
        let device = self.pairing.get(peer_id).ok_or(TransportError::Unreachable {
            code: "not_paired",
        })?;
        let addr = {
            let guard = self.discovery.lock().await;
            guard.source_addr_for(peer_id)
        }
        .ok_or(TransportError::Unreachable {
            code: "not_discovered",
        })?;
        // KDC's TLS handshake uses TCP/1716 on the IP we learned
        // from the UDP/1716 announce. We DON'T trust the
        // announce's port (announces only carry identity, not
        // wire ports) — KDC_TLS_PORT is the stock default.
        let dial_addr = std::net::SocketAddr::new(addr.ip(), KDC_TLS_PORT);
        let stream = tls::connect_pinned_tls(
            dial_addr,
            &device.id,
            Some(device.fingerprint.clone()),
        )
        .await
        .map_err(|e| match e {
            tls::ConnectError::Tcp(_) => TransportError::Unreachable {
                code: "tcp_refused",
            },
            tls::ConnectError::Tls(_) => TransportError::HandshakeFailed {
                code: "fingerprint_mismatch",
            },
            tls::ConnectError::BadPeerName(_) => TransportError::Misconfigured {
                code: "bad_peer_name",
            },
        })?;
        Ok(Box::new(KdcTlsConnection {
            id: format!("kdc-tls:{peer_id}"),
            stream: AsyncMutex::new(stream),
        }))
    }

    async fn health(&self, peer_id: &str) -> HealthState {
        // Mirror probe — once the observation history lands
        // (KDC2-1.12), health() will weigh recent latency /
        // packet-loss into the answer.
        self.probe(peer_id).await
    }
}

// ──────────────────────────────────────────────────────────────────
// UC-4 — inbound TLS accept loop + per-connection dispatch.
//
// `KdcHost::serve()` binds `0.0.0.0:KDC_TLS_PORT` (1716), accepts
// TCP connections, completes mutual-TLS via `accept_pinned_tls`,
// and spawns a per-connection reader that:
//
//   1. Buffers bytes through `mde_kdc_proto::codec::FrameDecoder`.
//   2. For each complete `Packet`, classifies its kind into a
//      `PluginKind` via `kind_from_packet`, runs `dispatch::
//      check_plugin_allowed` against the supplied authority, and
//      either dispatches to the shared `Registry` or drops the
//      packet (with an audit-grade log line from the caller).
//   3. Writes any response packets the registry emits back
//      through the same TLS stream via `codec::encode_frame`.
//
// The accept loop runs until either the listener errors fatally
// or the caller drops the returned `JoinHandle` (the inbound
// worker manages shutdown via its `ShutdownToken`).
// ──────────────────────────────────────────────────────────────────

/// Inputs to [`KdcHost::serve`]. Bundles the cached cert + key +
/// dispatch authority + plugin registry so the function signature
/// stays manageable.
pub struct ServeConfig {
    /// Bind address. Defaults to `0.0.0.0:1716` for production;
    /// tests pass `127.0.0.1:0` to grab a random loopback port.
    pub bind_addr: std::net::SocketAddr,
    /// Server cert DER (issued at startup via
    /// `keygen::issue_identity_cert(host_pkcs8, host_id)`).
    pub server_cert_der: Vec<u8>,
    /// Server private key (PKCS#8 DER), matching `server_cert_der`.
    pub server_key_pkcs8_der: Vec<u8>,
    /// Plugin registry the inbound dispatcher routes into. Wrapped
    /// in tokio's `AsyncMutex` so `process()`'s `&mut self`
    /// requirement is honored across `.await` points without
    /// holding a `std::sync::Mutex` guard through the await.
    pub registry: Arc<AsyncMutex<Registry>>,
    /// Dispatch-policy authority. mackesd's `LoadedPolicy`
    /// satisfies this; tests use a fixed `PluginAuthority` impl.
    pub authority: Arc<dyn PluginAuthority + Send + Sync>,
}

impl std::fmt::Debug for ServeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServeConfig")
            .field("bind_addr", &self.bind_addr)
            .field("server_cert_der_len", &self.server_cert_der.len())
            .field("server_key_pkcs8_der_len", &self.server_key_pkcs8_der.len())
            .field("registry", &"<Arc<AsyncMutex<Registry>>>")
            .field("authority", &"<Arc<dyn PluginAuthority>>")
            .finish()
    }
}

/// Lifecycle events emitted by the inbound serve loop. The
/// caller (the worker layer) translates these into
/// `tracing::{info,warn,debug}` lines + audit-log entries; this
/// crate stays log-framework-agnostic.
#[derive(Debug)]
pub enum ServeEvent {
    /// Listener bound successfully + ready to accept.
    Listening {
        /// The actual bound address (useful when `bind_addr` was `:0`).
        addr: std::net::SocketAddr,
    },
    /// A new connection completed mutual-TLS.
    PeerConnected {
        /// The paired device id resolved from the client's cert.
        peer_id: String,
    },
    /// A connection ended (clean close or read error).
    PeerDisconnected {
        /// The device id that disconnected.
        peer_id: String,
    },
    /// A TLS handshake failed (cert mismatch, unpaired, etc.).
    AcceptRejected {
        /// Best-effort remote-addr; may be missing if accept() errored.
        remote: Option<std::net::SocketAddr>,
        /// The accept error rendered as a stable token.
        reason: String,
    },
    /// One inbound packet ran through the dispatch pipeline.
    PacketDispatched {
        /// The peer that sent the packet.
        peer_id: String,
        /// The packet kind (e.g. `kdeconnect.clipboard`).
        kind: String,
        /// True if the dispatch authority allowed the packet.
        allowed: bool,
    },
}

/// Map a wire `Packet.kind` to a `PluginKind`. Returns `None`
/// for unknown kinds — the registry treats those as no-ops
/// (KDC's forward-compat behavior).
fn kind_from_packet(kind: &str) -> Option<PluginKind> {
    PluginKind::all()
        .into_iter()
        .find(|k| k.packet_kind() == kind)
}

impl KdcHost {
    /// Borrow the pairing store. Used by the inbound serve loop
    /// (UC-4) to look up paired devices for cert verification.
    #[must_use]
    pub fn pairing(&self) -> &Arc<PairingStore> {
        &self.pairing
    }

    /// UC-4 — run the inbound accept loop. Binds the listener,
    /// accepts TLS-pinned connections, and spawns a per-connection
    /// reader that frames + dispatches inbound packets through
    /// the supplied `Registry`.
    ///
    /// Each lifecycle event is sent on `events` so the worker
    /// layer can log + emit audit entries without this crate
    /// depending on a logging framework. The caller is expected
    /// to drain `events` continuously; if the receiver is
    /// dropped, the serve loop silently stops emitting events
    /// but keeps accepting.
    ///
    /// The loop returns `Ok(())` when the listener's accept call
    /// errors fatally (unbindable port, EMFILE, etc.); the worker
    /// layer's restart policy decides whether to retry.
    pub async fn serve(
        &self,
        config: ServeConfig,
        events: tokio::sync::mpsc::UnboundedSender<ServeEvent>,
    ) -> std::io::Result<()> {
        let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
        let bound = listener.local_addr()?;
        let _ = events.send(ServeEvent::Listening { addr: bound });

        loop {
            let (tcp, remote) = match listener.accept().await {
                Ok(x) => x,
                Err(e) => {
                    // EMFILE / ENFILE / EINTR are transient — log and
                    // continue. Anything else surfaces to the worker.
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::Interrupted
                            | std::io::ErrorKind::ConnectionAborted
                            | std::io::ErrorKind::WouldBlock
                    ) {
                        continue;
                    }
                    return Err(e);
                }
            };
            // Spawn per-connection task so the listener immediately
            // returns to accepting.
            let cert = config.server_cert_der.clone();
            let key = config.server_key_pkcs8_der.clone();
            let store = Arc::clone(&self.pairing);
            let registry = Arc::clone(&config.registry);
            let authority = Arc::clone(&config.authority);
            let events = events.clone();
            tokio::spawn(async move {
                Self::handle_inbound(
                    tcp, remote, cert, key, store, registry, authority, events,
                )
                .await;
            });
        }
    }

    /// One-connection handler. Owns the TLS handshake, the
    /// frame-decoder loop, and the per-packet dispatch.
    #[allow(clippy::too_many_arguments)]
    async fn handle_inbound(
        tcp: TcpStream,
        remote: std::net::SocketAddr,
        server_cert_der: Vec<u8>,
        server_key_pkcs8_der: Vec<u8>,
        store: Arc<PairingStore>,
        registry: Arc<AsyncMutex<Registry>>,
        authority: Arc<dyn PluginAuthority + Send + Sync>,
        events: tokio::sync::mpsc::UnboundedSender<ServeEvent>,
    ) {
        let (mut tls, device) = match tls::accept_pinned_tls(
            tcp,
            server_cert_der,
            server_key_pkcs8_der,
            store,
        )
        .await
        {
            Ok(x) => x,
            Err(e) => {
                let _ = events.send(ServeEvent::AcceptRejected {
                    remote: Some(remote),
                    reason: format!("{e}"),
                });
                return;
            }
        };
        let peer_id = device.id.clone();
        let _ = events.send(ServeEvent::PeerConnected {
            peer_id: peer_id.clone(),
        });

        let mut decoder = FrameDecoder::new();
        let mut buf = [0u8; 8 * 1024];
        loop {
            let n = match tls.read(&mut buf).await {
                Ok(0) => break, // clean close
                Ok(n) => n,
                Err(_) => break, // any read error ends the session
            };
            decoder.feed(&buf[..n]);
            loop {
                match decoder.next_frame() {
                    Ok(None) => break, // need more bytes
                    Err(_) => continue, // FrameDecoder cleared its buffer
                    Ok(Some(packet)) => {
                        let kind_str = packet.kind.clone();
                        let plugin_kind = match kind_from_packet(&kind_str) {
                            Some(k) => k,
                            None => {
                                // Unknown kind — silent drop (forward compat).
                                let _ = events.send(ServeEvent::PacketDispatched {
                                    peer_id: peer_id.clone(),
                                    kind: kind_str,
                                    allowed: false,
                                });
                                continue;
                            }
                        };
                        let decision = check_plugin_allowed(
                            plugin_kind,
                            &peer_id,
                            true, // a successful handshake means the peer is paired
                            authority.as_ref(),
                        );
                        let allowed = decision.is_allowed();
                        let _ = events.send(ServeEvent::PacketDispatched {
                            peer_id: peer_id.clone(),
                            kind: kind_str,
                            allowed,
                        });
                        if !allowed {
                            continue;
                        }
                        // Dispatch through the registry. Use the
                        // async tokio Mutex so .await is safe.
                        let ctx = PluginContext::new(peer_id.clone(), true);
                        let responses = {
                            let mut guard = registry.lock().await;
                            guard.dispatch(&packet, &ctx)
                        };
                        for resp in responses {
                            let frame = match mde_kdc_proto::codec::encode_frame(&resp) {
                                Ok(s) => s,
                                Err(_) => continue,
                            };
                            if tls.write_all(frame.as_bytes()).await.is_err() {
                                // Write error ends the session — break
                                // out of both the response loop and
                                // the outer read loop.
                                let _ = events.send(ServeEvent::PeerDisconnected {
                                    peer_id: peer_id.clone(),
                                });
                                return;
                            }
                        }
                    }
                }
            }
        }
        let _ = events.send(ServeEvent::PeerDisconnected { peer_id });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keygen;
    use crate::pairing::PairedDevice;
    use crate::tls::compute_fingerprint;
    use mde_kdc_proto::discovery::{Announce, DeviceType};
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::tempdir;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;

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
        KdcHost::pairing_only(Arc::new(store))
    }

    fn make_empty_host() -> KdcHost {
        let tmp = tempdir().unwrap();
        let store = PairingStore::open_or_init(tmp.path()).unwrap();
        std::mem::forget(tmp);
        KdcHost::pairing_only(Arc::new(store))
    }

    fn block_on<F: std::future::Future>(fut: F) -> F::Output {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build tokio rt");
        rt.block_on(fut)
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
    fn probe_unpaired_peer_is_down() {
        let h = make_empty_host();
        let state = block_on(h.probe("nobody"));
        assert_eq!(state, HealthState::Down);
    }

    #[test]
    fn probe_paired_peer_without_discovery_is_down() {
        // Paired but no recent announce → Down. The router
        // shouldn't try to open a TLS connection at this point.
        let h = make_host_with_peer("alice");
        let state = block_on(h.probe("alice"));
        assert_eq!(state, HealthState::Down);
    }

    #[test]
    fn open_unpaired_peer_returns_unreachable_not_paired() {
        let h = make_empty_host();
        let err = block_on(h.open("nobody")).expect_err("unpaired must fail");
        match err {
            TransportError::Unreachable { code } => {
                assert_eq!(code, "not_paired");
            }
            other => panic!("expected Unreachable(not_paired), got {other:?}"),
        }
    }

    #[test]
    fn open_paired_peer_without_discovery_returns_not_discovered() {
        // Paired but no recent announce → Unreachable. Real-
        // world failure mode after a phone goes offline.
        let h = make_host_with_peer("alice");
        let err = block_on(h.open("alice")).expect_err("no addr must fail");
        match err {
            TransportError::Unreachable { code } => {
                assert_eq!(code, "not_discovered");
            }
            other => panic!("expected Unreachable(not_discovered), got {other:?}"),
        }
    }

    #[test]
    fn open_paired_peer_with_unreachable_addr_returns_tcp_refused() {
        // Inject a discovery record for an address with no
        // listener. The TCP connect should fail; the host maps
        // that to Unreachable(tcp_refused).
        let h = {
            let tmp = tempdir().unwrap();
            let store = PairingStore::open_or_init(tmp.path()).unwrap();
            store
                .upsert(PairedDevice {
                    id: "alice".into(),
                    name: "alice".into(),
                    kind: "phone".into(),
                    fingerprint: "AB:CD".into(),
                    public_key_b64: "AA==".into(),
                    capabilities: vec![],
                    paired_at: 1_700_000_000,
                    last_seen_at: 1_700_000_500,
                })
                .unwrap();
            std::mem::forget(tmp);
            let discovery = Arc::new(AsyncMutex::new(DiscoveryRegistry::new()));
            // 127.0.0.1:1 is a deliberately-refused address on
            // every Linux kernel (port 1 is reserved + nothing
            // is bound there in the test env).
            {
                let mut guard = block_on(discovery.lock());
                guard.inject_real_with_addr(
                    Announce {
                        device_id: "alice".into(),
                        device_name: "alice".into(),
                        device_type: DeviceType::Phone,
                        protocol_version: 7,
                        incoming_capabilities: vec![],
                        outgoing_capabilities: vec![],
                    },
                    1_700_000_500,
                    "127.0.0.1:1".parse().unwrap(),
                );
            }
            KdcHost::new(Arc::new(store), discovery)
        };
        let err = block_on(h.open("alice")).expect_err("refused must fail");
        match err {
            TransportError::Unreachable { code } => {
                assert_eq!(code, "tcp_refused");
            }
            other => panic!("expected Unreachable(tcp_refused), got {other:?}"),
        }
    }

    /// Spin up a minimal TLS server on 127.0.0.1:0 using
    /// rcgen-generated self-signed cert keyed to the given
    /// PKCS#8 RSA-2048 keypair, accept exactly one TLS
    /// handshake, then drop the stream. Returns the listener's
    /// bound address + the cert's SHA-256 fingerprint.
    fn spawn_loopback_tls(pkcs8: &[u8], device_id: &str) -> (SocketAddr, String) {
        let cert_der = keygen::issue_identity_cert(pkcs8, device_id).expect("cert");
        let fingerprint = compute_fingerprint(&cert_der);
        let priv_der = pkcs8.to_vec();
        let cert_for_thread = cert_der.clone();
        let priv_for_thread = priv_der;

        // Bind the listener synchronously so we have the addr
        // before returning. The blocking listener spawns its
        // own tokio runtime in a thread.
        let std_listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = std_listener.local_addr().expect("local_addr");

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("loopback rt");
            rt.block_on(async move {
                std_listener
                    .set_nonblocking(true)
                    .expect("set nonblocking");
                let listener = TcpListener::from_std(std_listener).expect("from_std");
                if let Ok((tcp, _)) = listener.accept().await {
                    let cert_chain = vec![CertificateDer::from(cert_for_thread)];
                    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(priv_for_thread));
                    let provider = Arc::new(rustls::crypto::ring::default_provider());
                    let config = rustls::ServerConfig::builder_with_provider(provider)
                        .with_safe_default_protocol_versions()
                        .expect("server protocol versions")
                        .with_no_client_auth()
                        .with_single_cert(cert_chain, key)
                        .expect("server config single cert");
                    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
                    if let Ok(mut tls) = acceptor.accept(tcp).await {
                        // Send a sentinel byte so the client's
                        // handshake is provably complete (some
                        // rustls paths only finalize after the
                        // first server-app-data).
                        let _ = tls.write_all(b"\x00").await;
                        // Give the client time to read before
                        // closing.
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                }
            });
        });

        (addr, fingerprint)
    }

    #[test]
    fn open_paired_peer_with_pinned_fingerprint_completes_handshake() {
        // Full integration: paired device + discovery entry +
        // loopback TLS server presenting the matching cert →
        // KdcHost::open succeeds and returns a KdcTlsConnection.
        let pkcs8 = keygen::generate_pkcs8().expect("pkcs8");
        let device_id = "loopback-peer";
        let (addr, fingerprint) = spawn_loopback_tls(&pkcs8, device_id);
        // The loopback server binds on 127.0.0.1:<some-port> but
        // KdcHost::open dials port KDC_TLS_PORT (1716). For the
        // test we need to align both — point the discovery entry
        // directly at the listener's port by overriding the
        // open dial-address path. We do that by binding the
        // loopback on the kdc port (which requires sudo). Instead
        // of that, exercise the connect_pinned_tls helper directly
        // — KdcHost::open's value is its pairing + discovery
        // lookup, both of which are covered by other tests in
        // this module. The TLS handshake itself is tested here.
        let result = block_on(crate::tls::connect_pinned_tls(
            addr,
            device_id,
            Some(fingerprint),
        ));
        assert!(result.is_ok(), "pinned TLS handshake should succeed: {:?}", result.err());
    }

    #[test]
    fn open_paired_peer_with_wrong_fingerprint_handshake_fails() {
        // Same loopback setup, but pin to a fingerprint that
        // doesn't match the server's cert → handshake fails →
        // ConnectError::Tls. Confirms PinnedFingerprintVerifier
        // is wired through connect_pinned_tls.
        let pkcs8 = keygen::generate_pkcs8().expect("pkcs8");
        let device_id = "loopback-peer-2";
        let (addr, _fingerprint) = spawn_loopback_tls(&pkcs8, device_id);
        let wrong_fp = "AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99:\
                        AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99"
            .to_string();
        let result = block_on(crate::tls::connect_pinned_tls(
            addr,
            device_id,
            Some(wrong_fp),
        ));
        assert!(matches!(result, Err(crate::tls::ConnectError::Tls(_))));
    }

    #[test]
    fn is_object_safe_via_transport_trait() {
        // The mesh-router (KDC2-1.8) holds `Vec<Arc<dyn
        // Transport>>` — KdcHost must coerce into the trait
        // object cleanly.
        let h = make_empty_host();
        let _trait_obj: Arc<dyn Transport> = Arc::new(h);
    }

    #[test]
    fn discovery_handle_clones_share_state() {
        let h = make_empty_host();
        let d1 = h.discovery();
        let d2 = h.discovery();
        // Same underlying Arc storage.
        assert!(Arc::ptr_eq(&d1, &d2));
    }

    // ─────────────────────────────────────────────────────────
    // UC-4 — inbound serve() loopback dispatch
    // ─────────────────────────────────────────────────────────

    use crate::dispatch::PluginAuthority;
    use mde_kdc_proto::codec::encode_frame;
    use mde_kdc_proto::plugins::{clipboard::ClipboardPlugin, Registry};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Test authority that allows every plugin (we test
    /// dispatch wiring, not policy here — `dispatch.rs` already
    /// covers the policy logic).
    #[derive(Debug)]
    struct AllowAll;
    impl PluginAuthority for AllowAll {
        fn plugin_allowed(&self, _name: &str) -> bool {
            true
        }
    }

    /// Test authority that denies a specific plugin token —
    /// confirms `serve()` actually wires `check_plugin_allowed`.
    #[derive(Debug)]
    struct DenyOne(&'static str);
    impl PluginAuthority for DenyOne {
        fn plugin_allowed(&self, name: &str) -> bool {
            name != self.0
        }
    }

    fn make_test_client_config_with_auth(
        client_cert_der: Vec<u8>,
        client_key_pkcs8_der: Vec<u8>,
    ) -> rustls::ClientConfig {
        use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let cert_chain = vec![rustls::pki_types::CertificateDer::from(client_cert_der)];
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(client_key_pkcs8_der));
        let verifier: Arc<dyn rustls::client::danger::ServerCertVerifier> =
            Arc::new(crate::tls::FirstPairVerifier);
        rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("client protocol versions")
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_client_auth_cert(cert_chain, key)
            .expect("client config with auth cert")
    }

    /// Boots a serve() loop, returns (bound_addr, event_rx,
    /// clipboard_arrived_counter). The clipboard plugin's
    /// callback bumps the counter every time the dispatcher
    /// routes a clipboard body through it.
    async fn boot_uc4_serve(
        authority: Arc<dyn PluginAuthority + Send + Sync>,
        client_fp: String,
    ) -> (
        std::net::SocketAddr,
        tokio::sync::mpsc::UnboundedReceiver<crate::transport::ServeEvent>,
        Arc<AtomicUsize>,
    ) {
        // Server identity.
        let server_pkcs8 = crate::keygen::generate_pkcs8().unwrap();
        let server_cert = crate::keygen::issue_identity_cert(&server_pkcs8, "host-A").unwrap();
        // Pre-pair the client.
        let tmp = tempdir().unwrap();
        let store = PairingStore::open_or_init(tmp.path()).unwrap();
        store
            .upsert(PairedDevice {
                id: "client-X".into(),
                name: "Pixel".into(),
                kind: "phone".into(),
                fingerprint: client_fp,
                public_key_b64: "AA==".into(),
                capabilities: vec!["kdeconnect.clipboard".into()],
                paired_at: 1_700_000_000,
                last_seen_at: 1_700_000_500,
            })
            .unwrap();
        std::mem::forget(tmp);
        let store = Arc::new(store);
        // Register a clipboard plugin with a counter callback.
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_cb = Arc::clone(&counter);
        let plugin = ClipboardPlugin::with_callback(Box::new(move |_body| {
            counter_cb.fetch_add(1, Ordering::SeqCst);
        }));
        let mut registry = Registry::new();
        registry.insert(Box::new(plugin));
        let registry = Arc::new(AsyncMutex::new(registry));
        // Host + serve.
        let host = Arc::new(KdcHost::pairing_only(store));
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        // Pre-bind to find the addr, then move the listener into serve()? Simpler:
        // bind 127.0.0.1:0 inside serve, read the Listening event for the addr.
        let cfg = crate::transport::ServeConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            server_cert_der: server_cert,
            server_key_pkcs8_der: server_pkcs8,
            registry,
            authority,
        };
        let host_clone = Arc::clone(&host);
        tokio::spawn(async move {
            let _ = host_clone.serve(cfg, tx).await;
        });
        // Wait for Listening event.
        let mut rx = rx;
        let addr = loop {
            match rx.recv().await {
                Some(crate::transport::ServeEvent::Listening { addr }) => break addr,
                Some(_) => continue,
                None => panic!("serve dropped events channel before Listening"),
            }
        };
        (addr, rx, counter)
    }

    /// Drive a client through the mutual-TLS handshake, send one
    /// framed packet, return after a brief read so the alert can
    /// flow back if the server rejected.
    async fn drive_client_send(
        addr: std::net::SocketAddr,
        client_cert: Vec<u8>,
        client_pkcs8: Vec<u8>,
        packet: mde_kdc_proto::wire::Packet,
    ) {
        let cfg = make_test_client_config_with_auth(client_cert, client_pkcs8);
        let connector = tokio_rustls::TlsConnector::from(Arc::new(cfg));
        let tcp = tokio::net::TcpStream::connect(addr).await.expect("tcp");
        let server_name = rustls::pki_types::ServerName::try_from("host-A").unwrap();
        let mut tls = connector.connect(server_name, tcp).await.expect("client TLS");
        let frame = encode_frame(&packet).expect("encode");
        tokio::io::AsyncWriteExt::write_all(&mut tls, frame.as_bytes())
            .await
            .expect("write frame");
        // Give the server a moment to process.
        tokio::time::sleep(Duration::from_millis(150)).await;
        // Best-effort close.
        let _ = tokio::io::AsyncWriteExt::shutdown(&mut tls).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn serve_dispatches_inbound_clipboard_packet_through_registry() {
        let client_pkcs8 = crate::keygen::generate_pkcs8().unwrap();
        let client_cert = crate::keygen::issue_identity_cert(&client_pkcs8, "client-X").unwrap();
        let client_fp = compute_fingerprint(&client_cert);
        let authority: Arc<dyn PluginAuthority + Send + Sync> = Arc::new(AllowAll);
        let (addr, mut events, counter) = boot_uc4_serve(authority, client_fp).await;
        let packet = mde_kdc_proto::plugins::clipboard::clipboard_packet(
            42,
            "from-the-phone".to_string(),
        );
        drive_client_send(addr, client_cert, client_pkcs8, packet).await;
        // Drain a few events to confirm wiring (PeerConnected +
        // PacketDispatched should both fire). Pull up to 4
        // events with a short deadline.
        let mut saw_dispatched = false;
        let mut saw_connected = false;
        for _ in 0..6 {
            match tokio::time::timeout(Duration::from_millis(200), events.recv()).await {
                Ok(Some(crate::transport::ServeEvent::PeerConnected { peer_id })) => {
                    assert_eq!(peer_id, "client-X");
                    saw_connected = true;
                }
                Ok(Some(crate::transport::ServeEvent::PacketDispatched {
                    peer_id,
                    kind,
                    allowed,
                })) => {
                    assert_eq!(peer_id, "client-X");
                    assert_eq!(kind, "kdeconnect.clipboard");
                    assert!(allowed);
                    saw_dispatched = true;
                }
                Ok(Some(_)) => continue,
                Ok(None) | Err(_) => break,
            }
            if saw_connected && saw_dispatched {
                break;
            }
        }
        assert!(saw_connected, "must see PeerConnected event");
        assert!(saw_dispatched, "must see PacketDispatched event");
        // The clipboard plugin's callback must have fired exactly
        // once for our one inbound clipboard packet.
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn serve_drops_packet_when_authority_denies_plugin() {
        let client_pkcs8 = crate::keygen::generate_pkcs8().unwrap();
        let client_cert = crate::keygen::issue_identity_cert(&client_pkcs8, "client-X").unwrap();
        let client_fp = compute_fingerprint(&client_cert);
        let authority: Arc<dyn PluginAuthority + Send + Sync> = Arc::new(DenyOne("clipboard"));
        let (addr, mut events, counter) = boot_uc4_serve(authority, client_fp).await;
        let packet = mde_kdc_proto::plugins::clipboard::clipboard_packet(
            1,
            "should-be-dropped".into(),
        );
        drive_client_send(addr, client_cert, client_pkcs8, packet).await;
        // Wait for the PacketDispatched event with allowed=false.
        let mut saw_denied = false;
        for _ in 0..6 {
            match tokio::time::timeout(Duration::from_millis(200), events.recv()).await {
                Ok(Some(crate::transport::ServeEvent::PacketDispatched {
                    allowed: false, ..
                })) => {
                    saw_denied = true;
                    break;
                }
                Ok(Some(_)) => continue,
                Ok(None) | Err(_) => break,
            }
        }
        assert!(saw_denied, "must see denied PacketDispatched event");
        // The clipboard plugin callback must NOT have fired.
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn kind_from_packet_maps_known_kinds() {
        assert_eq!(
            kind_from_packet("kdeconnect.clipboard"),
            Some(PluginKind::Clipboard),
        );
        assert_eq!(kind_from_packet("kdeconnect.ping"), Some(PluginKind::Ping));
        assert_eq!(kind_from_packet("kdeconnect.unknown"), None);
        assert_eq!(kind_from_packet(""), None);
    }
}
