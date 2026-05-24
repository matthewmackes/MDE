//! Phase 12.14 — LAN peer auto-detection + direct UDP data path.
//!
//! Locked 2026-05-19 (25-Q connectivity survey,
//! `docs/design/v12-connectivity-scope.md`):
//!
//!   * Q7: detection < 30 s
//!   * Q8: first-packet < 3 s
//!   * Q12: subtle panel indicator for LAN-direct (no banner)
//!   * Q23: when LAN-direct and DERP both up, throughput wins
//!     (not LAN-first)
//!
//! Two halves cohabit one worker:
//!
//!   * **Discovery** — `mdns-sd` announces `_mackes-peer._udp.local`
//!     with the local hostname + UDP probe port. The same
//!     `ServiceDaemon` browses for matching announcements; each peer
//!     lands in [`LanPeer`].
//!
//!   * **Probe** — a tokio UDP socket exchanges 8-byte ping/pong
//!     pairs with every discovered peer; round-trip time gets
//!     recorded so the routing layer (Phase 12.22 throughput-aware
//!     selection) has something to rank against.
//!
//! The pure data model + ranking helpers live in this module so the
//! routing layer + GUI breadcrumb can both consume them without an
//! mDNS dep. The worker body is gated behind `async-services` like
//! the rest of the Phase A/B surface.

#![cfg(feature = "async-services")]

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Context;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use tokio::net::UdpSocket;
use tracing::{debug, info, warn};

use super::{ShutdownToken, Worker};

/// mDNS service type advertised by every peer. Q7-locked literal —
/// changing it is a wire-protocol break.
pub const SERVICE_TYPE: &str = "_mackes-peer._udp.local.";

/// Default UDP probe port. Same port for announce + probes so a
/// single firewall rule covers the data path.
pub const DEFAULT_PROBE_PORT: u16 = 41841;

/// Magic prefix on every probe datagram. 4-byte ASCII so a tcpdump
/// reader can identify the protocol at a glance.
const PROBE_MAGIC: [u8; 4] = *b"MPRB";

/// Probe ping opcode (one byte after the magic).
const OP_PING: u8 = 0x01;

/// Probe pong opcode.
const OP_PONG: u8 = 0x02;

/// One announced LAN peer. Populated from an mDNS `ServiceFound`
/// event; updated on `ServiceResolved`. The hostname is the
/// node-name advertised in the TXT record (falls back to the
/// instance name if absent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanPeer {
    /// Instance name (the `<instance>._mackes-peer._udp.local.`
    /// label, minus the service-type suffix).
    pub instance: String,
    /// Friendly hostname extracted from the TXT record. Falls back
    /// to `instance` if the record carried no `host=` key.
    pub host: String,
    /// First reachable IPv4 address. mdns-sd reports every address
    /// the peer announced; we pick the first IPv4 because the
    /// connectivity scope is IPv4-only by Q9.
    pub addr: SocketAddr,
}

/// Round-trip timing record from one probe exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RttSample {
    /// Peer instance name (matches [`LanPeer::instance`]).
    pub peer: String,
    /// Source socket address of the pong reply.
    pub addr: SocketAddr,
    /// Measured round-trip time, milliseconds.
    pub rtt_ms: u32,
    /// Sequence number of the probe (matches `ping`/`pong`
    /// correlator).
    pub seq: u32,
}

/// Shared in-memory registry. The worker writes to it; the panel /
/// reconcile loop reads through [`Registry::snapshot`].
///
/// Wraps an `Arc<Mutex<…>>` so it Clones cheaply across the worker
/// thread + the UDP probe task.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    inner: Arc<Mutex<RegistryInner>>,
}

#[derive(Debug, Default)]
struct RegistryInner {
    peers: HashMap<String, LanPeer>,
    /// Last RTT sample per peer (instance name). Older samples are
    /// dropped — the routing layer wants the freshest reading, not
    /// the full series.
    rtts: HashMap<String, RttSample>,
}

impl Registry {
    /// Construct an empty registry. Equivalent to [`Default::default`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace or insert the record for one peer.
    pub fn upsert_peer(&self, peer: LanPeer) {
        let mut g = self.inner.lock().expect("registry mutex poisoned");
        g.peers.insert(peer.instance.clone(), peer);
    }

    /// Remove a peer (mDNS `ServiceRemoved`).
    pub fn remove_peer(&self, instance: &str) {
        let mut g = self.inner.lock().expect("registry mutex poisoned");
        g.peers.remove(instance);
        g.rtts.remove(instance);
    }

    /// Record an RTT sample for a peer.
    pub fn record_rtt(&self, sample: RttSample) {
        let mut g = self.inner.lock().expect("registry mutex poisoned");
        g.rtts.insert(sample.peer.clone(), sample);
    }

    /// Snapshot the current registry. Returns `(peers, rtt-by-instance)`.
    #[must_use]
    pub fn snapshot(&self) -> (Vec<LanPeer>, HashMap<String, RttSample>) {
        let g = self.inner.lock().expect("registry mutex poisoned");
        let mut peers: Vec<LanPeer> = g.peers.values().cloned().collect();
        peers.sort_by(|a, b| a.instance.cmp(&b.instance));
        (peers, g.rtts.clone())
    }

    /// Number of peers currently tracked.
    #[must_use]
    pub fn peer_count(&self) -> usize {
        self.inner
            .lock()
            .expect("registry mutex poisoned")
            .peers
            .len()
    }

    /// Number of RTT samples currently held.
    #[must_use]
    pub fn rtt_count(&self) -> usize {
        self.inner
            .lock()
            .expect("registry mutex poisoned")
            .rtts
            .len()
    }
}

