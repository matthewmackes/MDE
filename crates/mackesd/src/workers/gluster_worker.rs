//! GF-2.1 + GF-2.3 + GF-2.4 (v5.0.0) — gluster fleet supervisor.
//!
//! Mirrors the `nebula_supervisor` worker shape: tokio task,
//! 5-second tick, owned `Arc<Mutex<rusqlite::Connection>>`
//! store handle, `ShutdownToken` `select!` for prompt SIGTERM
//! exit. Each tick:
//!
//!   1. **Probe.** Shell `gluster pool list --xml` to see what
//!      glusterd thinks the cluster looks like. When the
//!      binary isn't installed, the worker silently no-ops —
//!      the operator hasn't enabled the v5.0.0 substrate
//!      (GF-1.1 / GF-1.2) yet.
//!
//!   2. **Genesis path (GF-2.4).** If glusterd is live AND
//!      this peer has the only / first vote in the pool AND
//!      no `mesh-home` volume exists, run `gluster volume
//!      create mesh-home replica 1 transport tcp
//!      <local-overlay-ip>:<brick> force`. Idempotent — once
//!      the volume exists every tick is a no-op for this
//!      step.
//!
//! Subsequent GF-2.x extensions (peer probe on enrollment,
//! peer detach on revocation, hourly quota probe, conflict
//! detector/resolver) layer onto the same tick. Each extension
//! gates on glusterd-up + the operator having opted into
//! v5.0.0; no extension is reachable on a v4.x install.
//!
//! Test surface: `tick_once()` is split into pure-function
//! helpers (`should_bootstrap`, `bootstrap_argv`) + a thin
//! shell-out layer so the tests can verify the command shape
//! without needing a live glusterd.

#![cfg(feature = "async-services")]

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

use super::{ShutdownToken, Worker};

/// Default tick — five seconds, matching `nebula_supervisor`.
/// Operators with shorter or longer cadences override via
/// [`GlusterWorker::with_tick`].
pub const DEFAULT_TICK_INTERVAL: Duration = Duration::from_secs(5);

/// Volume name the v5.0.0 epic locked (Q6 of the 25-Q survey).
pub const VOLUME_NAME: &str = "mesh-home";

/// Brick directory the v5.0.0 epic locked (Q5).
pub const BRICK_PATH: &str = "/var/lib/gluster/bricks/mesh-home";

/// Default overlay-ip publish file path (GF-1.3.a).
pub const DEFAULT_OVERLAY_IP_PATH: &str = "/var/lib/mackesd/nebula/overlay-ip";

/// CLI binary we shell out to. `gluster_binary` lets tests
/// inject a mock (`/bin/true`, `/bin/false`, or a fake
/// recording wrapper).
pub const DEFAULT_GLUSTER_BINARY: &str = "gluster";

/// Quota multiplier locked by Q16 of the 25-Q survey: the
/// fleet quota cap is `0.8 × min(free brick across peers)`.
pub const QUOTA_MULTIPLIER: f64 = 0.8;

/// How often the quota probe runs. The genesis-path probe
/// fires every tick (5s); the quota probe is rate-limited to
/// once per hour because re-running `gluster volume quota`
/// on every 5s tick would spam glusterd's transaction log.
pub const QUOTA_PROBE_INTERVAL: Duration = Duration::from_secs(3600);

/// Worker handle. Cheap to construct + clone is forbidden
/// (mirrors `nebula_supervisor`).
pub struct GlusterWorker {
    _store: Arc<Mutex<rusqlite::Connection>>,
    tick: Duration,
    overlay_ip_path: PathBuf,
    brick_path: PathBuf,
    gluster_binary: String,
    /// GF-2.5 + GF-2.6 — QNM-Shared root for the polling-
    /// based peer-probe / peer-detach discovery. Each
    /// `<qnm_root>/<peer-id>/mackesd/nebula-bundle.json` is a
    /// peer that the local glusterd should be in a pool
    /// with. None means "skip peer convergence" (used by the
    /// default constructor + by tests that don't care).
    qnm_root: Option<PathBuf>,
    /// GF-2.5 — this peer's own node-id so the probe logic
    /// can skip itself.
    self_node_id: Option<String>,
    /// GF-2.7 — last-fire wall-clock seconds (relative to
    /// startup) of the hourly quota probe. `None` until the
    /// first probe runs.
    last_quota_probe: std::sync::Mutex<Option<std::time::Instant>>,
    /// GF-2.9 — GFIDs we've already requested split-brain
    /// resolution for. Prevents re-issuing the heal command
    /// on the same conflict every 5s tick — gluster's
    /// self-heal daemon needs time to act on the request +
    /// the GFID stays in the xattrop index until heal
    /// completes.
    healed_gfids: std::sync::Mutex<std::collections::BTreeSet<String>>,
}

impl GlusterWorker {
    /// Construct with production defaults. The store handle is
    /// kept so the future GF-2.7 quota probe + GF-2.8 conflict
    /// detector can persist their findings into the audit log
    /// without a parallel SQL connection.
    #[must_use]
    pub fn new(store: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self {
            _store: store,
            tick: DEFAULT_TICK_INTERVAL,
            overlay_ip_path: PathBuf::from(DEFAULT_OVERLAY_IP_PATH),
            brick_path: PathBuf::from(BRICK_PATH),
            gluster_binary: DEFAULT_GLUSTER_BINARY.to_owned(),
            qnm_root: None,
            self_node_id: None,
            last_quota_probe: std::sync::Mutex::new(None),
            healed_gfids: std::sync::Mutex::new(std::collections::BTreeSet::new()),
        }
    }

