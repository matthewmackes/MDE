//! MESH-A-4.c.2 (v5.0.0) — periodic surrounding-host discovery worker.
//!
//! Runs the MESH-A-4.c.1 local sweep (mDNS → reverse-DNS → ARP-MAC →
//! OUI vendor → classify) every 10 min (R8-Q12) and writes a per-peer
//! snapshot to `~/.local/share/mde/surrounding/<host>/<iso>-<hash>.json`.
//! The directory lands under mesh-storage once mounted, so every peer
//! reads the union of all peers' LAN-neighbour views (R8-Q13).
//!
//! Reuses the [`crate::surrounding_hosts`] collectors + classifier and
//! the netassess [`snapshot_filename`] content-addressing. HTTP-banner
//! + nmap `-O` fingerprint (A-4.c.3) + duplicate-coalescing / roaming /
//! retention (A-4.c.4) + manual Bus refresh land as follow-ons.
//!
//! Shell-outs that aren't present degrade to empty (the snapshot still
//! writes with whatever was collected); the pure collectors/classifier
//! are unit-tested in `surrounding_hosts`, the live sweep is
//! HW-bench-gated (§0.15).

#![cfg(feature = "async-services")]

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::surrounding_hosts::{
    arp_neigh_map, classify, collect_mdns, enrich_hosts, hosts_from_mdns, load_system_oui,
    refine_unknown_with_http, reverse_dns, HostSignals, SurroundingHost,
};
use crate::workers::netassess::{snapshot_filename, trim_older_than};

use super::{ShutdownToken, Worker};

/// Active-sweep cadence — 10 minutes (R8-Q12).
pub const DEFAULT_TICK_INTERVAL: Duration = Duration::from_secs(600);

/// Snapshot retention — 30 days in ms (R8-Q15 archive horizon). The
/// 7-day "fade" is a Portal render concern; the worker drops snapshots
/// older than 30 days each tick (reusing the netassess trim).
pub const RETENTION_MS: i64 = 30 * 24 * 60 * 60 * 1_000;

/// avahi-browse binary the mDNS collector shells out to.
const AVAHI_BROWSE: &str = "avahi-browse";

/// Worker handle.
pub struct SurroundingWorker {
    host: String,
    base_dir: PathBuf,
    tick: Duration,
}

impl SurroundingWorker {
    /// Construct with production defaults. `host` is this peer's name;
    /// `base_dir` is the `surrounding` root
    /// (`~/.local/share/mde/surrounding` in prod).
    #[must_use]
    pub fn new(host: String, base_dir: PathBuf) -> Self {
        Self {
            host,
            base_dir,
            tick: DEFAULT_TICK_INTERVAL,
        }
    }

    /// Override the sweep cadence. Used in tests.
    #[must_use]
    pub fn with_tick(mut self, d: Duration) -> Self {
        self.tick = d;
        self
    }

    /// Run one discovery sweep: mDNS browse → reverse-DNS fill →
    /// ARP-MAC + OUI-vendor enrichment → classify. `now_ms` stamps the
    /// hosts' first/last-seen.
    fn sweep(&self, now_ms: i64) -> Vec<SurroundingHost> {
        let records = collect_mdns(AVAHI_BROWSE);
        let mut hosts = hosts_from_mdns(&records, now_ms);
        for host in &mut hosts {
            if host.hostname.is_empty() {
                if let Some(name) = reverse_dns(&host.ip) {
                    host.hostname = name;
                    let sig = HostSignals {
                        mdns_services: host.services.clone(),
                        hostname: host.hostname.clone(),
                        ..Default::default()
                    };
                    host.host_type = classify(&sig);
                }
            }
        }
        let mut hosts = enrich_hosts(hosts, &arp_neigh_map(), &load_system_oui());
        refine_unknown_with_http(&mut hosts);
        hosts
    }

    fn host_dir(&self) -> PathBuf {
        self.base_dir.join(&self.host)
    }

    fn write_snapshot(&self, hosts: &[SurroundingHost]) {
        let dir = self.host_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            tracing::debug!(error = %e, "surrounding: mkdir failed");
            return;
        }
        let Ok(body) = serde_json::to_string_pretty(hosts) else {
            return;
        };
        let iso = chrono::Local::now().format("%Y%m%dT%H%M%S").to_string();
        let path = dir.join(snapshot_filename(&iso, &body));
        if let Err(e) = std::fs::write(&path, &body) {
            tracing::debug!(error = %e, "surrounding: write failed");
        }
    }

    fn tick_once(&self) {
        let now_ms = now_epoch_ms();
        let hosts = self.sweep(now_ms);
        self.write_snapshot(&hosts);
        // R8-Q15 — drop snapshots past the 30-day archive horizon.
        let _ = trim_older_than(&self.host_dir(), now_ms - RETENTION_MS);
    }
}

fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[async_trait::async_trait]
impl Worker for SurroundingWorker {
    fn name(&self) -> &'static str {
        "surrounding_hosts"
    }

    async fn run(&mut self, mut shutdown: ShutdownToken) -> anyhow::Result<()> {
        let mut tick = tokio::time::interval(self.tick);
        tick.tick().await; // consume the immediate first tick — first sweep lands after `tick`.
        loop {
            tokio::select! {
                _ = tick.tick() => {
                    self.tick_once();
                }
                _ = shutdown.wait() => break,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surrounding_hosts::{HostType, TrustState};

    #[test]
    fn worker_name_and_host_dir() {
        let w = SurroundingWorker::new("alice".into(), PathBuf::from("/base"));
        assert_eq!(w.name(), "surrounding_hosts");
        assert_eq!(w.host_dir(), PathBuf::from("/base/alice"));
    }

    #[test]
    fn with_tick_overrides_cadence() {
        let w = SurroundingWorker::new("h".into(), PathBuf::from("/b"))
            .with_tick(Duration::from_secs(5));
        assert_eq!(w.tick, Duration::from_secs(5));
    }

    #[test]
    fn write_snapshot_writes_colon_free_roundtrippable_file() {
        let tmp = tempfile::tempdir().unwrap();
        let w = SurroundingWorker::new("alice".into(), tmp.path().to_path_buf());
        let hosts = vec![SurroundingHost {
            ip: "192.168.1.1".into(),
            mac: "00:00:0c:aa:bb:cc".into(),
            vendor: "Cisco Systems".into(),
            hostname: "gw".into(),
            services: vec![],
            host_type: HostType::Router,
            trust: TrustState::Unknown,
            first_seen_ms: 1,
            last_seen_ms: 1,
        }];
        w.write_snapshot(&hosts);

        let dir = tmp.path().join("alice");
        let entries: Vec<_> = std::fs::read_dir(&dir).unwrap().filter_map(Result::ok).collect();
        assert_eq!(entries.len(), 1, "one snapshot written");
        let name = entries[0].file_name().into_string().unwrap();
        assert!(name.ends_with(".json"), "snapshot is JSON");
        assert!(!name.contains(':'), "filename is colon-free");
        let body = std::fs::read_to_string(entries[0].path()).unwrap();
        let back: Vec<SurroundingHost> = serde_json::from_str(&body).unwrap();
        assert_eq!(back, hosts, "snapshot round-trips");
    }
}