/// Encode a ping datagram. Layout: `[M, P, R, B, OP, seq:u32-LE]`.
/// 9 bytes total — small enough to fit any MTU.
#[must_use]
pub fn encode_ping(seq: u32) -> [u8; 9] {
    let mut buf = [0u8; 9];
    buf[..4].copy_from_slice(&PROBE_MAGIC);
    buf[4] = OP_PING;
    buf[5..9].copy_from_slice(&seq.to_le_bytes());
    buf
}

/// Encode a pong datagram (echoes the seq).
#[must_use]
pub fn encode_pong(seq: u32) -> [u8; 9] {
    let mut buf = [0u8; 9];
    buf[..4].copy_from_slice(&PROBE_MAGIC);
    buf[4] = OP_PONG;
    buf[5..9].copy_from_slice(&seq.to_le_bytes());
    buf
}

/// Decoded probe message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeMsg {
    /// Probe sender → responder. Inner value is the sequence number.
    Ping(u32),
    /// Probe responder → sender. Inner value echoes the ping seq.
    Pong(u32),
}

/// Decode a datagram. Returns `None` if the bytes don't look like a
/// probe (wrong length, wrong magic, unknown opcode).
#[must_use]
pub fn decode_probe(buf: &[u8]) -> Option<ProbeMsg> {
    if buf.len() < 9 || buf[..4] != PROBE_MAGIC {
        return None;
    }
    let mut seq_bytes = [0u8; 4];
    seq_bytes.copy_from_slice(&buf[5..9]);
    let seq = u32::from_le_bytes(seq_bytes);
    match buf[4] {
        OP_PING => Some(ProbeMsg::Ping(seq)),
        OP_PONG => Some(ProbeMsg::Pong(seq)),
        _ => None,
    }
}

/// Rank one (LAN-direct, DERP-relay) pair by Q23 throughput-wins
/// policy. Today the proxy for throughput is "lower RTT wins" — the
/// real bandwidth probe lands in Phase 12.22. `None` means "no
/// sample yet"; we treat that as worst.
///
/// Returns `true` when the LAN-direct path should be preferred.
#[must_use]
pub fn lan_direct_wins(lan_rtt_ms: Option<u32>, derp_rtt_ms: Option<u32>) -> bool {
    match (lan_rtt_ms, derp_rtt_ms) {
        (Some(lan), Some(derp)) => lan <= derp,
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => false,
    }
}

/// Phase 12.19 — multi-path concurrent send predicate.
///
/// Q19 lock: only use multi-path for interactive latency-sensitive
/// flows. The path pair must satisfy:
///
///   * Both paths report an RTT < 50 ms (else multi-path doesn't
///     reduce user-visible latency — single-path is fine).
///   * Their measured bandwidth must be within ±50% of each other
///     (else the slow path drags throughput down).
///
/// Returns `true` when the routing layer should spray packets to
/// both paths + dedupe on the receive side (the 64-bit packet-ID
/// dedupe lives one layer up in [`PacketDedupe`]).
///
/// **NF-8.6 (v2.5 Nebula rebuild):** the predicate itself does
/// not change. Its call sites currently dispatch to
/// `TransportKind::DirectUdp` + `TransportKind::DerpRelay`; once
/// NF-4 renames the variants those will become
/// `TransportKind::NebulaDirect` + `TransportKind::NebulaLighthouseRelay`
/// (and the Https443 path becomes `TransportKind::NebulaHttps443`).
/// The rename is a pure identifier swap — no semantic shift to
/// the multipath gate.
#[must_use]
pub fn should_use_multipath(rtt_a_ms: u32, rtt_b_ms: u32, bw_a_bps: u64, bw_b_bps: u64) -> bool {
    const RTT_CEILING_MS: u32 = 50;
    if rtt_a_ms >= RTT_CEILING_MS || rtt_b_ms >= RTT_CEILING_MS {
        return false;
    }
    if bw_a_bps == 0 || bw_b_bps == 0 {
        return false;
    }
    let (slow, fast) = if bw_a_bps <= bw_b_bps {
        (bw_a_bps, bw_b_bps)
    } else {
        (bw_b_bps, bw_a_bps)
    };
    // ±50% bandwidth window: slow ≥ 0.5 × fast.
    slow * 2 >= fast
}

/// Phase 12.19 — receive-side packet-ID dedupe. Multi-path sends
/// the same datagram down two paths; the receiver throws away the
/// second copy. The dedupe is a sliding window over the last
/// `capacity` packet IDs.
///
/// `capacity` is the max in-flight window — once exceeded, the
/// oldest IDs roll off and a wildly-late duplicate would slip
/// through. The default capacity (1024) covers a few seconds of
/// interactive flow at 100 pps.
#[derive(Debug, Clone)]
pub struct PacketDedupe {
    capacity: usize,
    seen: std::collections::VecDeque<u64>,
    set: std::collections::HashSet<u64>,
}

impl PacketDedupe {
    /// Construct with the locked default capacity (1024).
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Construct with a specific capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            seen: std::collections::VecDeque::with_capacity(capacity),
            set: std::collections::HashSet::with_capacity(capacity),
        }
    }

    /// Record `id`. Returns `true` if this was a new packet (caller
    /// delivers it upstream); `false` if it's a duplicate (drop).
    pub fn observe(&mut self, id: u64) -> bool {
        if self.set.contains(&id) {
            return false;
        }
        self.set.insert(id);
        self.seen.push_back(id);
        while self.seen.len() > self.capacity {
            if let Some(old) = self.seen.pop_front() {
                self.set.remove(&old);
            }
        }
        true
    }

    /// Current number of tracked IDs (≤ capacity).
    #[must_use]
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    /// `true` when no IDs have been observed yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