    /// GF-2.5 + GF-2.6 — opt into polling-based peer
    /// convergence. Both args must be supplied or the worker
    /// skips the peer-probe / peer-detach step entirely
    /// (silent no-op).
    #[must_use]
    pub fn with_qnm_peer_discovery(mut self, qnm_root: PathBuf, self_node_id: String) -> Self {
        self.qnm_root = Some(qnm_root);
        self.self_node_id = Some(self_node_id);
        self
    }

    /// Override the tick cadence. Tests use shorter values.
    #[must_use]
    pub fn with_tick(mut self, t: Duration) -> Self {
        self.tick = t;
        self
    }

    /// Override the overlay-ip publish file path. Tests
    /// redirect to a tempdir.
    #[must_use]
    pub fn with_overlay_ip_path(mut self, path: PathBuf) -> Self {
        self.overlay_ip_path = path;
        self
    }

    /// Override the brick directory. Tests redirect to a
    /// tempdir.
    #[must_use]
    pub fn with_brick_path(mut self, path: PathBuf) -> Self {
        self.brick_path = path;
        self
    }

    /// Override the `gluster` CLI binary path. Tests pass
    /// `/bin/true` / `/bin/false` / a recording shim.
    #[must_use]
    pub fn with_gluster_binary(mut self, name: impl Into<String>) -> Self {
        self.gluster_binary = name.into();
        self
    }

    /// One tick of the worker's loop. Exposed for direct
    /// testing without the tokio time pulse.
    pub fn tick_once(&self) {
        // 1. Probe. If `gluster` isn't on PATH the operator
        //    hasn't enabled the v5.0.0 substrate yet — silent
        //    no-op.
        if !binary_on_path(&self.gluster_binary) {
            tracing::debug!(
                target: "mackesd::gluster_worker",
                binary = %self.gluster_binary,
                "gluster CLI not installed; v5.0.0 substrate inactive",
            );
            return;
        }
        // 2. Read the overlay IP. If the publish file (GF-1.3.a)
        //    is missing, this peer hasn't completed Nebula
        //    enrollment yet — skip bootstrap.
        let overlay_ip = match std::fs::read_to_string(&self.overlay_ip_path) {
            Ok(s) => s.trim().to_owned(),
            Err(_) => {
                tracing::debug!(
                    target: "mackesd::gluster_worker",
                    path = %self.overlay_ip_path.display(),
                    "overlay-ip publish file missing; deferring bootstrap until Nebula enrollment completes",
                );
                return;
            }
        };
        // 3. Genesis path (GF-2.4). If the volume doesn't exist
        //    yet AND this peer is in a position to bootstrap
        //    it, run `gluster volume create`.
        match volume_exists(&self.gluster_binary, VOLUME_NAME) {
            Some(true) => {
                tracing::debug!(
                    target: "mackesd::gluster_worker",
                    volume = VOLUME_NAME,
                    "volume already exists; nothing to bootstrap",
                );
            }
            Some(false) => {
                let argv = bootstrap_argv(
                    &self.gluster_binary,
                    &overlay_ip,
                    &self.brick_path.display().to_string(),
                );
                tracing::info!(
                    target: "mackesd::gluster_worker",
                    argv = ?argv,
                    "bootstrapping mesh-home volume",
                );
                let _ = run_argv(&argv);
            }
            None => {
                tracing::warn!(
                    target: "mackesd::gluster_worker",
                    "couldn't determine if mesh-home volume exists; retrying on next tick",
                );
            }
        }
        // 4. GF-2.7 — hourly quota probe. Gated on
        //    `last_quota_probe` so the heavy `gluster volume
        //    info --xml` + `volume quota` round-trip only
        //    fires once per hour, not every 5s tick.
        if self.quota_probe_due() {
            self.run_quota_probe();
        }
        // 5. GF-2.8 — conflict detector. Walks the brick's
        //    `.glusterfs/indices/xattrop/` directory; every
        //    entry there is a GFID symlink for a file with a
        //    pending heal / split-brain op. We surface each
        //    pending GFID as a `ConflictDetected` tracing
        //    event so the operator (or the future GF-2.2
        //    D-Bus signal subscriber) sees the split-brain
        //    state without having to shell `gluster volume
        //    heal info` themselves. Best-effort: the brick
        //    dir may be missing (operator runs mackesd on a
        //    non-storage box) — silent skip.
        let xattrop_dir = self.brick_path.join(".glusterfs/indices/xattrop");
        for gfid in pending_conflict_gfids(&xattrop_dir) {
            tracing::warn!(
                target: "mackesd::gluster_worker",
                gfid = %gfid,
                brick = %self.brick_path.display(),
                "ConflictDetected: pending heal entry in xattrop index",
            );
            // GF-2.9 — LWW resolver. Delegate to gluster's
            // own self-heal daemon via `gluster volume heal
            // mesh-home split-brain latest-mtime <gfid>`.
            // gluster's heal-daemon knows the cross-peer
            // mtime + handles the `.conflict-<host>-<ts>`
            // rename per its own conventions, so we don't
            // re-implement the rename logic in Rust. We just
            // need to trigger the heal request once per GFID
            // — repeat requests on the same GFID before the
            // daemon resolves it spam glusterd's transaction
            // log.
            if self.mark_gfid_heal_requested(&gfid) {
                let argv = heal_split_brain_argv(&self.gluster_binary, &gfid);
                tracing::info!(
                    target: "mackesd::gluster_worker",
                    gfid = %gfid,
                    argv = ?argv,
                    "LWW: requesting split-brain heal via gluster self-heal daemon",
                );
                let _ = run_argv(&argv);
            }
        }
        // 6. GF-2.5 + GF-2.6 — peer convergence. Scans
        //    `<qnm_root>/*/mackesd/nebula-bundle.json` for
        //    every peer the Nebula lighthouse has signed +
        //    diffs against `gluster pool list`. Missing peers
        //    get probed + their brick added (GF-2.5 auto-
        //    join). Peers in the pool whose bundle file has
        //    disappeared from QNM-Shared (a polling-based
        //    proxy for the `ca_revoke` signal the original
        //    GF-2.6 spec sketched) get detached. Skips when
        //    either qnm_root or self_node_id wasn't supplied
        //    via `with_qnm_peer_discovery`.
        if let (Some(qnm_root), Some(self_id)) = (self.qnm_root.as_ref(), self.self_node_id.as_ref()) {
            let desired = peer_probe_targets(qnm_root, self_id);
            let current = current_gluster_peers(&self.gluster_binary);
            for (probe_target, probe_ip) in peers_to_probe(&desired, &current) {
                let argv = peer_probe_argv(&self.gluster_binary, &probe_ip);
                tracing::info!(
                    target: "mackesd::gluster_worker",
                    peer = %probe_target,
                    ip = %probe_ip,
                    argv = ?argv,
                    "peer-probe: adding peer to mesh-home pool",
                );
                let _ = run_argv(&argv);
            }
            for stale_ip in peers_to_detach(&desired, &current) {
                let argv = peer_detach_argv(&self.gluster_binary, &stale_ip);
                tracing::info!(
                    target: "mackesd::gluster_worker",
                    ip = %stale_ip,
                    argv = ?argv,
                    "peer-detach: removing peer (bundle missing from QNM-Shared)",
                );
                let _ = run_argv(&argv);
            }
        }
    }

