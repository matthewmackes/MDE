//! KDC2-1.8 — mesh-router worker.
//!
//! Long-running worker that holds the live per-peer routing
//! state + a registry of transport impls. On every tick it:
//!
//!   1. Walks every known peer.
//!   2. Probes each transport (cheap per-probe call).
//!   3. Updates the peer's [`PeerPath`] health + considers a
//!      transport switch.
//!   4. Emits a `PathSwitch` audit-chain entry whenever the
//!      primary transport flips (with the [`SwitchReason`]).
//!
//! Concrete scoring + transport selection (KDC2-1.9) +
//! audit-chain feed (KDC2-1.12) land as follow-ups. This commit
//! ships the worker scaffold: trait impl, tick loop, registry,
//! state-map.

#![cfg(feature = "async-services")]

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};

use mackes_transport::peer_path::{PeerPath, SwitchReason};
use mackes_transport::{Transport, TransportKind};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::metrics::Histogram;
use crate::transport::audit::PathSwitchEvent;

/// Shared handle to the `kdc2_router_decision_us` histogram —
/// the textfile-flush worker reads from this when assembling
/// the `.prom` snapshot.
pub type RouterMetrics = Arc<StdMutex<Histogram>>;

use super::{ShutdownToken, Worker};

/// Default tick cadence for the router. Matches the v12
/// connectivity-scope lock's "10s roaming switch budget" — the
/// router probes once per tick so a transport degradation gets
/// noticed within one cadence interval.
const DEFAULT_TICK: Duration = Duration::from_secs(10);

/// Identifier for one peer in the mesh.
pub type PeerId = String;

/// Per-peer routing state map. Behind a `tokio::sync::RwLock` so
/// the supervisor's tick task + any future API readers (zbus
/// `dev.mackes.MDE.Mesh.PathFor()`) can share access.
pub type RouterState = Arc<RwLock<HashMap<PeerId, PeerPath>>>;

/// Registered transport implementations. `Vec<Arc<dyn Transport>>`
/// so the worker can hold multiple references (clone the Arc into
/// the tick loop) without giving up ownership of the slice.
pub type TransportRegistry = Arc<Vec<Arc<dyn Transport>>>;

/// Async worker that ticks the mesh router on a fixed cadence.
///
/// State + registry are passed in at construction so the
/// supervisor's restart logic can hand the same router state
/// back after a worker restart — losing the in-memory PeerPath
/// table on every restart would defeat the whole point of
/// tracking health history.
pub struct MeshRouterWorker {
    state: RouterState,
    registry: TransportRegistry,
    tick: Duration,
    /// KDC2-1.12.b — optional handle to the
    /// `kdc2_router_decision_us` histogram. When `Some`, every
    /// `tick_once` records its decision microseconds via
    /// `Histogram::observe`. `None` for tests + bootstrap paths
    /// that don't care about telemetry.
    metrics: Option<RouterMetrics>,
}

impl MeshRouterWorker {
    /// Construct a new mesh-router worker with the default
    /// 10s tick cadence.
    #[must_use]
    pub fn new(state: RouterState, registry: TransportRegistry) -> Self {
        Self {
            state,
            registry,
            tick: DEFAULT_TICK,
            metrics: None,
        }
    }

    /// Override the tick cadence. Useful for tests (set to
    /// 100 ms) and the future operator-tunable
    /// `/etc/mde/connect/policy.toml` (KDC2-1.10).
    #[must_use]
    pub fn with_tick(mut self, tick: Duration) -> Self {
        self.tick = tick;
        self
    }

    /// KDC2-1.12.b — attach a shared
    /// `kdc2_router_decision_us` histogram. Subsequent ticks
    /// observe their decision microseconds into the supplied
    /// handle.
    #[must_use]
    pub fn with_metrics(mut self, metrics: RouterMetrics) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Total number of registered transports. Used by tests +
    /// `mackesd healthz` to confirm the worker has the expected
    /// transport set wired.
    #[must_use]
    pub fn transport_count(&self) -> usize {
        self.registry.len()
    }

