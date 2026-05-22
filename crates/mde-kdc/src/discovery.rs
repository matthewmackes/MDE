//! KDC2-2.10.a + 2.9.a — host-side discovery runners.
//!
//! The pure-data half (encoder + decoder + registry) lives in
//! `mde_kdc_proto::discovery`. This module wires a
//! `tokio::net::UdpSocket` to UDP/1716 so the daemon can:
//!
//!   * Broadcast its own [`mde_kdc_proto::discovery::Announce`]
//!     to `255.255.255.255:1716` every 30 s.
//!
//!   * Receive datagrams from the same port + feed every
//!     decoded peer announce into a
//!     [`DiscoveryRegistry`] via `inject_real`.
//!
//! Both halves are concrete by design — there's no async-trait
//! seam — so the production code reads top-to-bottom. Tests use
//! loopback + the synchronous helpers so they're CI-safe
//! without privileged ports.
//!
//! The mDNS host runner (`_kdeconnect._udp.local.` via
//! mdns-sd) is queued as KDC2-2.9.a — slightly different
//! lifecycle, separate file when it lands.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::Instant;

use mde_kdc_proto::discovery::{
    decode_announce_datagram, decode_mdns_txt_records, encode_announce_datagram,
    encode_mdns_txt_records, Announce, BroadcastError, DiscoveryRegistry, KDC_MDNS_SERVICE_TYPE,
    KDC_UDP_PORT, MAX_BROADCAST_BYTES,
};

/// Broadcast cadence — matches upstream KDE Connect's 30 s
/// re-announce window. Operator-tunable in a future
/// policy.toml knob; baked in for now.
pub const BROADCAST_INTERVAL: Duration = Duration::from_secs(30);

/// Errors the runner may surface during setup. Once `run` is
/// going, transient I/O is logged + skipped — a partially
/// failed broadcast tick must not kill the daemon.
#[derive(Debug)]
pub enum RunnerError {
    /// `UdpSocket::bind` failed (port busy, permission denied,
    /// no socket capability).
    Bind(std::io::Error),
    /// `set_broadcast(true)` failed — required so we can send
    /// to the broadcast address.
    BroadcastFlag(std::io::Error),
}

impl std::fmt::Display for RunnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunnerError::Bind(e) => write!(f, "bind: {e}"),
            RunnerError::BroadcastFlag(e) => write!(f, "broadcast_flag: {e}"),
        }
    }
}

impl std::error::Error for RunnerError {}

/// Async runner for the UDP/1716 broadcast loop.
///
/// `bind_port` defaults to [`KDC_UDP_PORT`] (1716) in
/// production. Tests pass `0` to get an ephemeral port.
pub struct UdpBroadcastRunner {
    /// Live socket.
    socket: Arc<UdpSocket>,
    /// Shared registry the runner feeds with decoded peer
    /// announces.
    registry: Arc<AsyncMutex<DiscoveryRegistry>>,
    /// Our identity to broadcast each tick. Captured at
    /// construction; if the user renames the host, restart the
    /// daemon.
    self_announce: Announce,
}