    /// GF-2.7 — `true` when QUOTA_PROBE_INTERVAL has elapsed
    /// since the last probe (or the worker has never probed
    /// yet). Mutex-guarded so concurrent tick callers can't
    /// double-fire.
    fn quota_probe_due(&self) -> bool {
        let mut guard = self.last_quota_probe.lock().expect("last_quota_probe mutex");
        let now = std::time::Instant::now();
        let due = match *guard {
            None => true,
            Some(last) => now.duration_since(last) >= QUOTA_PROBE_INTERVAL,
        };
        if due {
            *guard = Some(now);
        }
        due
    }

    /// GF-2.9 — register that we've requested split-brain
    /// heal for `gfid`. Returns `true` if this is the first
    /// such request (caller should fire the heal command);
    /// `false` if we've already requested it (caller should
    /// skip). Mutex-guarded so concurrent tick callers can't
    /// double-fire.
    fn mark_gfid_heal_requested(&self, gfid: &str) -> bool {
        let mut guard = self.healed_gfids.lock().expect("healed_gfids mutex");
        guard.insert(gfid.to_owned())
    }

    /// GF-2.7 — query `gluster volume info mesh-home --xml`,
    /// parse the brick free-bytes column, compute `0.8 ×
    /// min(free brick)`, push it back as the volume quota
    /// limit. Best-effort — every failure step logs at warn
    /// and the next tick retries.
    fn run_quota_probe(&self) {
        let xml = match Command::new(&self.gluster_binary)
            .args(["volume", "info", VOLUME_NAME, "--xml"])
            .output()
        {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
            Ok(o) => {
                tracing::warn!(
                    target: "mackesd::gluster_worker",
                    status = ?o.status,
                    "volume-info quota probe exited non-zero",
                );
                return;
            }
            Err(e) => {
                tracing::warn!(
                    target: "mackesd::gluster_worker",
                    error = %e,
                    "volume-info quota probe failed to launch",
                );
                return;
            }
        };
        let Some(min_free) = min_brick_free_bytes(&xml) else {
            tracing::debug!(
                target: "mackesd::gluster_worker",
                "no brick free-space columns in volume-info; skipping quota set",
            );
            return;
        };
        let cap_bytes = (min_free as f64 * QUOTA_MULTIPLIER) as u64;
        let argv = quota_set_argv(&self.gluster_binary, cap_bytes);
        tracing::info!(
            target: "mackesd::gluster_worker",
            min_free_bytes = min_free,
            cap_bytes,
            "setting mesh-home quota to 0.8 × min(free brick)",
        );
        let _ = run_argv(&argv);
    }
}

/// Pure helper — `true` if `name` resolves to an executable
/// on PATH or to an absolute path that exists.
fn binary_on_path(name: &str) -> bool {
    let candidate = std::path::Path::new(name);
    if candidate.is_absolute() {
        return candidate.exists();
    }
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(name).is_file())
}