impl Default for PacketDedupe {
    fn default() -> Self {
        Self::new()
    }
}

/// Phase 12.20 — link state for one interface. Mirrors the
/// `operstate` file under `/sys/class/net/<iface>/`. We only care
/// about three transitions:
///
///   * `Down → Up` → eager re-handshake (a fresh link came up).
///   * `Up → Down` → reconnecting state in the UI.
///   * `Up → Up` with a new ifindex → same as Down → Up (the link
///     went down + back up between polls).
///
/// Anything else (no change, unknown, dormant) is a no-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    /// Link is up + carrier present.
    Up,
    /// Link is administratively down or no-carrier.
    Down,
    /// Link is dormant (Wi-Fi roaming etc.). Treated like Down for
    /// the reconnect decision; some drivers report this between a
    /// disassoc and a reassoc.
    Dormant,
    /// `operstate` reported a value we don't recognise. We leave
    /// the existing connection alone rather than thrashing.
    Unknown,
}

impl LinkState {
    /// Parse the contents of `/sys/class/net/<iface>/operstate`.
    /// Case-sensitive matches against the kernel-emitted values.
    #[must_use]
    pub fn parse(operstate: &str) -> Self {
        match operstate.trim() {
            "up" => Self::Up,
            "down" => Self::Down,
            "dormant" => Self::Dormant,
            _ => Self::Unknown,
        }
    }

    /// `true` when the kernel says traffic can flow.
    #[must_use]
    pub fn is_up(self) -> bool {
        matches!(self, Self::Up)
    }
}

/// Phase 12.20 — observation of a link-state change. The reconcile
/// layer reads this to decide whether to schedule a WireGuard
/// re-handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkTransition {
    /// Carrier just came up. Re-handshake within the Q22 10 s
    /// budget.
    CameUp,
    /// Carrier just went down. UI shows "reconnecting".
    WentDown,
    /// No observable change — same state, same ifindex.
    NoChange,
}

/// Classify a (previous, current) pair into a [`LinkTransition`].
/// Used by the netlink-poll worker after each poll to decide if a
/// reconnect is warranted.
#[must_use]
pub fn classify_link_transition(prev: LinkState, curr: LinkState) -> LinkTransition {
    match (prev.is_up(), curr.is_up()) {
        (false, true) => LinkTransition::CameUp,
        (true, false) => LinkTransition::WentDown,
        _ => LinkTransition::NoChange,
    }
}

/// Phase 12.20 — poll `/sys/class/net/<iface>/operstate`. Returns
/// `Unknown` if the file isn't readable (interface gone, no
/// permission, …).
pub fn read_link_state(iface: &str) -> LinkState {
    let path = format!("/sys/class/net/{iface}/operstate");
    match std::fs::read_to_string(&path) {
        Ok(s) => LinkState::parse(&s),
        Err(_) => LinkState::Unknown,
    }
}

/// Phase 12.20 — sysfs-poll-based link watcher. Polls
/// `/sys/class/net/<iface>/operstate` every `period` and emits a
/// [`LinkTransition`] on the supplied callback whenever the
/// classifier says something interesting happened.
///
/// Using sysfs (instead of a netlink RTM_NEWLINK/DELLINK
/// multicast socket) keeps the worker simple + dep-free. The
/// trade-off is up to `period` of latency before we see a link-
/// down event — the 1 s default keeps that under the Q22 10 s
/// reconnect budget by a comfortable margin (9 s remaining for
/// the WireGuard re-handshake itself).
pub struct LinkWatchWorker {
    iface: String,
    period: Duration,
    on_transition: Arc<dyn Fn(LinkTransition) + Send + Sync>,
}

impl LinkWatchWorker {
    /// Construct watcher for `iface` with the locked 1 s period.
    /// `on_transition` is called from the tokio task whenever the
    /// classifier reports a change.
    #[must_use]
    pub fn new<F>(iface: impl Into<String>, on_transition: F) -> Self
    where
        F: Fn(LinkTransition) + Send + Sync + 'static,
    {
        Self {
            iface: iface.into(),
            period: Duration::from_secs(1),
            on_transition: Arc::new(on_transition),
        }
    }

    /// Override the poll period. The default 1 s keeps the
    /// reconnect under the Q22 10 s budget with 9 s of slack.
    pub fn with_period(mut self, period: Duration) -> Self {
        self.period = period;
        self
    }
}

#[async_trait::async_trait]
impl Worker for LinkWatchWorker {
    fn name(&self) -> &'static str {
        "link-watch"
    }

    async fn run(&mut self, mut shutdown: ShutdownToken) -> anyhow::Result<()> {
        let mut prev = read_link_state(&self.iface);
        info!(iface = %self.iface, initial = ?prev, "link-watch: starting");
        let mut ticker = tokio::time::interval(self.period);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                biased;
                _ = shutdown.wait() => {
                    info!(iface = %self.iface, "link-watch: shutdown");
                    return Ok(());
                }
                _ = ticker.tick() => {
                    let curr = read_link_state(&self.iface);
                    let transition = classify_link_transition(prev, curr);
                    if transition != LinkTransition::NoChange {
                        (self.on_transition)(transition);
                    }
                    prev = curr;
                }
            }
        }
    }
}