impl UdpBroadcastRunner {
    /// Bind a UDP socket on `0.0.0.0:bind_port`, flip the
    /// broadcast flag, and return a ready-to-run runner.
    /// Doesn't actually start ticking until `run` is awaited.
    pub async fn bind(
        bind_port: u16,
        self_announce: Announce,
        registry: Arc<AsyncMutex<DiscoveryRegistry>>,
    ) -> Result<Self, RunnerError> {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), bind_port);
        let socket = UdpSocket::bind(addr).await.map_err(RunnerError::Bind)?;
        socket
            .set_broadcast(true)
            .map_err(RunnerError::BroadcastFlag)?;
        Ok(Self {
            socket: Arc::new(socket),
            registry,
            self_announce,
        })
    }

    /// Local-port the socket is actually bound to. Used by
    /// tests that bind to port 0 + need to know where to send.
    pub fn local_port(&self) -> std::io::Result<u16> {
        Ok(self.socket.local_addr()?.port())
    }

    /// One iteration of the broadcast loop. Pure helper so
    /// tests can drive a single tick.
    pub async fn broadcast_once(&self, ts_ms: i64) -> std::io::Result<usize> {
        let datagram = encode_announce_datagram(&self.self_announce, ts_ms)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{e}")))?;
        let target = SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), KDC_UDP_PORT);
        self.socket.send_to(&datagram, target).await
    }

    /// Receive one datagram + decode it into an [`Announce`].
    /// Returns the parsed announce + the sender's address so the
    /// caller can record peer reachability. Returns
    /// `Ok(None)` for a datagram that decoded as the wrong kind
    /// (handled silently — could be a stray clipboard packet
    /// from a misconfigured peer).
    ///
    /// Buffered against `MAX_BROADCAST_BYTES` — bigger datagrams
    /// surface as `WouldBlock`-style discards so a hostile peer
    /// can't OOM the runner.
    pub async fn recv_one(&self) -> std::io::Result<Option<(Announce, SocketAddr)>> {
        let mut buf = vec![0u8; MAX_BROADCAST_BYTES];
        let (n, src) = self.socket.recv_from(&mut buf).await?;
        match decode_announce_datagram(&buf[..n]) {
            Ok(announce) => Ok(Some((announce, src))),
            Err(BroadcastError::WrongPacketKind(_)) => Ok(None),
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("{e}"),
            )),
        }
    }

    /// Drain one received announce into the registry. Glue
    /// between [`recv_one`] and the shared [`DiscoveryRegistry`]
    /// — caller-visible because tests want to assert the
    /// registry got fed.
    pub async fn ingest_one(&self, announce: Announce, now_ms: i64) {
        let mut guard = self.registry.lock().await;
        guard.inject_real(announce, now_ms);
    }

    /// KDC2-3.2.b — drain a received announce *with* its source
    /// address into the registry so `KdcHost::open(peer_id)` can
    /// later resolve where to TCP-connect.
    pub async fn ingest_one_with_addr(
        &self,
        announce: Announce,
        now_ms: i64,
        source_addr: SocketAddr,
    ) {
        let mut guard = self.registry.lock().await;
        guard.inject_real_with_addr(announce, now_ms, source_addr);
    }

    /// Main loop. Concurrent broadcast tick + recv loop. Runs
    /// until the supplied shutdown future resolves. Returns
    /// `Ok(())` on clean shutdown.
    pub async fn run(
        self: Arc<Self>,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), std::io::Error> {
        let mut interval = tokio::time::interval(BROADCAST_INTERVAL);
        let started = Instant::now();
        loop {
            tokio::select! {
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        return Ok(());
                    }
                }
                _ = interval.tick() => {
                    let ts_ms = started.elapsed().as_millis() as i64;
                    let _ = self.broadcast_once(ts_ms).await;
                }
                got = self.recv_one() => {
                    if let Ok(Some((announce, src))) = got {
                        let now_ms = started.elapsed().as_millis() as i64;
                        // KDC2-3.2.b — cache the sender's
                        // address so KdcHost::open can resolve
                        // it later without re-binding the UDP
                        // socket.
                        self.ingest_one_with_addr(announce, now_ms, src).await;
                    }
                }
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────────
// KDC2-2.9.a — mDNS host runner.
//
// `mdns-sd` runs its own daemon thread internally. We wrap the
// service-daemon handle + browse-result receiver so the caller
// can:
//
//   * `announce()` — publish our identity under
//     `_kdeconnect._udp.local.` with TXT records produced by
//     `encode_mdns_txt_records`.
//
//   * `pump_into_registry()` — drain one ServiceResolved event
//     from the browser channel, decode its TXT records into an
//     `Announce`, and call `DiscoveryRegistry::inject_real`.
//
// The runner is async-friendly but doesn't own a tokio task —
// the caller composes it into a `select!` arm alongside the UDP
// runner. Keeps the lifetime + cancellation explicit.
// ──────────────────────────────────────────────────────────────────

/// mDNS host runner. Wraps an `mdns_sd::ServiceDaemon` + a
/// browser receiver tuned for the KDC service type.
pub struct MdnsRunner {
    daemon: mdns_sd::ServiceDaemon,
    browser: flume::Receiver<mdns_sd::ServiceEvent>,
    registry: Arc<AsyncMutex<DiscoveryRegistry>>,
}

/// Errors the mDNS runner may surface.
#[derive(Debug)]
pub enum MdnsError {
    /// `ServiceDaemon::new` failed (no network namespace, no
    /// multicast interface).
    Daemon(String),
    /// `browse` registration failed.
    Browse(String),
    /// `register` of our own service failed.
    Register(String),
    /// TXT records decoded but produced an invalid Announce.
    Decode(String),
}

impl std::fmt::Display for MdnsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MdnsError::Daemon(s) => write!(f, "daemon: {s}"),
            MdnsError::Browse(s) => write!(f, "browse: {s}"),
            MdnsError::Register(s) => write!(f, "register: {s}"),
            MdnsError::Decode(s) => write!(f, "decode: {s}"),
        }
    }
}