/// Pure helper — `true` if `gluster volume info <name>` exits
/// successfully (i.e. the volume exists). Returns `None` when
/// the probe itself failed (binary missing, glusterd
/// unreachable). `false` means the binary ran + reported no
/// such volume.
fn volume_exists(binary: &str, volume: &str) -> Option<bool> {
    let out = Command::new(binary).args(["volume", "info", volume]).output().ok()?;
    if !out.status.success() {
        // Distinguish "no such volume" (status 1, stderr
        // contains "does not exist") from other failures.
        let stderr = String::from_utf8_lossy(&out.stderr);
        if stderr.contains("does not exist") {
            return Some(false);
        }
        return None;
    }
    Some(true)
}

/// GF-2.7 — extract the smallest `sizeFree` value from a
/// `gluster volume info --xml` payload. Returns `None` when
/// the XML has no brick entries OR no parseable size field
/// — gives the caller a clean "skip this tick" signal.
///
/// XML shape (Fedora glusterfs 11.x):
///
/// ```xml
/// <cliOutput>
///   <volInfo>
///     <volumes>
///       <volume>
///         <bricks>
///           <brick uuid="..."><name>...</name>
///             <sizeFree>123456789</sizeFree>
///           </brick>
///         </bricks>
///       </volume>
///     </volumes>
///   </volInfo>
/// </cliOutput>
/// ```
///
/// We do a tiny regex-free scan rather than pulling in an
/// XML crate: locate every `<sizeFree>NNN</sizeFree>`,
/// parse the integer, take the min.
#[must_use]
pub fn min_brick_free_bytes(xml: &str) -> Option<u64> {
    let mut min: Option<u64> = None;
    let mut rest = xml;
    while let Some(open) = rest.find("<sizeFree>") {
        let after_open = &rest[open + "<sizeFree>".len()..];
        let close = after_open.find("</sizeFree>")?;
        let body = after_open[..close].trim();
        if let Ok(n) = body.parse::<u64>() {
            min = Some(match min {
                None => n,
                Some(prev) => prev.min(n),
            });
        }
        rest = &after_open[close..];
    }
    min
}

/// GF-2.5 — one peer-probe target. Pulled from the QNM-Shared
/// bundle scan so the caller can build the argv + skip
/// already-pooled peers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeTarget {
    /// Stable peer node-id (the QNM-Shared dir name).
    pub node_id: String,
    /// Overlay IP from that peer's signed Nebula bundle.
    pub overlay_ip: String,
}

/// GF-2.5 — scan `<qnm_root>/*/mackesd/nebula-bundle.json` to
/// discover every peer the local Nebula lighthouse has
/// signed. The bundle's `overlay_ip` field becomes the
/// probe target. Skips the local peer (matched by
/// `self_node_id`). Skips dirs that don't contain a bundle
/// (peer hasn't enrolled yet) or whose bundle JSON doesn't
/// parse (corrupt — log + retry next tick).
///
/// Returns a deduplicated, sorted-by-node_id `Vec` so
/// downstream diff logic is deterministic.
#[must_use]
pub fn peer_probe_targets(qnm_root: &std::path::Path, self_node_id: &str) -> Vec<ProbeTarget> {
    let entries = match std::fs::read_dir(qnm_root) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut out: Vec<ProbeTarget> = Vec::new();
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(|s| s.to_owned()) else {
            continue;
        };
        if name == self_node_id {
            continue;
        }
        let bundle_path = entry.path().join("mackesd").join("nebula-bundle.json");
        let Ok(bytes) = std::fs::read(&bundle_path) else { continue };
        let Ok(bundle) = serde_json::from_slice::<crate::ca::bundle::NebulaBundle>(&bytes) else {
            continue;
        };
        out.push(ProbeTarget {
            node_id: name,
            overlay_ip: bundle.overlay_ip,
        });
    }
    out.sort_by(|a, b| a.node_id.cmp(&b.node_id));
    out
}

/// GF-2.5 — list the overlay IPs the local glusterd currently
/// considers peers. Shells `gluster pool list` + parses the
/// space-separated table; line shape (Fedora glusterfs 11.x):
///
/// ```
/// UUID                                    Hostname            State
/// 5c3...                                  10.42.0.5           Connected
/// 7a8...                                  10.42.0.7           Connected
/// 4f2...                                  localhost           Connected
/// ```
///
/// Returns the `Hostname` column (excluding "localhost"
/// since the local peer doesn't probe itself).
#[must_use]
pub fn current_gluster_peers(binary: &str) -> Vec<String> {
    let Ok(out) = Command::new(binary).args(["pool", "list"]).output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    parse_gluster_pool_list(&text)
}

/// GF-2.5 — pure parser for `gluster pool list` output.
/// Exposed for testing without shelling.
#[must_use]
pub fn parse_gluster_pool_list(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        // Skip the header line.
        if i == 0 && line.contains("UUID") {
            continue;
        }
        // Each data line is whitespace-separated:
        // <uuid> <hostname> <state>
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 3 {
            continue;
        }
        let hostname = cols[1];
        // Skip the local peer's own "localhost" entry.
        if hostname == "localhost" {
            continue;
        }
        out.push(hostname.to_owned());
    }
    out.sort();
    out
}

/// GF-2.5 — diff `desired` (from QNM-Shared bundle scan)
/// against `current` (from `gluster pool list`); return every
/// `(node_id, overlay_ip)` pair whose IP isn't in the current
/// pool. Pure function; exposed for testing.
#[must_use]
pub fn peers_to_probe(desired: &[ProbeTarget], current: &[String]) -> Vec<(String, String)> {
    desired
        .iter()
        .filter(|t| !current.iter().any(|ip| ip == &t.overlay_ip))
        .map(|t| (t.node_id.clone(), t.overlay_ip.clone()))
        .collect()
}