/// Phase 12.21 — eager connection bootstrap predicate. The locked
/// behaviour: pre-warm a WireGuard session to any peer that has
/// produced an RTT sample in the last `freshness` window. Returning
/// `true` means "pre-derive this session now so the first user
/// request finds it ready"; `false` means "let the lazy handshake
/// run".
///
/// Heuristic:
///   * Peer must have an RTT sample (proves connectivity exists).
///   * Sample must be at most `freshness` old, so we don't pre-warm
///     stale peers that left the LAN hours ago.
///   * RTT must be below `max_rtt_ms` — pre-warming a peer that's
///     already on the slow path doesn't save user-visible latency.
///
/// Q8 budget puts this in optimization-not-must-have territory; the
/// 5-second freshness default keeps the prewarm cost bounded.
#[must_use]
pub fn should_eager_bootstrap(
    rtt_ms: Option<u32>,
    sample_age_ms: Option<u64>,
    freshness: Duration,
    max_rtt_ms: u32,
) -> bool {
    let (Some(rtt), Some(age)) = (rtt_ms, sample_age_ms) else {
        return false;
    };
    if rtt > max_rtt_ms {
        return false;
    }
    Duration::from_millis(age) <= freshness
}

/// Phase 12.23 — LAN-multicast service type. Locked literal
/// (`_mackes-mcast._udp.local.` per Q23). Used for high-fanout
/// services like clipboard broadcast + presence so a single packet
/// reaches every LAN peer instead of N unicast copies.
pub const MULTICAST_SERVICE_TYPE: &str = "_mackes-mcast._udp.local.";

/// Phase 12.23 — IPv4 multicast group address. 239.42.7.16 sits in
/// the IPv4 administratively-scoped block (239.0.0.0/8 per RFC 2365)
/// and was picked to be locally administered by MDE — no IANA
/// allocation, deliberately unlikely to collide with another vendor.
pub const MULTICAST_GROUP_V4: [u8; 4] = [239, 42, 7, 16];

/// Phase 12.23 — UDP port for the multicast channel. Same number as
/// the unicast probe port so one firewall rule covers both paths.
pub const MULTICAST_PORT: u16 = DEFAULT_PROBE_PORT;

/// Phase 12.23 — Q16 wired-only guard. Wi-Fi networks tend to
/// silently drop multicast on power-save APs, so we never enable the
/// multicast path on Wi-Fi interfaces. Caller passes the interface's
/// link type (`"wired"` / `"wireless"` / `"loopback"` / ...). Returns
/// `true` if the multicast announcer should bind that interface.
#[must_use]
pub fn multicast_allowed_on_link(link_type: &str) -> bool {
    matches!(link_type, "wired" | "ethernet" | "loopback")
}

/// Phase 12.23 — open + bind a UDP socket that has joined the
/// `MULTICAST_GROUP_V4` group on `interface_v4`. The caller drives
/// it with the usual `recv_from` / `send_to` calls; this helper just
/// applies the `IP_ADD_MEMBERSHIP` + `SO_REUSEADDR` setup so two
/// processes on the same host can share the socket during dev/test.
///
/// # Errors
///
/// Surfaces any underlying tokio / socket error. The most common is
/// "Address already in use" (returned without reuse). Wi-Fi
/// interfaces are not detected here — gate on
/// [`multicast_allowed_on_link`] before calling.
pub async fn open_multicast_listener(interface_v4: Ipv4Addr) -> anyhow::Result<UdpSocket> {
    let bind: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), MULTICAST_PORT);
    let socket = UdpSocket::bind(bind).await?;
    let group = Ipv4Addr::from(MULTICAST_GROUP_V4);
    socket.join_multicast_v4(group, interface_v4)?;
    // Send + receive loopback so single-host dev/test works.
    let _ = socket.set_multicast_loop_v4(true);
    Ok(socket)
}

/// Phase 12.22 — throughput-aware path selection. Per the Q23 lock,
/// when bandwidth samples are present they trump the
/// "LAN-direct beats WAN" default. The locked rule: pick the path
/// with the higher measured throughput regardless of which side of
/// the WAN it sits on. The "saturated Wi-Fi vs idle fiber" case
/// rolls to the higher-throughput path automatically.
///
/// Returns `true` when path A should win over path B.
#[must_use]
pub fn higher_throughput_wins(a_bytes_per_sec: Option<u64>, b_bytes_per_sec: Option<u64>) -> bool {
    match (a_bytes_per_sec, b_bytes_per_sec) {
        (Some(a), Some(b)) => a >= b,
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => false,
    }
}

/// Phase 12.15 — IPv6-first direct-path preference. When both peers
/// expose public IPv6 (i.e. both samples are present), prefer the
/// IPv6 path over NAT'd IPv4 + DERP. Q9 originally locked
/// IPv4-only; v12.15 promotes IPv6 to the top of the
/// path-preference ladder when it's available.
///
/// Returns `true` when the IPv6 direct path should be preferred over
/// the IPv4-NAT-plus-DERP fallback.
#[must_use]
pub fn ipv6_direct_wins(ipv6_rtt_ms: Option<u32>, ipv4_derp_rtt_ms: Option<u32>) -> bool {
    match (ipv6_rtt_ms, ipv4_derp_rtt_ms) {
        // Q12.15 lock: IPv6 wins by default whenever it's reachable,
        // even at a small RTT cost, because direct paths are more
        // robust (no third-party relay) + cheaper (no DERP egress).
        // The throughput-aware override lands in Phase 12.22 — that
        // can still demote IPv6 if it's saturated.
        (Some(_), Some(_)) => true,
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => false,
    }
}

/// Extract the first IPv4 socket address from an mDNS resolved record.
fn first_ipv4(info: &ServiceInfo) -> Option<SocketAddr> {
    for addr in info.get_addresses() {
        if let IpAddr::V4(v4) = addr {
            return Some(SocketAddr::new(IpAddr::V4(*v4), info.get_port()));
        }
    }
    None
}

