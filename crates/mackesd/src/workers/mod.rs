//! v2.0.0 Phase A.2 (locked 2026-05-19) — in-process worker pool.
//!
//! The unified backend folds 8 standalone Python daemons (and one
//! Rust bridge) into a single `mackesd` process. Each former-daemon
//! becomes a [`Worker`] task driven by [`Supervisor`]. Worker bodies
//! land in Phase B; this module ships the trait surface, the shutdown
//! plumbing, and the per-worker join semantics every Phase B worker
//! will share.
//!
//! Design choices (locked via the 2026 stack survey 2026-05-19):
//!
//! * **Async runtime: tokio** (full features). The legacy reconcile
//!   loop (`crate::worker`) keeps its `std::thread` model — they
//!   coexist by living in separate scheduler domains.
//! * **Per-worker future: native `async fn` via `async_trait`**.
//!   Object-safety matters because the supervisor stores
//!   `Box<dyn Worker>`; native async-fn-in-trait drops object safety,
//!   so we keep `async_trait` for this trait only.
//! * **Restart policy: Erlang OTP-ish**. Phase B layers the
//!   `task-supervisor` crate (already a dep) on top of this trait so
//!   each worker gets per-task restart back-off + health-tick
//!   semantics. Phase A ships only the *contract*; the supervisor
//!   here is the minimal "spawn-and-shutdown" version.
//!
//! All public types are gated behind the `async-services` feature so
//! a fresh checkout that only builds the sync read-API doesn't pull
//! tokio into its dep tree.

#![cfg(feature = "async-services")]

use std::sync::Arc;

use anyhow::Context;
use tokio::sync::watch;
use tokio::task::JoinSet;
use tracing::{error, info, warn};

/// Shutdown signal handed to every worker. Workers should `select!`
/// on the underlying `watch::Receiver` so they exit promptly when
/// the supervisor requests stop. Cloning is cheap (it's a watch
/// receiver under the hood).
#[derive(Clone, Debug)]
pub struct ShutdownToken {
    pub(crate) rx: watch::Receiver<bool>,
}

impl ShutdownToken {
    /// Construct a token from a raw watch receiver. Crate-private —
    /// the supervisor's [`Supervisor::token`] is the public surface
    /// for normal callers; this constructor lets sibling worker
    /// modules build a token from a freshly-paired sender/receiver
    /// pair in their unit tests.
    #[must_use]
    pub(crate) fn from_receiver(rx: watch::Receiver<bool>) -> Self {
        Self { rx }
    }

    /// `true` once shutdown has been requested. Workers should poll
    /// or `await` on [`Self::changed`] for prompt notification.
    #[must_use]
    pub fn is_shutdown(&self) -> bool {
        *self.rx.borrow()
    }

    /// Async wait for shutdown. Resolves the first time the
    /// supervisor flips the flag to `true`. Returns immediately if
    /// shutdown was already requested.
    pub async fn wait(&mut self) {
        if self.is_shutdown() {
            return;
        }
        // `changed()` errors only when the sender is dropped — at
        // which point we're shutting down anyway, so treat it as
        // shutdown-requested.
        let _ = self.rx.changed().await;
    }
}

// v2.0.0 Phase B workers reparented under workers/. Each is a thin
// adapter over an existing sync implementation today; they grow real
// bodies as Phase B fills in.
pub mod ansible_pull;
pub mod clipboard;
pub mod derp;
pub mod fs_sync;
pub mod heartbeat;
pub mod kdc_bridge;
pub mod mdns;
pub mod media_sync;
pub mod nats;
pub mod notification_relay;
pub mod perf;
pub mod remmina_sync;
pub mod subprocess_tick;
pub mod thumbnailer;
pub mod wol;

/// Every worker registered with the supervisor implements this
/// trait. The trait is `async_trait` because the supervisor stores
/// `Box<dyn Worker>`, which native async-fn-in-trait doesn't yet
/// support.
#[async_trait::async_trait]
pub trait Worker: Send + 'static {
    /// Short, stable identifier used in logs + `mackesd healthz`
    /// output. Should be `kebab-case` and match the matching
    /// `crates/mackesd/src/workers/<name>.rs` module name (e.g.
    /// `clipboard`, `mdns`, `notifications-server`).
    fn name(&self) -> &'static str;

    /// Body of the worker. Runs on the tokio runtime until
    /// `shutdown.wait().await` resolves OR the body returns. Errors
    /// returned here surface to the supervisor's restart logic
    /// (Phase B); for Phase A the supervisor simply logs and exits
    /// the join.
    async fn run(&mut self, shutdown: ShutdownToken) -> anyhow::Result<()>;
}