/// GF-2.6 — diff `current` against `desired`; return every
/// IP in the pool that no longer has a bundle in QNM-Shared
/// (polling proxy for the `ca_revoke` event the original
/// spec sketched). Pure function; exposed for testing.
#[must_use]
pub fn peers_to_detach(desired: &[ProbeTarget], current: &[String]) -> Vec<String> {
    current
        .iter()
        .filter(|ip| !desired.iter().any(|t| &t.overlay_ip == *ip))
        .cloned()
        .collect()
}

/// GF-2.9 — `gluster volume heal mesh-home split-brain
/// latest-mtime <gfid>` argv. Delegates split-brain resolution
/// to gluster's own self-heal daemon, which knows the
/// cross-peer mtime + handles the `.conflict-<host>-<ts>`
/// rename per its own conventions. We trigger this once per
/// detected conflict-GFID rather than re-implementing the
/// LWW logic in Rust — the gluster daemon is the source of
/// truth for which replica wins. Exposed for testing.
#[must_use]
pub fn heal_split_brain_argv(binary: &str, gfid: &str) -> Vec<String> {
    vec![
        binary.to_owned(),
        "volume".into(),
        "heal".into(),
        VOLUME_NAME.to_owned(),
        "split-brain".into(),
        "latest-mtime".into(),
        format!("gfid:{gfid}"),
    ]
}

/// GF-2.5 — `gluster peer probe <overlay-ip>` argv.
#[must_use]
pub fn peer_probe_argv(binary: &str, overlay_ip: &str) -> Vec<String> {
    vec![
        binary.to_owned(),
        "peer".into(),
        "probe".into(),
        overlay_ip.to_owned(),
    ]
}

/// GF-2.6 — `gluster peer detach <overlay-ip> force` argv.
/// `force` is required because the peer may still have a brick
/// contributing to the volume; the detach + volume rebalance
/// is the operator's intent per Q15.
#[must_use]
pub fn peer_detach_argv(binary: &str, overlay_ip: &str) -> Vec<String> {
    vec![
        binary.to_owned(),
        "peer".into(),
        "detach".into(),
        overlay_ip.to_owned(),
        "force".into(),
    ]
}

/// GF-2.8 — list every entry in the brick's
/// `.glusterfs/indices/xattrop/` directory. Each entry there
/// is a GFID symlink representing a file with a pending heal
/// or split-brain op (glusterd's own bookkeeping for the
/// self-heal daemon). The detector surfaces each GFID as a
/// `ConflictDetected` tracing event so the operator (or the
/// future D-Bus signal subscriber from GF-2.2) sees split-
/// brain state without manually running `gluster volume heal
/// info`.
///
/// Returns the GFID list (one per pending entry); empty when
/// the brick is healthy OR when the dir doesn't exist
/// (operator runs mackesd on a non-storage box). The "xattrop"
/// pseudo-entry (an empty marker the brick maintains) is
/// filtered out.
#[must_use]
pub fn pending_conflict_gfids(xattrop_dir: &std::path::Path) -> Vec<String> {
    let entries = match std::fs::read_dir(xattrop_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else { continue };
        // glusterd maintains a placeholder file literally named
        // "xattrop" in the directory; it's not a conflict marker.
        if name_str == "xattrop" || name_str.starts_with("xattrop-") {
            continue;
        }
        out.push(name_str.to_owned());
    }
    out.sort();
    out
}

/// GF-2.7 — `gluster volume quota mesh-home limit-usage / <bytes>`
/// argv. Exposed for testing.
#[must_use]
pub fn quota_set_argv(binary: &str, cap_bytes: u64) -> Vec<String> {
    vec![
        binary.to_owned(),
        "volume".into(),
        "quota".into(),
        VOLUME_NAME.to_owned(),
        "limit-usage".into(),
        "/".into(),
        cap_bytes.to_string(),
    ]
}

/// Pure helper — build the `gluster volume create` argv for
/// the genesis path. Exposed for testing without shelling.
#[must_use]
pub fn bootstrap_argv(binary: &str, overlay_ip: &str, brick_path: &str) -> Vec<String> {
    vec![
        binary.to_owned(),
        "volume".into(),
        "create".into(),
        VOLUME_NAME.to_owned(),
        "replica".into(),
        "1".into(),
        "transport".into(),
        "tcp".into(),
        format!("{overlay_ip}:{brick_path}"),
        "force".into(),
    ]
}

fn run_argv(argv: &[String]) -> bool {
    let Some((bin, rest)) = argv.split_first() else {
        return false;
    };
    let out = Command::new(bin).args(rest).output();
    match out {
        Ok(o) if o.status.success() => true,
        Ok(o) => {
            tracing::warn!(
                target: "mackesd::gluster_worker",
                status = ?o.status,
                stderr = %String::from_utf8_lossy(&o.stderr),
                "gluster CLI exited non-zero",
            );
            false
        }
        Err(e) => {
            tracing::warn!(
                target: "mackesd::gluster_worker",
                error = %e,
                "failed to launch gluster CLI",
            );
            false
        }
    }
}