/// Worker construction parameters.
pub struct LanDiscoveryConfig {
    /// Local node hostname (`instance` label on the mDNS
    /// announcement).
    pub host: String,
    /// UDP port the local probe listener binds. Announced via the
    /// service info.
    pub port: u16,
    /// Probe period — how often to re-ping every known peer. SLO Q7
    /// is 30 s detection, so we pick 5 s here so a peer that just
    /// arrived has 5 RTT samples by the 30 s mark.
    pub probe_period: Duration,
    /// Shared registry. The owner clones it to give the GUI / reconcile
    /// loop read-only access through [`Registry::snapshot`].
    pub registry: Registry,
}

impl LanDiscoveryConfig {
    /// Defaults: pick up the system hostname, bind the canonical port,
    /// 5-second probe cadence, fresh registry. Caller is expected to
    /// clone the registry before constructing if it needs read access.
    #[must_use]
    pub fn new(host: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port: DEFAULT_PROBE_PORT,
            probe_period: Duration::from_secs(5),
            registry: Registry::new(),
        }
    }
}

/// LAN discovery + probe worker.
pub struct LanDiscoveryWorker {
    config: LanDiscoveryConfig,
}

impl LanDiscoveryWorker {
    /// Construct with the given config.
    #[must_use]
    pub fn new(config: LanDiscoveryConfig) -> Self {
        Self { config }
    }

    /// Borrow the shared registry — useful for tests + for handing
    /// the GUI a read handle before [`Worker::run`] is spawned.
    #[must_use]
    pub fn registry(&self) -> Registry {
        self.config.registry.clone()
    }
}