impl std::error::Error for MdnsError {}

impl MdnsRunner {
    /// Spin up an mdns-sd daemon + start browsing for
    /// `_kdeconnect._udp.local.`.
    pub fn start(registry: Arc<AsyncMutex<DiscoveryRegistry>>) -> Result<Self, MdnsError> {
        let daemon =
            mdns_sd::ServiceDaemon::new().map_err(|e| MdnsError::Daemon(format!("{e}")))?;
        let browser = daemon
            .browse(KDC_MDNS_SERVICE_TYPE)
            .map_err(|e| MdnsError::Browse(format!("{e}")))?;
        Ok(Self {
            daemon,
            browser,
            registry,
        })
    }

    /// Publish our identity under `_kdeconnect._udp.local.` so
    /// browsers (phones running stock KDE Connect, other MDE
    /// peers) resolve us. `host_name` is the local interface's
    /// hostname (e.g. `lab-01.local.`); `port` is the TCP/TLS
    /// port we listen on for incoming pair handshakes
    /// (typically `KDC_UDP_PORT` aka 1716 — the same number
    /// upstream uses for both UDP and TCP).
    pub fn announce(
        &self,
        announce: &Announce,
        host_name: &str,
        port: u16,
    ) -> Result<(), MdnsError> {
        let txt: Vec<(String, String)> = encode_mdns_txt_records(announce);
        let info = mdns_sd::ServiceInfo::new(
            KDC_MDNS_SERVICE_TYPE,
            &announce.device_id,
            host_name,
            (), // let mdns-sd auto-detect the addresses
            port,
            &txt[..],
        )
        .map_err(|e| MdnsError::Register(format!("info: {e}")))?;
        self.daemon
            .register(info)
            .map_err(|e| MdnsError::Register(format!("register: {e}")))
    }

    /// Drain one ServiceResolved event from the browser channel
    /// and ingest the decoded Announce into the registry. Other
    /// event kinds (SearchStarted, etc.) are skipped silently.
    /// Returns the device_id that was ingested, or None if no
    /// event was available before the optional timeout.
    pub async fn pump_into_registry(
        &self,
        wait: Option<Duration>,
        now_ms: i64,
    ) -> Result<Option<String>, MdnsError> {
        let event_opt = match wait {
            Some(d) => match tokio::time::timeout(d, recv_async(&self.browser)).await {
                Ok(Ok(ev)) => Some(ev),
                _ => None,
            },
            None => self.browser.try_recv().ok(),
        };
        let Some(event) = event_opt else {
            return Ok(None);
        };
        if let mdns_sd::ServiceEvent::ServiceResolved(info) = event {
            let pairs: Vec<(&str, &str)> = info
                .get_properties()
                .iter()
                .map(|p| (p.key(), p.val_str()))
                .collect();
            let announce =
                decode_mdns_txt_records(pairs).map_err(|e| MdnsError::Decode(format!("{e}")))?;
            let device_id = announce.device_id.clone();
            self.registry.lock().await.inject_real(announce, now_ms);
            return Ok(Some(device_id));
        }
        Ok(None)
    }

    /// Best-effort shutdown of the daemon thread. Drops any
    /// pending browse events.
    pub fn shutdown(self) {
        let _ = self.daemon.shutdown();
    }
}

