//! KDC2-1 — `Transport` trait + capability + health model.
//!
//! v2.1 KDC2 lock (2026-05-22): the mesh router (`mackesd::workers::
//! mesh_router`) picks the per-message path between peers. To do that
//! generically, it needs a single trait every transport implementation
//! consumes — direct UDP, DERP relay, HTTPS-443 tunnel, KDC-TLS. This
//! crate is the seam.
//!
//! Why a stand-alone crate (instead of a module inside `mackesd`)?
//!
//! - `mde-kdc` (host integration, KDC2-3) implements `Transport` and
//!   shouldn't drag in `mackesd`'s SQLite + zbus + worker pool just to
//!   compile.
//! - Future transport impls (`mackes-https-tunnel`, BLE/LoRa/Matrix
//!   per the v2.1 KDC2 lock's deferred items) land as new workspace
//!   members that depend only on this crate.
//! - The mesh router can live in `mackesd` while still being unit-
//!   testable against a `MockTransport` in this crate's own tests.
//!
//! The trait is **object-safe** (`async-trait` desugaring) so the
//! router can hold a `Box<dyn Transport>` per registered transport
//! and dispatch dynamically.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod conformance;
pub mod health;
pub mod peer_path;
pub mod scorer;
pub mod transport_capabilities;

pub use health::{HealthSnapshot, ProbeOutcome, RouterError};
pub use peer_path::{PeerPath, SwitchReason};
pub use transport_capabilities::{EncryptionKind, TransportCapabilities};

/// Identifier for a specific transport implementation.
///
/// Mirrors `mackesd::topology::EdgeKind` 1:1 so the topology engine
/// and the router agree on which transport carries which edge. The
/// `From<TransportKind> for EdgeKind` conversion lives in
/// `mackesd::topology` to keep this crate dependency-free.
///
/// v2.5 (NF-4.1, 2026-05-24) — renamed from the Tailscale-era
/// `{DirectUdp, DerpRelay, Https443}` to Nebula-rooted
/// `{NebulaDirect, NebulaLighthouseRelay, NebulaHttps443}`.
/// `KdcTls` is unchanged. The `as_str` tokens follow the same
/// rename (`direct_udp` → `nebula_direct`, etc.) — audit-log
/// readers + policy.toml files written against the v2.x tokens
/// are migrated by `mackesd_core::transport::policy::migrate_tokens`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    /// Best-case: direct UDP between two peers' Nebula tun
    /// sockets after the lighthouse-assisted hole-punch
    /// (matches `EdgeKind::NebulaDirect`).
    NebulaDirect,
    /// Lighthouse-relayed path — the host-role peer carries
    /// traffic for two peers that can't hole-punch each other
    /// (matches `EdgeKind::NebulaLighthouseRelay`). Subsumes
    /// the prior Tailscale DERP fallback.
    NebulaLighthouseRelay,
    /// Covert TCP/443 path — Nebula UDP wrapped in TLS through
    /// `mackes-nebula-https-tunnel` (matches
    /// `EdgeKind::NebulaHttps443`).
    NebulaHttps443,
    /// KDE Connect wire over TLS (matches `EdgeKind::KdcTls`, lands
    /// with the KDC2 work). Used for phone↔peer and peer↔peer-via-
    /// KDC links once mesh-shunt (KDC2-4) wires phones up to the
    /// mesh router as first-class participants.
    KdcTls,
}

impl TransportKind {
    /// Iteration order is the **preference order** the router uses
    /// as a tiebreaker when two transports report identical health
    /// + capabilities. Lower-latency transports come first.
    ///
    /// Locked by v12 throughput-first routing + the v2.5 Nebula
    /// rebuild: `NebulaDirect > KdcTls > NebulaLighthouseRelay >
    /// NebulaHttps443`. KdcTls outranks the lighthouse relay
    /// because the KDC handshake reuses a long-lived TLS session
    /// (~0 RTT for steady-state messages), where the lighthouse
    /// relay adds the host-peer's RTT on every packet.
    #[must_use]
    pub const fn all() -> [TransportKind; 4] {
        [
            TransportKind::NebulaDirect,
            TransportKind::KdcTls,
            TransportKind::NebulaLighthouseRelay,
            TransportKind::NebulaHttps443,
        ]
    }