    /// Total number of peers currently tracked. Cheap async
    /// read; exposed for instrumentation.
    pub async fn peer_count(&self) -> usize {
        self.state.read().await.len()
    }
}

#[async_trait::async_trait]
impl Worker for MeshRouterWorker {
    fn name(&self) -> &'static str {
        "mesh-router"
    }

    async fn run(&mut self, mut shutdown: ShutdownToken) -> anyhow::Result<()> {
        info!(
            transport_count = self.transport_count(),
            tick_ms = self.tick.as_millis() as u64,
            "mesh-router: starting",
        );

        let mut interval = tokio::time::interval(self.tick);
        // First tick fires immediately; skip it so the first
        // observation isn't done before any transport has had a
        // chance to settle after worker startup.
        interval.tick().await;

        loop {
            tokio::select! {
                _ = shutdown.wait() => {
                    info!("mesh-router: shutdown requested; exiting");
                    return Ok(());
                }
                _ = interval.tick() => {
                    self.tick_once().await;
                }
            }
        }
    }
}

impl MeshRouterWorker {
    /// One iteration of the router's main loop. Pure-async — no
    /// shared mutable state outside the locked `state` map.
    ///
    /// KDC2-1.8 scaffolds the loop; KDC2-1.12 wires the audit-
    /// chain emission seam. The actual scorer call is folded
    /// into [`Self::scored_primary_for`] so unit tests can
    /// exercise the path-switch detection logic without running
    /// the full tick loop.
    async fn tick_once(&self) {
        let started = Instant::now();
        let peer_count = self.peer_count().await;
        let transport_count = self.transport_count();
        debug!(
            peer_count,
            transport_count,
            "mesh-router: tick (scaffold; full scorer integration KDC2-1.9 follow-up)",
        );
        // KDC2-1.12.b — record decision time. Use saturating
        // cast so a freakishly long tick (clock skew, stall)
        // bucket-saturates rather than panics.
        if let Some(m) = &self.metrics {
            let us = started.elapsed().as_micros() as f64;
            if let Ok(mut guard) = m.lock() {
                guard.observe(us);
            }
        }
    }