/// Tiny wrapper that turns flume's blocking recv into an async
/// future via a oneshot bridge. mdns-sd uses flume's
/// `Receiver`, which exposes a `recv_async` method natively;
/// this wrapper picks it up.
async fn recv_async(
    rx: &flume::Receiver<mdns_sd::ServiceEvent>,
) -> Result<mdns_sd::ServiceEvent, flume::RecvError> {
    rx.recv_async().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use mde_kdc_proto::discovery::DeviceType;
    use mde_kdc_proto::PROTOCOL_VERSION;

    fn sample_announce(id: &str) -> Announce {
        Announce {
            device_id: id.into(),
            device_name: format!("test-host {}", mde_kdc_proto::MDE_DEVICE_NAME_SUFFIX),
            device_type: DeviceType::Desktop,
            protocol_version: PROTOCOL_VERSION,
            incoming_capabilities: vec!["kdeconnect.ping".into()],
            outgoing_capabilities: vec!["kdeconnect.ping".into()],
        }
    }

    fn new_registry() -> Arc<AsyncMutex<DiscoveryRegistry>> {
        Arc::new(AsyncMutex::new(DiscoveryRegistry::new()))
    }

    #[tokio::test(flavor = "current_thread")]
    async fn bind_succeeds_on_ephemeral_port() {
        let r = UdpBroadcastRunner::bind(0, sample_announce("me"), new_registry())
            .await
            .unwrap();
        let port = r.local_port().unwrap();
        assert!(port > 0, "ephemeral bind returned port 0");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn ingest_one_records_into_registry() {
        let registry = new_registry();
        let r = UdpBroadcastRunner::bind(0, sample_announce("me"), Arc::clone(&registry))
            .await
            .unwrap();
        r.ingest_one(sample_announce("peer-A"), 1000).await;
        let guard = registry.lock().await;
        assert_eq!(guard.relayer_for("peer-A"), Some("self"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn round_trip_broadcast_and_decode() {
        // Sender + receiver bound to two ephemeral ports on
        // loopback. The sender shoots a datagram at the
        // receiver's port; the receiver decodes + ingests.
        let sender_registry = new_registry();
        let receiver_registry = new_registry();
        let sender = UdpBroadcastRunner::bind(0, sample_announce("sender"), sender_registry)
            .await
            .unwrap();
        let receiver =
            UdpBroadcastRunner::bind(0, sample_announce("recv"), Arc::clone(&receiver_registry))
                .await
                .unwrap();
        let recv_port = receiver.local_port().unwrap();

        // Encode + send directly to the receiver's loopback
        // port (skips the broadcast-address path which CI
        // sandboxes block).
        let bytes = encode_announce_datagram(&sample_announce("sender"), 100).unwrap();
        let target = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), recv_port);
        sender.socket.send_to(&bytes, target).await.unwrap();

        // Receiver pulls + ingests.
        let (got, _src) = tokio::time::timeout(Duration::from_secs(2), receiver.recv_one())
            .await
            .expect("recv timed out")
            .unwrap()
            .expect("received None");
        assert_eq!(got.device_id, "sender");
        receiver.ingest_one(got, 200).await;
        let guard = receiver_registry.lock().await;
        assert_eq!(guard.relayer_for("sender"), Some("self"));
    }

    // ─────────────────────────────────────────────────────────
    // KDC2-2.9.a — mDNS runner tests live below the impl.
    // ─────────────────────────────────────────────────────────

    #[tokio::test(flavor = "current_thread")]
    async fn recv_one_silently_drops_wrong_kind_datagrams() {
        // A peer broadcasts a clipboard packet on UDP/1716 by
        // mistake. The runner must not treat that as an error
        // (which would log noise) — it returns Ok(None).
        let registry = new_registry();
        let receiver = UdpBroadcastRunner::bind(0, sample_announce("recv"), registry)
            .await
            .unwrap();
        let recv_port = receiver.local_port().unwrap();

        let bad_packet = mde_kdc_proto::wire::Packet {
            id: 1,
            kind: "kdeconnect.clipboard".into(),
            body: serde_json::json!({}),
            ..Default::default()
        };
        let mut bytes = serde_json::to_vec(&bad_packet).unwrap();
        bytes.push(b'\n');
        let sender_socket = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .await
            .unwrap();
        let target = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), recv_port);
        sender_socket.send_to(&bytes, target).await.unwrap();

        let result = tokio::time::timeout(Duration::from_secs(2), receiver.recv_one())
            .await
            .expect("recv timed out")
            .unwrap();
        assert!(result.is_none(), "wrong-kind packet should yield None");
    }

    // ─────────────────────────────────────────────────────────
    // KDC2-2.9.a — mDNS runner tests
    //
    // mdns-sd talks real multicast under the hood, which CI
    // sandboxes often disallow. The tests below construct the
    // daemon + drain via `try_recv` so they don't depend on a
    // working multicast path — they exercise the wiring, not
    // the network.
    // ─────────────────────────────────────────────────────────

    #[tokio::test(flavor = "current_thread")]
    async fn mdns_runner_starts_and_browses_without_panicking() {
        let registry = new_registry();
        let r = MdnsRunner::start(Arc::clone(&registry));
        // Either succeeds, or fails cleanly with MdnsError on a
        // sandboxed CI. Both outcomes are within the test's
        // expectation; the lock is "no panic + the error type
        // is well-formed."
        match r {
            Ok(runner) => {
                // try_recv with no wait returns Ok(None)
                // immediately because no peer is announcing.
                let drained = runner.pump_into_registry(None, 1000).await.unwrap();
                assert!(drained.is_none(), "fresh browser must be empty");
                runner.shutdown();
            }
            Err(e) => {
                let msg = format!("{e}");
                assert!(msg.starts_with("daemon: ") || msg.starts_with("browse: "));
            }
        }
    }
}