/// Restart policy for a worker. Phase A only honors `Never` and
/// `OnFailure` — Phase B integrates the `task-supervisor` crate to
/// implement back-off + max-restarts + circuit-breaker semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicy {
    /// Don't restart — once the worker returns (Ok or Err), the
    /// supervisor records the outcome and moves on. Right for
    /// one-shot timer workers like `media_sync`.
    Never,
    /// Restart only if the worker returned `Err`. Right for
    /// long-running watchers (`clipboard`, `mdns`, `notification_relay`).
    OnFailure,
    /// Restart on any return (Ok or Err). Right for "should never
    /// exit" workers like `notifications_server`.
    Always,
}

/// Declarative registration: a worker + its restart policy. The
/// supervisor builds its task list from a `Vec<Spawn>`.
pub struct Spawn {
    /// Worker to spawn. Boxed for trait-object storage.
    pub worker: Box<dyn Worker>,
    /// Restart policy.
    pub policy: RestartPolicy,
}

impl Spawn {
    /// Convenience constructor.
    pub fn new<W: Worker>(worker: W, policy: RestartPolicy) -> Self {
        Self {
            worker: Box::new(worker),
            policy,
        }
    }
}

/// Minimal in-process supervisor. Phase A scope: spawn each worker
/// once, log restarts, broadcast shutdown via a watch channel,
/// `join_all` on stop. Phase B re-wraps this in `task-supervisor` for
/// per-task back-off + add/remove-at-runtime semantics.
pub struct Supervisor {
    shutdown_tx: Arc<watch::Sender<bool>>,
    shutdown_rx: watch::Receiver<bool>,
    join: JoinSet<(&'static str, anyhow::Result<()>)>,
}

impl Default for Supervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl Supervisor {
    /// Construct an empty supervisor. Use [`Self::spawn`] to register
    /// workers, then [`Self::join_all`] / [`Self::shutdown_and_join`]
    /// to drive them.
    #[must_use]
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(false);
        Self {
            shutdown_tx: Arc::new(tx),
            shutdown_rx: rx,
            join: JoinSet::new(),
        }
    }

    /// Issue every spawned worker a fresh shutdown token cloned from
    /// our channel.
    #[must_use]
    pub fn token(&self) -> ShutdownToken {
        ShutdownToken {
            rx: self.shutdown_rx.clone(),
        }
    }

    /// Spawn a worker. The supervisor honors `Spawn::policy` for
    /// restart decisions (Phase A: `Never`/`OnFailure`/`Always`
    /// implemented via a self-spawning loop inside `run_one`).
    pub fn spawn(&mut self, spec: Spawn) {
        let token = self.token();
        let Spawn { mut worker, policy } = spec;
        let name = worker.name();
        let shutdown = token;
        self.join.spawn(async move {
            // `break outcome` carries the worker's final result out
            // of the loop, so we don't need a pre-initialized
            // `last_result` slot (which would dead-code in the
            // can-never-be-empty `loop {}`).
            let last_result: anyhow::Result<()> = loop {
                info!(worker = %name, "starting worker");
                let token_for_run = shutdown.clone();
                let outcome = worker.run(token_for_run).await;
                let should_restart = match (policy, &outcome) {
                    (RestartPolicy::Never, _) => false,
                    (RestartPolicy::OnFailure, Err(_)) => true,
                    (RestartPolicy::OnFailure, Ok(())) => false,
                    (RestartPolicy::Always, _) => true,
                };
                match &outcome {
                    Ok(()) => info!(worker = %name, "worker returned Ok"),
                    Err(e) => warn!(worker = %name, error = ?e, "worker returned Err"),
                }
                if !should_restart {
                    break outcome;
                }
                if shutdown.is_shutdown() {
                    info!(worker = %name, "shutdown requested; not restarting");
                    break outcome;
                }
                // Phase A: fixed 250 ms back-off so a hot-looping
                // bug doesn't pin a core. Phase B replaces this
                // with task-supervisor's exponential back-off.
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                // No `shutdown.wait().await` here — that would block
                // restarts indefinitely. The 250 ms sleep is the
                // restart delay; the worker's next `run()` should
                // observe `shutdown.is_shutdown()` itself.
            };
            (name, last_result)
        });
    }