    /// Pure path-switch detector. Given a peer's current
    /// `PeerPath` and a fresh scoring result, return the
    /// `PathSwitchEvent` to emit if the primary changed —
    /// `None` when the new selection matches the old one.
    ///
    /// Exposed for unit tests + future direct callers that want
    /// to drive the scoring + audit side without owning the
    /// tick loop.
    #[must_use]
    pub fn detect_switch(
        prior: &PeerPath,
        new_primary: TransportKind,
        new_reason: SwitchReason,
        now_ms: i64,
    ) -> Option<PathSwitchEvent> {
        if prior.primary == new_primary {
            return None;
        }
        Some(PathSwitchEvent::switch(
            prior.peer_id.clone(),
            Some(prior.primary),
            new_primary,
            new_reason,
            now_ms,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mackes_transport::conformance::MockTransport;
    use mackes_transport::TransportKind;

    fn new_state() -> RouterState {
        Arc::new(RwLock::new(HashMap::new()))
    }

    fn new_registry() -> TransportRegistry {
        Arc::new(vec![
            Arc::new(MockTransport::new(TransportKind::DirectUdp)) as Arc<dyn Transport>,
            Arc::new(MockTransport::new(TransportKind::KdcTls)) as Arc<dyn Transport>,
        ])
    }

    #[test]
    fn worker_construction_records_transport_count() {
        let w = MeshRouterWorker::new(new_state(), new_registry());
        assert_eq!(w.transport_count(), 2);
    }

    #[test]
    fn worker_with_tick_overrides_default_cadence() {
        let w = MeshRouterWorker::new(new_state(), new_registry())
            .with_tick(Duration::from_millis(50));
        assert_eq!(w.tick, Duration::from_millis(50));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn peer_count_starts_at_zero() {
        let w = MeshRouterWorker::new(new_state(), new_registry());
        assert_eq!(w.peer_count().await, 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn peer_count_reflects_inserted_peers() {
        let state = new_state();
        let w = MeshRouterWorker::new(state.clone(), new_registry());
        {
            let mut s = state.write().await;
            s.insert(
                "peer-A".into(),
                PeerPath::initial("peer-A".into(), TransportKind::DirectUdp),
            );
            s.insert(
                "peer-B".into(),
                PeerPath::initial("peer-B".into(), TransportKind::KdcTls),
            );
        }
        assert_eq!(w.peer_count().await, 2);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn tick_once_records_decision_us_when_metrics_attached() {
        // KDC2-1.12.b lock — the wired-in histogram must
        // see a sample after one tick_once.
        let metrics = Arc::new(StdMutex::new(crate::metrics::kdc2_router_decision_us()));
        let w = MeshRouterWorker::new(new_state(), new_registry())
            .with_metrics(Arc::clone(&metrics));
        w.tick_once().await;
        let snapshot = metrics.lock().unwrap();
        assert_eq!(snapshot.count, 1, "tick_once must record one sample");
        // Some bucket must be non-zero — concrete value depends
        // on machine speed; the test_loop is well under 50 ms.
        assert!(snapshot.buckets.iter().any(|b| b.count > 0));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn tick_once_without_metrics_is_a_noop_observation() {
        // Default constructor doesn't attach metrics; tick must
        // still run cleanly (regression guard against panics).
        let w = MeshRouterWorker::new(new_state(), new_registry());
        w.tick_once().await;
        // No metrics handle to assert on; reaching this line is
        // the lock.
    }

    #[tokio::test(flavor = "current_thread")]
    async fn worker_name_matches_module() {
        let w = MeshRouterWorker::new(new_state(), new_registry());
        assert_eq!(w.name(), "mesh-router");
    }

    #[test]
    fn detect_switch_returns_none_when_primary_unchanged() {
        let prior = PeerPath::initial("p".into(), TransportKind::DirectUdp);
        let r = MeshRouterWorker::detect_switch(
            &prior,
            TransportKind::DirectUdp,
            SwitchReason::Initial,
            0,
        );
        assert!(r.is_none(), "primary unchanged → no audit emission");
    }

    #[test]
    fn detect_switch_emits_when_primary_flips() {
        let prior = PeerPath::initial("peer-A".into(), TransportKind::DirectUdp);
        let event = MeshRouterWorker::detect_switch(
            &prior,
            TransportKind::KdcTls,
            SwitchReason::HealthDegraded(TransportKind::DirectUdp),
            1_700_000_000_000,
        )
        .expect("primary flipped → event emitted");
        let summary = event.summary();
        assert!(summary.contains("peer=peer-A"));
        assert!(summary.contains("from=direct_udp"));
        assert!(summary.contains("to=kdc_tls"));
        assert!(summary.contains("reason=health_degraded_direct_udp"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn worker_exits_on_shutdown_request() {
        // Construct + spawn the worker. Trigger shutdown
        // immediately. Worker must exit cleanly (Ok(())) without
        // waiting for a tick.
        let state = new_state();
        let registry = new_registry();
        let mut w = MeshRouterWorker::new(state, registry)
            .with_tick(Duration::from_secs(60));

        // Build a fresh shutdown-token pair the same way every
        // other worker test does (clipboard.rs, fs_sync.rs).
        let (tx, rx) = tokio::sync::watch::channel(false);
        let token = super::super::ShutdownToken::from_receiver(rx);

        let handle = tokio::spawn(async move { w.run(token).await });
        // Flip the shutdown flag.
        tx.send(true).expect("shutdown channel intact");
        let result = handle.await.expect("worker join");
        assert!(result.is_ok(), "worker must exit Ok on shutdown");
    }
}