    /// Stable string identifier used in metric labels + audit log
    /// entries. Matches the `serde` snake_case rendering so a
    /// machine reading audit JSON sees the same token in both
    /// places.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            TransportKind::NebulaDirect => "nebula_direct",
            TransportKind::NebulaLighthouseRelay => "nebula_lighthouse_relay",
            TransportKind::NebulaHttps443 => "nebula_https443",
            TransportKind::KdcTls => "kdc_tls",
        }
    }
}

impl fmt::Display for TransportKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Coarse health of a transport at a moment in time. The router
/// queries `health()` before every send so transient blips skip
/// to the next-best option without retry-storming the failed one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    /// Last probe within `Capabilities.health_window` succeeded with
    /// no degradation. Router sends freely.
    Healthy,
    /// Last probe succeeded but latency / packet loss exceeded the
    /// transport's degradation threshold. Router still sends but
    /// prefers a `Healthy` peer transport if one exists.
    Degraded,
    /// Last probe failed (timeout, refused, unreachable). Router
    /// skips this transport until the next health window elapses.
    Down,
}

impl HealthState {
    /// True when the router may send through this transport.
    /// `Healthy` always; `Degraded` yes-but-prefers-an-alternative;
    /// `Down` never.
    #[must_use]
    pub const fn is_sendable(self) -> bool {
        matches!(self, HealthState::Healthy | HealthState::Degraded)
    }
}

/// Per-transport capability advertisement. Each transport reports
/// what it can carry so the router can match message classes to
/// transports without hard-coding the table.
///
/// All durations are window-or-budget values, not "current measured"
/// values. The `MeshRouter` keeps a separate observation history.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Capabilities {
    /// Maximum frame this transport will accept in a single send.
    /// `None` means "unbounded" (the implementation streams).
    pub max_frame_bytes: Option<u64>,
    /// Health is re-probed on this cadence. Lower window =
    /// snappier failover but more chatty. Direct UDP defaults to
    /// 5 s; HTTPS-443 to 30 s (cert handshake is expensive).
    pub health_window: Duration,
    /// Whether this transport carries the four canonical message
    /// classes the router dispatches.
    pub carries: MessageClassSet,
    /// Operator-readable name for log lines + audit entries.
    /// Independent of `TransportKind::as_str` so two
    /// implementations of the same kind (e.g. `DirectUdp` over
    /// wireguard vs. over plain socket) can differentiate.
    pub label: String,
}

/// The four message classes the v2.1 KDC2 lock recognizes. Each
/// transport reports whether it carries each class — the router
/// matches per-message based on this set.
///
/// Locks per memory `project_v2_1_kdc2_native.md`:
///   * `Control` — KDC always (paired-device commands, ring, find).
///   * `Clipboard` — best-path (latency-bound, small frames).
///   * `FileBulk` — throughput-best (large frames, DERP/HTTPS only
///     when DirectUdp is unhealthy).
///   * `Notification` — dual-send, idempotent at receiver. Router
///     sends through every healthy transport; receiver dedupes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageClassSet {
    /// Control messages (e.g. KDC commands).
    pub control: bool,
    /// Clipboard sync messages.
    pub clipboard: bool,
    /// Bulk file transfer.
    pub file_bulk: bool,
    /// Notification mirroring.
    pub notification: bool,
}

impl MessageClassSet {
    /// Transport that carries every message class. Useful constant
    /// for full-fat transports like direct UDP and KDC-TLS.
    #[must_use]
    pub const fn all() -> Self {
        Self {
            control: true,
            clipboard: true,
            file_bulk: true,
            notification: true,
        }
    }

    /// Transport that carries only control + clipboard (small-frame
    /// reach via a relay). Used by DERP defaults.
    #[must_use]
    pub const fn small_only() -> Self {
        Self {
            control: true,
            clipboard: true,
            file_bulk: false,
            notification: true,
        }
    }

    /// True if this transport can carry the given message class.
    #[must_use]
    pub const fn carries(&self, class: MessageClass) -> bool {
        match class {
            MessageClass::Control => self.control,
            MessageClass::Clipboard => self.clipboard,
            MessageClass::FileBulk => self.file_bulk,
            MessageClass::Notification => self.notification,
        }
    }
}