    /// Wait until every spawned worker has finished. The runtime
    /// drives them; this just blocks until the join set drains.
    pub async fn join_all(&mut self) -> Vec<(&'static str, anyhow::Result<()>)> {
        let mut outcomes = Vec::new();
        while let Some(joined) = self.join.join_next().await {
            match joined {
                Ok(o) => outcomes.push(o),
                Err(e) => {
                    error!(error = ?e, "worker task panicked");
                }
            }
        }
        outcomes
    }

    /// Signal shutdown and drain. The watch channel's atomic flip
    /// means every cloned [`ShutdownToken`] sees `true` on its next
    /// poll.
    ///
    /// # Errors
    ///
    /// Returns an error only if the watch sender is somehow already
    /// closed, which would indicate a programmer error.
    pub async fn shutdown_and_join(
        &mut self,
    ) -> anyhow::Result<Vec<(&'static str, anyhow::Result<()>)>> {
        self.shutdown_tx
            .send(true)
            .context("broadcasting shutdown to workers")?;
        Ok(self.join_all().await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountdownWorker {
        remaining: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl Worker for CountdownWorker {
        fn name(&self) -> &'static str {
            "countdown"
        }
        async fn run(&mut self, mut shutdown: ShutdownToken) -> anyhow::Result<()> {
            loop {
                let n = self.remaining.fetch_sub(1, Ordering::SeqCst);
                if n == 0 {
                    return Ok(());
                }
                tokio::select! {
                    _ = shutdown.wait() => return Ok(()),
                    _ = tokio::time::sleep(std::time::Duration::from_millis(5)) => {}
                }
            }
        }
    }

    struct ShutdownObserver {
        observed: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl Worker for ShutdownObserver {
        fn name(&self) -> &'static str {
            "observer"
        }
        async fn run(&mut self, mut shutdown: ShutdownToken) -> anyhow::Result<()> {
            shutdown.wait().await;
            self.observed.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    struct FailOnce {
        attempts: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl Worker for FailOnce {
        fn name(&self) -> &'static str {
            "fail-once"
        }
        async fn run(&mut self, _shutdown: ShutdownToken) -> anyhow::Result<()> {
            let n = self.attempts.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                anyhow::bail!("intentional first-attempt failure")
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn worker_runs_to_completion_under_never_policy() {
        let mut sup = Supervisor::new();
        let counter = Arc::new(AtomicUsize::new(3));
        sup.spawn(Spawn::new(
            CountdownWorker {
                remaining: counter.clone(),
            },
            RestartPolicy::Never,
        ));
        let outcomes = sup.join_all().await;
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].0, "countdown");
        assert!(outcomes[0].1.is_ok());
    }

    #[tokio::test]
    async fn shutdown_token_propagates_to_workers() {
        let mut sup = Supervisor::new();
        let observed = Arc::new(AtomicUsize::new(0));
        sup.spawn(Spawn::new(
            ShutdownObserver {
                observed: observed.clone(),
            },
            RestartPolicy::Never,
        ));
        sup.shutdown_and_join().await.unwrap();
        assert_eq!(observed.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn on_failure_policy_restarts_until_ok() {
        let mut sup = Supervisor::new();
        let attempts = Arc::new(AtomicUsize::new(0));
        sup.spawn(Spawn::new(
            FailOnce {
                attempts: attempts.clone(),
            },
            RestartPolicy::OnFailure,
        ));
        let outcomes = sup.join_all().await;
        assert_eq!(outcomes.len(), 1);
        // Final attempt should have returned Ok.
        assert!(outcomes[0].1.is_ok());
        assert!(attempts.load(Ordering::SeqCst) >= 2);
    }

    #[test]
    fn restart_policy_match_completeness() {
        // Compile-time check that every variant is named here. If a
        // new variant is added, this match will fail to compile.
        for p in [
            RestartPolicy::Never,
            RestartPolicy::OnFailure,
            RestartPolicy::Always,
        ] {
            match p {
                RestartPolicy::Never | RestartPolicy::OnFailure | RestartPolicy::Always => {}
            }
        }
    }
}