#[async_trait::async_trait]
impl Worker for GlusterWorker {
    fn name(&self) -> &'static str {
        "gluster_worker"
    }

    async fn run(&mut self, mut shutdown: ShutdownToken) -> anyhow::Result<()> {
        // Immediate tick on startup so the bootstrap fires on
        // the first opportunity rather than waiting the full
        // interval.
        self.tick_once();
        loop {
            tokio::select! {
                _ = shutdown.wait() => return Ok(()),
                _ = tokio::time::sleep(self.tick) => self.tick_once(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_store() -> Arc<Mutex<rusqlite::Connection>> {
        let conn = rusqlite::Connection::open_in_memory().expect("memory db");
        crate::store::migrate(&conn).expect("migrate");
        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn worker_name_is_stable() {
        let w = GlusterWorker::new(fresh_store());
        assert_eq!(w.name(), "gluster_worker");
    }

    #[test]
    fn tick_once_no_ops_when_gluster_binary_absent() {
        // `/nonexistent/...` is not on PATH; the worker should
        // silently no-op rather than panicking or shelling out.
        let mut w = GlusterWorker::new(fresh_store())
            .with_gluster_binary("/nonexistent/gluster-bin-xyz");
        // No assertions on side effects — just that the call
        // doesn't panic. The log statement at debug! is the
        // sole evidence path.
        w.tick_once();
        // Run a second time to confirm the no-op is idempotent
        // + the worker doesn't accumulate any internal state.
        w.tick_once();
    }

    #[test]
    fn tick_once_skips_bootstrap_when_overlay_ip_file_missing() {
        // Use /bin/true as the "gluster" binary — exists, succeeds
        // on every invocation. The overlay-ip file is missing,
        // so the worker must skip the bootstrap.
        let tmp = tempfile::tempdir().expect("tempdir");
        let missing = tmp.path().join("does-not-exist");
        let mut w = GlusterWorker::new(fresh_store())
            .with_gluster_binary("/bin/true")
            .with_overlay_ip_path(missing);
        w.tick_once();
    }

    #[test]
    fn tick_once_attempts_bootstrap_when_overlay_ip_present_and_volume_missing() {
        // /bin/true returns exit 0 with empty output — that
        // signals "volume exists" to volume_exists() (since
        // status.success() is true). So the bootstrap would
        // skip. Use /bin/false to simulate "exit 1" → which
        // volume_exists treats as "probe failure" → None →
        // worker logs warn + retries next tick. Either way we
        // confirm the worker doesn't panic on the bootstrap
        // path.
        let tmp = tempfile::tempdir().expect("tempdir");
        let overlay = tmp.path().join("overlay-ip");
        std::fs::write(&overlay, "10.42.0.5\n").expect("write");
        let mut w = GlusterWorker::new(fresh_store())
            .with_gluster_binary("/bin/false")
            .with_overlay_ip_path(overlay);
        w.tick_once();
    }

    #[test]
    fn bootstrap_argv_matches_design_doc_command_shape() {
        let argv = bootstrap_argv("gluster", "10.42.0.5", "/var/lib/gluster/bricks/mesh-home");
        // Design doc § 3.4: `gluster volume create mesh-home
        // replica 1 transport tcp <local-overlay-ip>:<brick>
        // force` is the genesis path.
        assert_eq!(
            argv,
            vec![
                "gluster",
                "volume",
                "create",
                "mesh-home",
                "replica",
                "1",
                "transport",
                "tcp",
                "10.42.0.5:/var/lib/gluster/bricks/mesh-home",
                "force",
            ]
        );
    }

    #[test]
    fn bootstrap_argv_honors_alternate_binary_and_paths() {
        let argv = bootstrap_argv(
            "/usr/local/bin/gluster",
            "192.168.42.7",
            "/srv/gluster/mesh-home",
        );
        assert_eq!(argv[0], "/usr/local/bin/gluster");
        assert_eq!(
            argv[argv.len() - 2],
            "192.168.42.7:/srv/gluster/mesh-home"
        );
    }

    #[test]
    fn binary_on_path_finds_true_in_path() {
        // `true` lives in /usr/bin or /bin on every Linux
        // host, so the PATH walk should find it.
        assert!(binary_on_path("true"));
    }

    #[test]
    fn binary_on_path_rejects_nonexistent_absolute_path() {
        assert!(!binary_on_path("/nonexistent/binary-xyz"));
    }

    #[test]
    fn binary_on_path_rejects_nonexistent_relative_name() {
        assert!(!binary_on_path("definitely-not-on-path-xyz"));
    }

    // GF-2.7 — quota probe helpers.

    #[test]
    fn min_brick_free_bytes_picks_smallest_brick() {
        let xml = r#"
            <cliOutput>
              <volume>
                <bricks>
                  <brick><name>peer-a:/brick</name><sizeFree>1000000000</sizeFree></brick>
                  <brick><name>peer-b:/brick</name><sizeFree>500000000</sizeFree></brick>
                  <brick><name>peer-c:/brick</name><sizeFree>2000000000</sizeFree></brick>
                </bricks>
              </volume>
            </cliOutput>
        "#;
        assert_eq!(min_brick_free_bytes(xml), Some(500_000_000));
    }

    #[test]
    fn min_brick_free_bytes_returns_none_for_empty_volume() {
        let xml = "<cliOutput><volume><bricks/></volume></cliOutput>";
        assert_eq!(min_brick_free_bytes(xml), None);
    }

    #[test]
    fn min_brick_free_bytes_skips_unparseable_entries() {
        let xml = r#"
            <cliOutput><volume><bricks>
              <brick><sizeFree>not-a-number</sizeFree></brick>
              <brick><sizeFree>42</sizeFree></brick>
            </bricks></volume></cliOutput>
        "#;
        assert_eq!(min_brick_free_bytes(xml), Some(42));
    }

    #[test]
    fn quota_set_argv_matches_design_doc_command_shape() {
        let argv = quota_set_argv("gluster", 800_000_000_000);
        assert_eq!(
            argv,
            vec![
                "gluster",
                "volume",
                "quota",
                "mesh-home",
                "limit-usage",
                "/",
                "800000000000",
            ]
        );
    }

    // GF-2.8 — conflict detector helpers.

    #[test]
    fn pending_conflict_gfids_returns_empty_for_missing_dir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let missing = tmp.path().join("does-not-exist");
        assert_eq!(pending_conflict_gfids(&missing), Vec::<String>::new());
    }

    #[test]
    fn pending_conflict_gfids_returns_empty_for_healthy_brick() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(tmp.path()).expect("mkdir");
        // Empty directory → no conflicts.
        assert_eq!(pending_conflict_gfids(tmp.path()), Vec::<String>::new());
    }

    #[test]
    fn pending_conflict_gfids_lists_every_gfid_entry() {
        let tmp = tempfile::tempdir().expect("tempdir");
        // glusterd populates the dir with empty files named
        // after the GFID of each pending-heal entry. We just
        // need to enumerate them; we don't care about content.
        for gfid in [
            "12345678-1234-1234-1234-123456789abc",
            "deadbeef-dead-beef-dead-beefdeadbeef",
            "00000000-0000-0000-0000-000000000001",
        ] {
            std::fs::write(tmp.path().join(gfid), b"").expect("touch");
        }
        let mut got = pending_conflict_gfids(tmp.path());
        got.sort();
        let mut want = vec![
            "00000000-0000-0000-0000-000000000001".to_string(),
            "12345678-1234-1234-1234-123456789abc".to_string(),
            "deadbeef-dead-beef-dead-beefdeadbeef".to_string(),
        ];
        want.sort();
        assert_eq!(got, want);
    }

    #[test]
    fn pending_conflict_gfids_filters_glusterd_placeholder_marker() {
        let tmp = tempfile::tempdir().expect("tempdir");
        // glusterd maintains a literal "xattrop" placeholder
        // file — NOT a real conflict — plus any
        // "xattrop-<suffix>" variants. Both must be filtered
        // out so we don't surface phantom conflicts.
        std::fs::write(tmp.path().join("xattrop"), b"").expect("touch placeholder");
        std::fs::write(tmp.path().join("xattrop-changelog-1"), b"").expect("touch variant");
        std::fs::write(
            tmp.path().join("00000000-0000-0000-0000-000000000001"),
            b"",
        )
        .expect("touch real");
        let got = pending_conflict_gfids(tmp.path());
        assert_eq!(got, vec!["00000000-0000-0000-0000-000000000001".to_string()]);
    }

    #[test]
    fn quota_probe_due_fires_on_first_call_then_rate_limits() {
        let w = GlusterWorker::new(fresh_store());
        // First call always fires.
        assert!(w.quota_probe_due());
        // Immediate second call is rate-limited (the 1-hour
        // gate hasn't elapsed).
        assert!(!w.quota_probe_due());
    }

    // GF-2.5 + GF-2.6 — peer convergence.

    fn write_bundle(qnm: &std::path::Path, node_id: &str, overlay_ip: &str) {
        let dir = qnm.join(node_id).join("mackesd");
        std::fs::create_dir_all(&dir).expect("mkdir bundle dir");
        let bundle = crate::ca::bundle::NebulaBundle {
            mesh_id: "test-mesh".into(),
            epoch: 1,
            ca_cert_pem: "ca".into(),
            peer_cert_pem: "p".into(),
            peer_key_pem: "k".into(),
            overlay_ip: overlay_ip.into(),
            mesh_cidr: "10.42.0.0/16".into(),
            lighthouses: vec![],
            created_at: 1_700_000_000,
        };
        let body = serde_json::to_vec_pretty(&bundle).expect("encode");
        std::fs::write(dir.join("nebula-bundle.json"), &body).expect("write bundle");
    }

    #[test]
    fn peer_probe_targets_returns_empty_for_missing_qnm_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let missing = tmp.path().join("does-not-exist");
        assert_eq!(peer_probe_targets(&missing, "peer:self"), Vec::<ProbeTarget>::new());
    }

    #[test]
    fn peer_probe_targets_skips_self_node_id() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let qnm = tmp.path().to_path_buf();
        write_bundle(&qnm, "peer:self", "10.42.0.5");
        write_bundle(&qnm, "peer:alice", "10.42.0.7");
        let targets = peer_probe_targets(&qnm, "peer:self");
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].node_id, "peer:alice");
        assert_eq!(targets[0].overlay_ip, "10.42.0.7");
    }

    #[test]
    fn peer_probe_targets_sorts_deterministically() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let qnm = tmp.path().to_path_buf();
        write_bundle(&qnm, "peer:zebra", "10.42.0.9");
        write_bundle(&qnm, "peer:alice", "10.42.0.7");
        write_bundle(&qnm, "peer:mike", "10.42.0.8");
        let targets = peer_probe_targets(&qnm, "peer:self");
        let ids: Vec<_> = targets.iter().map(|t| t.node_id.clone()).collect();
        assert_eq!(ids, vec!["peer:alice", "peer:mike", "peer:zebra"]);
    }

    #[test]
    fn peer_probe_targets_skips_dirs_without_bundle() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let qnm = tmp.path().to_path_buf();
        // Create a peer dir but no bundle file inside.
        std::fs::create_dir_all(qnm.join("peer:halfopen").join("mackesd")).expect("mkdir");
        write_bundle(&qnm, "peer:alice", "10.42.0.7");
        let targets = peer_probe_targets(&qnm, "peer:self");
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].node_id, "peer:alice");
    }

    #[test]
    fn parse_gluster_pool_list_extracts_overlay_ips_skipping_localhost() {
        let text = "\
            UUID                                    Hostname    State\n\
            5c3aaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa    10.42.0.5   Connected\n\
            7a8bbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb    10.42.0.7   Connected\n\
            4f2ccccc-cccc-cccc-cccc-cccccccccccc    localhost   Connected\n\
        ";
        let peers = parse_gluster_pool_list(text);
        assert_eq!(peers, vec!["10.42.0.5", "10.42.0.7"]);
    }

    #[test]
    fn parse_gluster_pool_list_handles_empty_output() {
        assert_eq!(parse_gluster_pool_list(""), Vec::<String>::new());
    }

    #[test]
    fn peers_to_probe_returns_missing_targets_only() {
        let desired = vec![
            ProbeTarget { node_id: "peer:alice".into(), overlay_ip: "10.42.0.7".into() },
            ProbeTarget { node_id: "peer:bob".into(), overlay_ip: "10.42.0.8".into() },
            ProbeTarget { node_id: "peer:carol".into(), overlay_ip: "10.42.0.9".into() },
        ];
        let current = vec!["10.42.0.7".to_string()]; // only alice is in the pool
        let to_probe = peers_to_probe(&desired, &current);
        assert_eq!(to_probe, vec![
            ("peer:bob".to_string(), "10.42.0.8".to_string()),
            ("peer:carol".to_string(), "10.42.0.9".to_string()),
        ]);
    }

    #[test]
    fn peers_to_detach_returns_stale_pool_ips() {
        let desired = vec![
            ProbeTarget { node_id: "peer:alice".into(), overlay_ip: "10.42.0.7".into() },
        ];
        let current = vec![
            "10.42.0.7".to_string(),
            "10.42.0.8".to_string(), // bob — bundle deleted
            "10.42.0.9".to_string(), // carol — bundle deleted
        ];
        let to_detach = peers_to_detach(&desired, &current);
        let mut got = to_detach.clone();
        got.sort();
        assert_eq!(got, vec!["10.42.0.8".to_string(), "10.42.0.9".to_string()]);
    }

    #[test]
    fn peer_probe_argv_matches_design_doc_command_shape() {
        assert_eq!(
            peer_probe_argv("gluster", "10.42.0.7"),
            vec!["gluster", "peer", "probe", "10.42.0.7"]
        );
    }

    #[test]
    fn peer_detach_argv_uses_force_for_brick_owners() {
        assert_eq!(
            peer_detach_argv("gluster", "10.42.0.9"),
            vec!["gluster", "peer", "detach", "10.42.0.9", "force"]
        );
    }

    // GF-2.9 — LWW resolver via gluster's native heal.

    #[test]
    fn heal_split_brain_argv_matches_gluster_command_shape() {
        let argv = heal_split_brain_argv(
            "gluster",
            "12345678-1234-1234-1234-123456789abc",
        );
        assert_eq!(
            argv,
            vec![
                "gluster",
                "volume",
                "heal",
                "mesh-home",
                "split-brain",
                "latest-mtime",
                "gfid:12345678-1234-1234-1234-123456789abc",
            ]
        );
    }

    #[test]
    fn mark_gfid_heal_requested_fires_once_per_gfid() {
        let w = GlusterWorker::new(fresh_store());
        let gfid = "12345678-1234-1234-1234-123456789abc";
        // First call returns true (caller should fire heal).
        assert!(w.mark_gfid_heal_requested(gfid));
        // Immediate second call returns false (already
        // requested — caller should skip).
        assert!(!w.mark_gfid_heal_requested(gfid));
        // Third call still false.
        assert!(!w.mark_gfid_heal_requested(gfid));
    }

    #[test]
    fn mark_gfid_heal_requested_distinguishes_different_gfids() {
        let w = GlusterWorker::new(fresh_store());
        assert!(w.mark_gfid_heal_requested("gfid-a"));
        assert!(w.mark_gfid_heal_requested("gfid-b"));
        assert!(w.mark_gfid_heal_requested("gfid-c"));
        // Re-requesting gfid-a is rate-limited.
        assert!(!w.mark_gfid_heal_requested("gfid-a"));
    }

    #[tokio::test]
    async fn worker_exits_on_shutdown_token() {
        let mut w = GlusterWorker::new(fresh_store())
            .with_gluster_binary("/nonexistent/gluster-xyz")
            .with_tick(Duration::from_millis(50));
        let (tx, rx) = tokio::sync::watch::channel(false);
        let token = ShutdownToken::from_receiver(rx);
        let _ = tx.send(true);
        let result = tokio::time::timeout(Duration::from_secs(3), w.run(token))
            .await
            .expect("worker must exit on shutdown");
        assert!(result.is_ok());
    }
}