/// Single-message-class selector. The router asks
/// `capabilities.carries.carries(class)` per send.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageClass {
    /// Per the v2.1 KDC2 lock — control messages.
    Control,
    /// Clipboard sync.
    Clipboard,
    /// Bulk file transfer.
    FileBulk,
    /// Notification mirroring (dual-send idempotent).
    Notification,
}

/// Connection handle returned by `Transport::open()`. The trait is
/// intentionally minimal — implementations hold the actual socket /
/// TLS session / queue inside their own `Connection` type and erase
/// it behind this boxed handle.
///
/// The router never calls back into the connection — sends are
/// driven by `Transport::send_through` per-message. The handle
/// exists so the router can hold a `Connection` alive across sends
/// without re-opening the transport every time.
pub trait Connection: Send + Sync + std::fmt::Debug {
    /// Stable identifier for this connection — used in audit log
    /// entries so the operator can correlate router decisions
    /// with the actual long-lived connection that carried them.
    fn id(&self) -> &str;
}

/// The core router-facing trait. Object-safe via `async-trait`.
///
/// Implementations live in their own crates (`mde-kdc` for KDC2,
/// `mackes-https-tunnel` for 12.18, `mackesd::transport::direct_udp`
/// for the WireGuard path).
#[async_trait]
pub trait Transport: Send + Sync + std::fmt::Debug {
    /// What flavor of transport is this. The router uses this to
    /// surface the carrier in audit entries + the operator's
    /// topology diff view.
    fn kind(&self) -> TransportKind;

    /// Advertise what this transport carries. Pure — the same
    /// instance must return the same `Capabilities` for its
    /// lifetime. Use the `MeshRouter`'s observation history (not
    /// this) for live latency / loss data.
    fn capabilities(&self) -> Capabilities;

    /// Quick reachability probe to `peer_id`. Implementations
    /// should keep this **cheap** — sub-second, no large
    /// allocations — because the router calls it on the hot path.
    async fn probe(&self, peer_id: &str) -> HealthState;

    /// Open (or return a cached) connection to `peer_id`. The
    /// connection is held by the router for the lifetime of the
    /// peer session — implementations free to multiplex sends
    /// through a single connection.
    async fn open(&self, peer_id: &str) -> Result<Box<dyn Connection>, TransportError>;

    /// Snapshot of current health for the given peer. The router
    /// queries this *before* every send (after `open()` returns
    /// a handle) so a transport that's open-but-degraded gets
    /// downgraded mid-session.
    async fn health(&self, peer_id: &str) -> HealthState;
}

/// Errors a `Transport` impl may surface to the router. Each error
/// is paired with a stable `code` string so audit-log entries
/// stay machine-greppable.
#[derive(Debug)]
pub enum TransportError {
    /// Peer is not reachable via this transport right now (NAT
    /// blocked direct UDP, DERP region down, etc.).
    Unreachable {
        /// Stable code for audit entries (e.g. `nat_blocked`).
        code: &'static str,
    },
    /// Peer is reachable but the handshake / TLS / KDC negotiation
    /// failed (cert mismatch, bad credentials).
    HandshakeFailed {
        /// Stable code for audit entries (e.g. `cert_mismatch`).
        code: &'static str,
    },
    /// Transport is misconfigured (missing key file, no allowed
    /// peers, etc.). Distinct from runtime failures because the
    /// router shouldn't retry until the config changes.
    Misconfigured {
        /// Stable code for audit entries (e.g. `missing_key`).
        code: &'static str,
    },
    /// Transient I/O failure — router may retry next health window.
    Io {
        /// Stable code for audit entries (e.g. `timeout`).
        code: &'static str,
    },
}

impl TransportError {
    /// Stable machine-greppable code for the error.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            TransportError::Unreachable { code }
            | TransportError::HandshakeFailed { code }
            | TransportError::Misconfigured { code }
            | TransportError::Io { code } => code,
        }
    }

    /// Stable family identifier for the error variant — used by
    /// the router to bucket retry policy (Misconfigured → never
    /// retry without config change; Io → retry next window; etc.).
    #[must_use]
    pub const fn family(&self) -> &'static str {
        match self {
            TransportError::Unreachable { .. } => "unreachable",
            TransportError::HandshakeFailed { .. } => "handshake_failed",
            TransportError::Misconfigured { .. } => "misconfigured",
            TransportError::Io { .. } => "io",
        }
    }
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.family(), self.code())
    }
}