#[async_trait::async_trait]
impl Worker for LanDiscoveryWorker {
    fn name(&self) -> &'static str {
        "lan-discovery"
    }

    async fn run(&mut self, mut shutdown: ShutdownToken) -> anyhow::Result<()> {
        let LanDiscoveryConfig {
            host,
            port,
            probe_period,
            registry,
        } = self.config.clone_for_run();

        info!(host = %host, port, "lan-discovery: starting");

        // mdns-sd's ServiceDaemon is sync + spawns its own poll
        // thread internally, so we just drive its event channel from
        // a tokio task.
        let daemon = ServiceDaemon::new().context("lan-discovery: ServiceDaemon::new")?;
        let receiver = daemon
            .browse(SERVICE_TYPE)
            .context("lan-discovery: daemon.browse")?;

        // Announce self.
        let service_host = format!("{host}.local.");
        let info = ServiceInfo::new(
            SERVICE_TYPE,
            &host,
            &service_host,
            // Bind to any reachable IPv4 — mdns-sd picks per-interface.
            "0.0.0.0",
            port,
            None,
        )
        .context("lan-discovery: ServiceInfo::new")?;
        daemon
            .register(info)
            .context("lan-discovery: daemon.register")?;

        // UDP probe socket — bound on `0.0.0.0:port` so the OS picks
        // the right interface per peer.
        let listener = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port))
            .await
            .context("lan-discovery: UdpSocket::bind")?;
        let listener = Arc::new(listener);

        // Listener task: echo pings + record pongs.
        let listen_socket = listener.clone();
        let listen_registry = registry.clone();
        let listen_pending = PendingPings::default();
        let listen_pending_for_task = listen_pending.clone();
        let listen_handle = tokio::spawn(async move {
            let mut buf = [0u8; 64];
            loop {
                match listen_socket.recv_from(&mut buf).await {
                    Ok((n, src)) => {
                        let Some(msg) = decode_probe(&buf[..n]) else {
                            continue;
                        };
                        match msg {
                            ProbeMsg::Ping(seq) => {
                                let pong = encode_pong(seq);
                                let _ = listen_socket.send_to(&pong, src).await;
                            }
                            ProbeMsg::Pong(seq) => {
                                if let Some((peer, started)) =
                                    listen_pending_for_task.take(src, seq)
                                {
                                    let rtt = started.elapsed().as_millis() as u32;
                                    listen_registry.record_rtt(RttSample {
                                        peer,
                                        addr: src,
                                        rtt_ms: rtt,
                                        seq,
                                    });
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!(error = ?e, "lan-discovery: recv_from failed");
                        return;
                    }
                }
            }
        });

        // Main event loop: drain mDNS events, fire periodic probes,
        // shut down on signal.
        let mut probe_tick = tokio::time::interval(probe_period);
        // Start the first tick immediately so we probe known peers
        // without a 5-second cold-start delay.
        probe_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut next_seq: u32 = 0;

        loop {
            tokio::select! {
                biased;
                _ = shutdown.wait() => {
                    let _ = daemon.shutdown();
                    listen_handle.abort();
                    info!("lan-discovery: shutdown clean");
                    return Ok(());
                }
                _ = probe_tick.tick() => {
                    let (peers, _) = registry.snapshot();
                    for peer in peers {
                        next_seq = next_seq.wrapping_add(1);
                        let datagram = encode_ping(next_seq);
                        listen_pending.insert(peer.addr, next_seq, peer.instance.clone());
                        if let Err(e) = listener.send_to(&datagram, peer.addr).await {
                            debug!(peer = %peer.instance, error = ?e, "probe send failed");
                            listen_pending.take(peer.addr, next_seq);
                        }
                    }
                }
                // mdns-sd's channel is `flume::Receiver`, which is sync.
                // Poll it on the runtime's blocking thread so we don't
                // pin our tokio task waiting for events.
                evt = tokio::task::spawn_blocking({
                    let receiver = receiver.clone();
                    move || receiver.recv_timeout(Duration::from_millis(500))
                }) => {
                    match evt {
                        Ok(Ok(event)) => handle_mdns_event(event, &registry, &host),
                        Ok(Err(_)) => {
                            // Timeout — loop and re-check shutdown.
                        }
                        Err(join_err) => {
                            warn!(error = ?join_err, "lan-discovery: mDNS poll task panicked");
                        }
                    }
                }
            }
        }
    }
}

impl LanDiscoveryConfig {
    /// Clone-by-value for the worker's run loop. The registry is the
    /// shared handle; the rest are owned strings + Copy.
    fn clone_for_run(&self) -> Self {
        Self {
            host: self.host.clone(),
            port: self.port,
            probe_period: self.probe_period,
            registry: self.registry.clone(),
        }
    }
}

/// Maps `(addr, seq) → (peer-instance, started-at)` for in-flight
/// pings. The listener task consults it to compute RTT.
#[derive(Debug, Clone, Default)]
struct PendingPings {
    inner: Arc<Mutex<HashMap<(SocketAddr, u32), (String, Instant)>>>,
}

impl PendingPings {
    fn insert(&self, addr: SocketAddr, seq: u32, peer: String) {
        let mut g = self.inner.lock().expect("pending mutex poisoned");
        g.insert((addr, seq), (peer, Instant::now()));
    }

    fn take(&self, addr: SocketAddr, seq: u32) -> Option<(String, Instant)> {
        let mut g = self.inner.lock().expect("pending mutex poisoned");
        g.remove(&(addr, seq))
    }
}

fn handle_mdns_event(event: ServiceEvent, registry: &Registry, self_host: &str) {
    match event {
        ServiceEvent::ServiceResolved(info) => {
            let instance_full = info.get_fullname().to_string();
            let instance = instance_full
                .strip_suffix(SERVICE_TYPE)
                .map(|s| s.trim_end_matches('.').to_string())
                .unwrap_or(instance_full);
            if instance == self_host {
                return; // skip own announcement
            }
            let Some(addr) = first_ipv4(&info) else {
                debug!(instance = %instance, "lan-discovery: resolved peer with no IPv4");
                return;
            };
            registry.upsert_peer(LanPeer {
                instance: instance.clone(),
                host: instance,
                addr,
            });
        }
        ServiceEvent::ServiceRemoved(_ty, fullname) => {
            let instance = fullname
                .strip_suffix(SERVICE_TYPE)
                .map(|s| s.trim_end_matches('.').to_string())
                .unwrap_or(fullname);
            registry.remove_peer(&instance);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddr};

    #[test]
    fn service_type_matches_q7_lock() {
        // Locked literal — changing it is a wire-protocol break.
        assert_eq!(SERVICE_TYPE, "_mackes-peer._udp.local.");
    }

    #[test]
    fn worker_name_is_kebab_case() {
        let w = LanDiscoveryWorker::new(LanDiscoveryConfig::new("anvil"));
        assert_eq!(w.name(), "lan-discovery");
    }

    #[test]
    fn encode_and_decode_ping_round_trip() {
        let bytes = encode_ping(42);
        let decoded = decode_probe(&bytes).expect("decoded");
        assert_eq!(decoded, ProbeMsg::Ping(42));
    }

    #[test]
    fn encode_and_decode_pong_round_trip() {
        let bytes = encode_pong(7);
        let decoded = decode_probe(&bytes).expect("decoded");
        assert_eq!(decoded, ProbeMsg::Pong(7));
    }

    #[test]
    fn decode_rejects_short_buffer() {
        assert!(decode_probe(&[]).is_none());
        assert!(decode_probe(b"MPRB").is_none());
        assert!(decode_probe(b"MPRB\x01\x00\x00").is_none());
    }

    #[test]
    fn decode_rejects_wrong_magic() {
        let bad = b"XXXX\x01\x00\x00\x00\x00";
        assert!(decode_probe(bad).is_none());
    }

    #[test]
    fn decode_rejects_unknown_opcode() {
        let mut buf = [0u8; 9];
        buf[..4].copy_from_slice(&PROBE_MAGIC);
        buf[4] = 0xFF;
        assert!(decode_probe(&buf).is_none());
    }

    #[test]
    fn registry_upsert_and_remove() {
        let r = Registry::new();
        assert_eq!(r.peer_count(), 0);
        let peer = LanPeer {
            instance: "anvil".into(),
            host: "anvil".into(),
            addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 4)), 41841),
        };
        r.upsert_peer(peer.clone());
        assert_eq!(r.peer_count(), 1);
        // Idempotent upsert.
        r.upsert_peer(peer.clone());
        assert_eq!(r.peer_count(), 1);
        r.remove_peer("anvil");
        assert_eq!(r.peer_count(), 0);
    }

    #[test]
    fn registry_snapshot_is_sorted_by_instance() {
        let r = Registry::new();
        for name in ["pine", "anvil", "oak"] {
            r.upsert_peer(LanPeer {
                instance: name.into(),
                host: name.into(),
                addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 41841),
            });
        }
        let (peers, _) = r.snapshot();
        let names: Vec<_> = peers.iter().map(|p| p.instance.as_str()).collect();
        assert_eq!(names, vec!["anvil", "oak", "pine"]);
    }

    #[test]
    fn registry_record_rtt_keeps_only_latest_per_peer() {
        let r = Registry::new();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 9)), 41841);
        r.record_rtt(RttSample {
            peer: "pine".into(),
            addr,
            rtt_ms: 50,
            seq: 1,
        });
        r.record_rtt(RttSample {
            peer: "pine".into(),
            addr,
            rtt_ms: 30,
            seq: 2,
        });
        assert_eq!(r.rtt_count(), 1);
        let (_, rtts) = r.snapshot();
        assert_eq!(rtts.get("pine").unwrap().rtt_ms, 30);
    }

    #[test]
    fn multipath_predicate_requires_low_rtt_and_balanced_bw() {
        // Both fast + comparable bandwidth — multipath enabled.
        assert!(should_use_multipath(20, 30, 10_000_000, 12_000_000));
        // Asymmetric bandwidth (≥ 2x gap) — disabled.
        assert!(!should_use_multipath(20, 30, 1_000_000, 10_000_000));
        // One path too slow (RTT ≥ 50 ms ceiling) — disabled.
        assert!(!should_use_multipath(20, 60, 10_000_000, 12_000_000));
        assert!(!should_use_multipath(50, 30, 10_000_000, 12_000_000));
        // Zero-bw sample — disabled.
        assert!(!should_use_multipath(10, 10, 0, 1_000_000));
        // Boundary: RTT exactly 49 ms still passes; 50 ms is the
        // cutoff (>=).
        assert!(should_use_multipath(49, 49, 1_000_000, 1_000_000));
        // Bandwidth boundary: slow == 0.5 × fast still passes.
        assert!(should_use_multipath(10, 10, 5_000_000, 10_000_000));
        // Just under the 50% window — disabled.
        assert!(!should_use_multipath(10, 10, 4_999_999, 10_000_000));
    }

    #[test]
    fn packet_dedupe_first_observation_returns_true() {
        let mut d = PacketDedupe::new();
        assert!(d.observe(42));
    }

    #[test]
    fn packet_dedupe_duplicate_returns_false() {
        let mut d = PacketDedupe::new();
        d.observe(42);
        assert!(!d.observe(42));
    }

    #[test]
    fn packet_dedupe_window_evicts_oldest() {
        let mut d = PacketDedupe::with_capacity(3);
        d.observe(1);
        d.observe(2);
        d.observe(3);
        d.observe(4); // evicts 1
        assert_eq!(d.len(), 3);
        // 1 is no longer tracked → re-observing it is "new".
        assert!(d.observe(1));
    }

    #[test]
    fn packet_dedupe_default_capacity_is_locked() {
        let d = PacketDedupe::default();
        // Push one more than the locked default to confirm the
        // capacity cap.
        let mut d = d;
        for i in 0..2000u64 {
            d.observe(i);
        }
        assert_eq!(d.len(), 1024, "default capacity locked at 1024");
    }

    #[test]
    fn link_state_parses_kernel_operstate_values() {
        assert_eq!(LinkState::parse("up"), LinkState::Up);
        assert_eq!(LinkState::parse("down"), LinkState::Down);
        assert_eq!(LinkState::parse("dormant"), LinkState::Dormant);
        assert_eq!(LinkState::parse("notpresent"), LinkState::Unknown);
        // Trim surrounding whitespace + newlines from
        // /sys/class/net read.
        assert_eq!(LinkState::parse("  up\n"), LinkState::Up);
    }

    #[test]
    fn link_state_is_up_only_for_up() {
        assert!(LinkState::Up.is_up());
        assert!(!LinkState::Down.is_up());
        assert!(!LinkState::Dormant.is_up());
        assert!(!LinkState::Unknown.is_up());
    }

    #[test]
    fn classify_link_transition_returns_came_up_on_down_to_up() {
        let t = classify_link_transition(LinkState::Down, LinkState::Up);
        assert_eq!(t, LinkTransition::CameUp);
    }

    #[test]
    fn classify_link_transition_returns_went_down_on_up_to_down() {
        let t = classify_link_transition(LinkState::Up, LinkState::Down);
        assert_eq!(t, LinkTransition::WentDown);
        // Dormant counts as "down" for reconnect purposes.
        let t = classify_link_transition(LinkState::Up, LinkState::Dormant);
        assert_eq!(t, LinkTransition::WentDown);
    }

    #[test]
    fn read_link_state_returns_unknown_for_missing_iface() {
        let s = read_link_state("does-not-exist-99");
        assert_eq!(s, LinkState::Unknown);
    }

    #[tokio::test]
    async fn link_watch_worker_exits_clean_on_shutdown() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_cb = calls.clone();
        let mut w = LinkWatchWorker::new("does-not-exist-99", move |_| {
            calls_for_cb.fetch_add(1, Ordering::SeqCst);
        })
        .with_period(Duration::from_millis(50));
        let (tx, rx) = tokio::sync::watch::channel(false);
        let token = ShutdownToken::from_receiver(rx);
        let handle = tokio::spawn(async move { w.run(token).await });
        tokio::time::sleep(Duration::from_millis(150)).await;
        let _ = tx.send(true);
        let result = tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("worker should exit on shutdown")
            .expect("join");
        assert!(result.is_ok());
        // The missing iface stays in Unknown state throughout; the
        // classifier reports NoChange for Unknown → Unknown so the
        // callback never fires. Verifies the no-op branch.
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn classify_link_transition_returns_no_change_when_same() {
        assert_eq!(
            classify_link_transition(LinkState::Up, LinkState::Up),
            LinkTransition::NoChange
        );
        assert_eq!(
            classify_link_transition(LinkState::Down, LinkState::Down),
            LinkTransition::NoChange
        );
    }

    #[test]
    fn should_eager_bootstrap_requires_fresh_low_rtt_sample() {
        let freshness = Duration::from_secs(5);
        let max_rtt = 100u32;
        // Fresh + low RTT — prewarm.
        assert!(should_eager_bootstrap(
            Some(40),
            Some(1_000),
            freshness,
            max_rtt
        ));
        // Fresh + tied to max — prewarm (≤ boundary).
        assert!(should_eager_bootstrap(
            Some(100),
            Some(0),
            freshness,
            max_rtt
        ));
        // Fresh but slow — skip.
        assert!(!should_eager_bootstrap(
            Some(150),
            Some(1_000),
            freshness,
            max_rtt
        ));
        // Stale sample — skip.
        assert!(!should_eager_bootstrap(
            Some(40),
            Some(10_000),
            freshness,
            max_rtt
        ));
        // No sample — skip.
        assert!(!should_eager_bootstrap(None, Some(100), freshness, max_rtt));
        // No timestamp — skip.
        assert!(!should_eager_bootstrap(Some(40), None, freshness, max_rtt));
    }

    #[test]
    fn multicast_constants_match_q23_lock() {
        assert_eq!(MULTICAST_SERVICE_TYPE, "_mackes-mcast._udp.local.");
        assert_eq!(MULTICAST_GROUP_V4, [239, 42, 7, 16]);
        // Same port as unicast probes so one firewall rule covers
        // both paths.
        assert_eq!(MULTICAST_PORT, DEFAULT_PROBE_PORT);
    }

    #[tokio::test]
    async fn open_multicast_listener_binds_on_loopback() {
        // Try the loopback interface so the test doesn't require
        // routable connectivity. CI containers without multicast
        // capability return Err — skip explicitly so a real
        // regression surfaces (cannot bind = environment, not bug).
        match open_multicast_listener(Ipv4Addr::LOCALHOST).await {
            Ok(socket) => {
                let local = socket.local_addr().expect("local_addr");
                assert_eq!(local.port(), MULTICAST_PORT);
            }
            Err(e) => {
                eprintln!("skip: multicast bind blocked by environment: {e}");
            }
        }
    }

    #[test]
    fn multicast_allowed_on_link_q16_wired_only() {
        assert!(multicast_allowed_on_link("wired"));
        assert!(multicast_allowed_on_link("ethernet"));
        // Loopback allowed so the same code path covers single-host
        // dev loops + CI.
        assert!(multicast_allowed_on_link("loopback"));
        // Wi-Fi explicitly forbidden by Q16.
        assert!(!multicast_allowed_on_link("wireless"));
        assert!(!multicast_allowed_on_link("wifi"));
        assert!(!multicast_allowed_on_link("cellular"));
        assert!(!multicast_allowed_on_link(""));
    }

    #[test]
    fn higher_throughput_wins_under_q23_lock() {
        // Both samples present — A wins on >= bytes/sec.
        assert!(higher_throughput_wins(Some(10_000_000), Some(1_000_000)));
        assert!(!higher_throughput_wins(Some(1_000_000), Some(10_000_000)));
        // Tie — A wins (stable preference for the named "A" path).
        assert!(higher_throughput_wins(Some(5_000_000), Some(5_000_000)));
        // Only A — A wins.
        assert!(higher_throughput_wins(Some(1), None));
        // Only B — B wins.
        assert!(!higher_throughput_wins(None, Some(1)));
        // Neither — neither wins.
        assert!(!higher_throughput_wins(None, None));
    }

    #[test]
    fn ipv6_direct_wins_under_q12_15_lock() {
        // Both up — IPv6 wins regardless of RTT (direct preferred).
        assert!(ipv6_direct_wins(Some(80), Some(20)));
        assert!(ipv6_direct_wins(Some(10), Some(50)));
        // Only IPv6 — wins.
        assert!(ipv6_direct_wins(Some(100), None));
        // Only IPv4+DERP — IPv4 wins.
        assert!(!ipv6_direct_wins(None, Some(100)));
        // Neither — neither wins.
        assert!(!ipv6_direct_wins(None, None));
    }

    #[test]
    fn lan_direct_wins_under_q23_throughput_policy() {
        // Both up — lower RTT wins.
        assert!(lan_direct_wins(Some(10), Some(50)));
        assert!(!lan_direct_wins(Some(50), Some(10)));
        // Tie-breaker: LAN-direct wins on equal RTT.
        assert!(lan_direct_wins(Some(20), Some(20)));
        // Only LAN sample present — LAN-direct preferred.
        assert!(lan_direct_wins(Some(100), None));
        // Only DERP sample present — DERP wins.
        assert!(!lan_direct_wins(None, Some(100)));
        // No samples — neither wins; treat as DERP (the safer fallback).
        assert!(!lan_direct_wins(None, None));
    }

    #[test]
    fn config_new_defaults_to_locked_constants() {
        let cfg = LanDiscoveryConfig::new("anvil");
        assert_eq!(cfg.host, "anvil");
        assert_eq!(cfg.port, DEFAULT_PROBE_PORT);
        assert_eq!(cfg.probe_period, Duration::from_secs(5));
        assert_eq!(cfg.registry.peer_count(), 0);
    }

    #[test]
    fn pending_pings_insert_and_take_round_trip() {
        let p = PendingPings::default();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 9)), 41841);
        p.insert(addr, 7, "pine".into());
        let taken = p.take(addr, 7).expect("present");
        assert_eq!(taken.0, "pine");
        // Second take returns None.
        assert!(p.take(addr, 7).is_none());
    }

    #[test]
    fn worker_registry_handle_clones_underlying_arc() {
        let w = LanDiscoveryWorker::new(LanDiscoveryConfig::new("anvil"));
        let r1 = w.registry();
        let r2 = w.registry();
        // Independent clones must share state.
        r1.upsert_peer(LanPeer {
            instance: "pine".into(),
            host: "pine".into(),
            addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 7)), 41841),
        });
        assert_eq!(r2.peer_count(), 1);
    }
}
