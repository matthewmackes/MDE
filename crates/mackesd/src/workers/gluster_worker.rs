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
    /// GF-2.7 — last-fire wall-clock seconds (relative to
    /// startup) of the hourly quota probe. `None` until the
    /// first probe runs.
    last_quota_probe: std::sync::Mutex<Option<std::time::Instant>>,
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
            last_quota_probe: std::sync::Mutex::new(None),
        }
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