impl std::error::Error for TransportError {}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_kind_preference_order_locks_v2_5_routing() {
        let order = TransportKind::all();
        assert_eq!(order[0], TransportKind::NebulaDirect);
        assert_eq!(order[1], TransportKind::KdcTls);
        assert_eq!(order[2], TransportKind::NebulaLighthouseRelay);
        assert_eq!(order[3], TransportKind::NebulaHttps443);
    }

    #[test]
    fn transport_kind_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&TransportKind::NebulaDirect).unwrap(),
            r#""nebula_direct""#,
        );
        assert_eq!(
            serde_json::to_string(&TransportKind::KdcTls).unwrap(),
            r#""kdc_tls""#,
        );
        assert_eq!(
            serde_json::to_string(&TransportKind::NebulaLighthouseRelay).unwrap(),
            r#""nebula_lighthouse_relay""#,
        );
        assert_eq!(
            serde_json::to_string(&TransportKind::NebulaHttps443).unwrap(),
            r#""nebula_https443""#,
        );
    }

    #[test]
    fn transport_kind_display_matches_serde_token() {
        // Audit log and metric label must use the same token.
        for k in TransportKind::all() {
            let display = format!("{k}");
            let serde_token = serde_json::to_string(&k)
                .unwrap()
                .trim_matches('"')
                .to_string();
            assert_eq!(display, serde_token, "Display drifted from serde for {k:?}");
        }
    }

    #[test]
    fn health_state_sendable_only_when_not_down() {
        assert!(HealthState::Healthy.is_sendable());
        assert!(HealthState::Degraded.is_sendable());
        assert!(!HealthState::Down.is_sendable());
    }

    #[test]
    fn message_class_set_all_carries_every_class() {
        let s = MessageClassSet::all();
        assert!(s.carries(MessageClass::Control));
        assert!(s.carries(MessageClass::Clipboard));
        assert!(s.carries(MessageClass::FileBulk));
        assert!(s.carries(MessageClass::Notification));
    }

    #[test]
    fn message_class_set_small_only_blocks_file_bulk() {
        let s = MessageClassSet::small_only();
        assert!(s.carries(MessageClass::Control));
        assert!(s.carries(MessageClass::Clipboard));
        assert!(!s.carries(MessageClass::FileBulk));
        assert!(s.carries(MessageClass::Notification));
    }

    #[test]
    fn transport_error_code_and_family_are_stable() {
        // The router log line "transport=kdc_tls family=unreachable
        // code=nat_blocked" depends on these tokens never changing
        // out from under audit consumers.
        let e = TransportError::Unreachable {
            code: "nat_blocked",
        };
        assert_eq!(e.code(), "nat_blocked");
        assert_eq!(e.family(), "unreachable");

        let e = TransportError::HandshakeFailed {
            code: "cert_mismatch",
        };
        assert_eq!(e.code(), "cert_mismatch");
        assert_eq!(e.family(), "handshake_failed");

        let e = TransportError::Misconfigured {
            code: "missing_key",
        };
        assert_eq!(e.code(), "missing_key");
        assert_eq!(e.family(), "misconfigured");

        let e = TransportError::Io { code: "timeout" };
        assert_eq!(e.code(), "timeout");
        assert_eq!(e.family(), "io");
    }

    #[test]
    fn transport_error_display_includes_family_and_code() {
        let e = TransportError::Unreachable {
            code: "nat_blocked",
        };
        let s = format!("{e}");
        assert!(s.contains("unreachable"));
        assert!(s.contains("nat_blocked"));
    }

    /// Compile-time guard that the trait stays object-safe. If a
    /// future edit adds a non-object-safe method (e.g. `Self: Sized`
    /// constraint, generic method without `where Self: Sized`),
    /// this won't compile.
    #[allow(dead_code)]
    fn _trait_is_object_safe(t: Box<dyn Transport>) -> TransportKind {
        t.kind()
    }
}
