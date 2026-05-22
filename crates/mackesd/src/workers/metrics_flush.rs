//! KDC2-1.12.c — periodic textfile-collector flush worker.
//!
//! Owns shared handles to every live counter + histogram + on
//! each tick snapshots them into a Prometheus textfile under
//! `/var/lib/node_exporter/textfile_collector/mackesd.prom`
//! via [`crate::metrics::write_textfile`]. The atomic temp-file
//! + rename inside `write_textfile` means the collector never
//! reads a half-written snapshot.
//!
//! Cadence: 10s — matches the mesh-router tick + the Prometheus
//! scrape default. Worth tuning per-deploy via a future
//! `policy.toml` knob; baked-in for now.

#![cfg(feature = "async-services")]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tracing::{debug, info, warn};

use crate::metrics::{write_textfile, Counter, Histogram};

use super::{ShutdownToken, Worker};

/// Tick cadence — matches mesh-router. Operator-tunable in a
/// future policy.toml knob; baked-in for now.
const TICK: Duration = Duration::from_secs(10);

/// Async worker that flushes Counter + Histogram snapshots to
/// the textfile collector path on every tick.
pub struct MetricsFlushWorker {
    dir: PathBuf,
    counters: Vec<Counter>,
    histograms: Vec<Arc<Mutex<Histogram>>>,
}

impl MetricsFlushWorker {
    /// Construct with the target collector directory + a static
    /// list of counters + shared handles to every histogram the
    /// daemon publishes. Histogram handles are `Arc<Mutex<...>>`
    /// so other workers (e.g. mesh_router) observe into the
    /// same memory; this worker just reads + snapshots.
    #[must_use]
    pub fn new(
        dir: PathBuf,
        counters: Vec<Counter>,
        histograms: Vec<Arc<Mutex<Histogram>>>,
    ) -> Self {
        Self {
            dir,
            counters,
            histograms,
        }
    }

    /// Snapshot the current state + write the textfile. Pure
    /// helper exposed for tests that want to drive a single
    /// flush without spinning the worker.
    pub fn flush_once(&self) -> std::io::Result<PathBuf> {
        let histograms: Vec<Histogram> = self
            .histograms
            .iter()
            .filter_map(|h| h.lock().ok().map(|g| g.clone()))
            .collect();
        write_textfile(&self.dir, &self.counters, &histograms)
    }
}

#[async_trait::async_trait]
impl Worker for MetricsFlushWorker {
    fn name(&self) -> &'static str {
        "metrics-flush"
    }

    async fn run(&mut self, mut shutdown: ShutdownToken) -> anyhow::Result<()> {
        info!(
            dir = %self.dir.display(),
            counters = self.counters.len(),
            histograms = self.histograms.len(),
            "metrics-flush: starting",
        );
        // Make the directory before the first tick so the
        // initial flush has somewhere to land.
        if let Err(e) = std::fs::create_dir_all(&self.dir) {
            warn!(
                error = %e,
                dir = %self.dir.display(),
                "metrics-flush: directory create failed; will retry each tick",
            );
        }
        let mut interval = tokio::time::interval(TICK);
        // Skip the immediate first tick so the worker logs
        // "started" cleanly before flushing anything.
        interval.tick().await;
        loop {
            tokio::select! {
                _ = shutdown.wait() => {
                    info!("metrics-flush: shutdown requested; exiting");
                    return Ok(());
                }
                _ = interval.tick() => {
                    match self.flush_once() {
                        Ok(path) => debug!(path = %path.display(), "metrics-flush: tick"),
                        Err(e) => warn!(error = %e, "metrics-flush: write failed"),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{kdc2_router_decision_us, Bucket};
    use std::collections::BTreeMap;

    fn sample_histogram() -> Histogram {
        Histogram {
            name: "x_seconds",
            help: "test",
            buckets: vec![Bucket { le: 1.0, count: 0 }],
            sum: 0.0,
            count: 0,
        }
    }

    #[test]
    fn worker_name_matches_module() {
        let w = MetricsFlushWorker::new(PathBuf::from("/tmp"), vec![], vec![]);
        assert_eq!(w.name(), "metrics-flush");
    }

    #[test]
    fn flush_once_writes_counter_and_histogram_rows() {
        let tmp = tempfile::tempdir().unwrap();
        let counter = Counter {
            name: "test_counter_total",
            help: "test counter",
            value: 42,
            labels: BTreeMap::new(),
        };
        let hist = Arc::new(Mutex::new(sample_histogram()));
        let w =
            MetricsFlushWorker::new(tmp.path().to_path_buf(), vec![counter], vec![hist.clone()]);
        let path = w.flush_once().unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("test_counter_total 42"));
        assert!(content.contains("x_seconds_bucket"));
    }

    #[test]
    fn flush_once_snapshots_live_observations() {
        let tmp = tempfile::tempdir().unwrap();
        let hist = Arc::new(Mutex::new(kdc2_router_decision_us()));
        let w = MetricsFlushWorker::new(tmp.path().to_path_buf(), vec![], vec![hist.clone()]);
        // Observe 5 samples — flush sees them.
        hist.lock().unwrap().observe(500.0);
        hist.lock().unwrap().observe(800.0);
        hist.lock().unwrap().observe(1200.0);
        let path = w.flush_once().unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        // _count row shows the 3 observations.
        assert!(content.contains("kdc2_router_decision_us_count 3"));
        assert!(content.contains("kdc2_router_decision_us_bucket"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn worker_exits_on_shutdown_request() {
        let tmp = tempfile::tempdir().unwrap();
        let mut w = MetricsFlushWorker::new(tmp.path().to_path_buf(), vec![], vec![]);
        let (tx, rx) = tokio::sync::watch::channel(false);
        let token = super::super::ShutdownToken::from_receiver(rx);
        let handle = tokio::spawn(async move { w.run(token).await });
        tx.send(true).expect("shutdown channel intact");
        let result = handle.await.expect("worker join");
        assert!(result.is_ok());
    }
}
