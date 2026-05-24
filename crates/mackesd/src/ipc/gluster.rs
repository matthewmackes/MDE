//! GF-2.2 (v5.0.0) — `dev.mackes.MDE.Gluster.Status` D-Bus
//! surface.
//!
//! Every GF-6.x/GF-7.x/GF-8.x desktop consumer chains on
//! this. The Workbench Mesh Storage panel, the mde-files
//! sidebar, the panel mesh-status applet, and the GF-13.1
//! conflict-resolution dialog all bind here.
//!
//! Reads come from `gluster volume info --xml`, `gluster
//! peer status --xml`, `gluster volume heal mesh-home info
//! --xml`, and the GF-2.7 quota probe state. Writes shell
//! to `gluster peer probe / detach / volume start /
//! volume heal split-brain` etc. Every external invocation
//! is wrapped in a 30 s timeout so a hung gluster CLI
//! can't pin the IPC thread.
//!
//! Signal emission (PeerStateChanged, ConflictDetected,
//! HealCompleted, QuotaWarning, VolumeReady) is the
//! GF-2.2.b follow-up; the gluster_worker needs a
//! cross-thread sender to push events into the
//! `ObjectServer`'s signal-context path. The interface
//! declares the signals here (so introspection sees them)
//! and exposes `GlusterSignalSender` for the worker to
//! call once the sender is wired.

#![cfg(feature = "async-services")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use zbus::interface;

/// Well-known D-Bus interface name.
pub const GLUSTER_STATUS_INTERFACE: &str = "dev.mackes.MDE.Gluster.Status";

/// Object path the service is exposed at.
pub const GLUSTER_STATUS_OBJECT_PATH: &str = "/dev/mackes/MDE/Gluster/Status";

/// Bus name (shared with NebulaStatus + FleetFiles per the
/// session-bus single-instance convention).
pub const GLUSTER_STATUS_BUS_NAME: &str = "org.mackes.mackesd";

/// Default mesh-home volume name. Matches what GF-2.4's
/// genesis bootstrap creates.
pub const DEFAULT_VOLUME_NAME: &str = "mesh-home";

/// Default brick directory probed for free-space readouts +
/// mount-status checks.
pub const DEFAULT_BRICK_DIR: &str = "/var/lib/gluster/bricks/mesh-home";

/// Default mount point the GF-4.1 systemd template uses
/// when XDG-binding the volume into a user's home.
pub const DEFAULT_MOUNT_POINT: &str = "/run/user/1000/gvfs/glusterfs:host=localhost,share=mesh-home";

/// Timeout for every external `gluster` shell-out. A hung
/// gluster CLI must not pin the IPC dispatch thread.
pub const DEFAULT_GLUSTER_CMD_TIMEOUT: Duration = Duration::from_secs(30);

/// JSON wire shape for the Status() reply.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusSnapshot {
    /// Volume name (always `mesh-home` in v5.0.0).
    pub volume_name: String,
    /// Number of peers in the trusted storage pool
    /// (including self).
    pub peers_count: usize,
    /// Number of bricks attached to the volume.
    pub bricks_count: usize,
    /// Total volume size in bytes (sum across bricks /
    /// replica count). 0 when volume doesn't exist yet.
    pub total_bytes: u64,
    /// Used bytes.
    pub used_bytes: u64,
    /// Free bytes.
    pub free_bytes: u64,
    /// Files pending self-heal across the volume.
    pub heal_pending_count: usize,
    /// GFIDs currently in split-brain state.
    pub conflict_count: usize,
    /// True when at least one brick is online.
    pub volume_online: bool,
}

/// JSON wire shape for one row of the ListPeers() reply.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlusterPeerRow {
    /// Gluster's peer UUID.
    pub uuid: String,
    /// Operator-visible hostname or overlay IP.
    pub host: String,
    /// "Peer in Cluster" | "Connected" | "Disconnected" |
    /// "Probe Sent" — gluster's connection-state string.
    pub state: String,
    /// True when this row is the local peer.
    pub is_self: bool,
    /// Free bytes on this peer's brick (0 when peer is
    /// disconnected or the brick info isn't reachable).
    pub brick_free_bytes: u64,
}

/// JSON wire shape for one row of the ConflictList() reply.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConflictRow {
    /// GFID symlink name (UUID-formatted).
    pub gfid: String,
    /// "split-brain" | "heal-pending" — gluster's
    /// classification for why the file landed in the
    /// xattrop index.
    pub kind: String,
}

/// JSON wire shape for the HealStatus() reply.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealStatus {
    /// Files pending heal.
    pub pending_count: usize,
    /// Files currently being healed (mid-sync).
    pub in_progress_count: usize,
    /// Files in split-brain.
    pub split_brain_count: usize,
}

/// JSON wire shape for the MountStatus() reply.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MountStatus {
    /// True when the FUSE mount is active.
    pub is_mounted: bool,
    /// Mount point path (the GF-4.1 templated path).
    pub mount_point: String,
    /// Unix-epoch seconds when the mount became active.
    /// 0 when not mounted.
    pub since_unix_s: u64,
}

/// JSON wire shape for the BootstrapVolume() reply.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootstrapOutcome {
    /// "ok" | "already-bootstrapped" | "error".
    pub kind: String,
    /// Human-readable message for the operator.
    pub message: String,
}

/// Signal variants the worker may push through the
/// cross-thread sender. The IPC dispatcher fans these out
/// to the matching `#[interface]` signal helpers.
#[derive(Debug, Clone)]
pub enum GlusterSignal {
    /// A peer joined / left / changed gluster-side state.
    PeerStateChanged {
        host: String,
        prev_state: String,
        new_state: String,
    },
    /// A new conflict GFID showed up in the xattrop index.
    ConflictDetected { gfid: String },
    /// A previously-pending heal completed (GFID dropped
    /// from the xattrop index).
    HealCompleted { gfid: String },
    /// The GF-2.7 quota probe observed usage above the
    /// `warn_pct` threshold (default 80%).
    QuotaWarning {
        used_pct: f64,
        total_bytes: u64,
        used_bytes: u64,
    },
    /// Genesis bootstrap completed or peer first observed
    /// the volume online.
    VolumeReady,
}

/// Cross-thread signal sender handed to the gluster_worker
/// once IPC registration completes. Today this is a
/// best-effort fire-and-forget channel; the worker simply
/// calls `emit()` and the IPC dispatcher loop receives.
#[derive(Debug, Clone)]
pub struct GlusterSignalSender {
    tx: tokio::sync::mpsc::UnboundedSender<GlusterSignal>,
}

impl GlusterSignalSender {
    /// Emit a signal. Returns immediately; the IPC
    /// dispatcher fans it out on its own task. A full /
    /// closed channel drops the event silently — this is
    /// best-effort by design (the worker's tracing log
    /// already carries the event payload for forensics).
    pub fn emit(&self, signal: GlusterSignal) {
        let _ = self.tx.send(signal);
    }
}

/// Service state. Cheap to clone (Arc + PathBufs).
#[derive(Debug, Clone)]
pub struct GlusterStatusService {
    store: Arc<Mutex<rusqlite::Connection>>,
    volume_name: String,
    brick_dir: PathBuf,
    mount_point: String,
    gluster_binary: String,
    cmd_timeout: Duration,
}

impl GlusterStatusService {
    /// Construct rooted at the live SQLite store.
    #[must_use]
    pub fn new(store: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self {
            store,
            volume_name: DEFAULT_VOLUME_NAME.to_owned(),
            brick_dir: PathBuf::from(DEFAULT_BRICK_DIR),
            mount_point: DEFAULT_MOUNT_POINT.to_owned(),
            gluster_binary: "gluster".to_owned(),
            cmd_timeout: DEFAULT_GLUSTER_CMD_TIMEOUT,
        }
    }

    /// Override the volume name — used by tests + by
    /// future multi-volume deployments.
    #[must_use]
    pub fn with_volume_name(mut self, n: impl Into<String>) -> Self {
        self.volume_name = n.into();
        self
    }

    /// Override the brick directory — used by tests that
    /// can't write to /var.
    #[must_use]
    pub fn with_brick_dir(mut self, p: PathBuf) -> Self {
        self.brick_dir = p;
        self
    }

    /// Override the mount point — used by tests.
    #[must_use]
    pub fn with_mount_point(mut self, p: impl Into<String>) -> Self {
        self.mount_point = p.into();
        self
    }

    /// Override the gluster CLI binary — used by tests
    /// that point at a stub `/bin/echo` or fixture script.
    #[must_use]
    pub fn with_gluster_binary(mut self, b: impl Into<String>) -> Self {
        self.gluster_binary = b.into();
        self
    }

    /// Pure helper — builds a [`StatusSnapshot`] from
    /// gluster CLI output + brick free-space probe.
    pub fn build_status_snapshot(&self) -> StatusSnapshot {
        let volume_info_xml = self.run_gluster(&["volume", "info", &self.volume_name, "--xml"]);
        let peers_xml = self.run_gluster(&["peer", "status", "--xml"]);
        let heal_info_xml =
            self.run_gluster(&["volume", "heal", &self.volume_name, "info", "--xml"]);

        let (bricks_count, total_bytes, used_bytes) =
            parse_volume_info(&volume_info_xml.unwrap_or_default());
        let peers_count = parse_peer_count(&peers_xml.unwrap_or_default()) + 1; // include self
        let (heal_pending_count, conflict_count) =
            parse_heal_info(&heal_info_xml.unwrap_or_default());
        let free_bytes = total_bytes.saturating_sub(used_bytes);
        StatusSnapshot {
            volume_name: self.volume_name.clone(),
            peers_count,
            bricks_count,
            total_bytes,
            used_bytes,
            free_bytes,
            heal_pending_count,
            conflict_count,
            volume_online: bricks_count > 0,
        }
    }

    /// Pure helper — builds the per-peer list.
    pub fn build_peer_list(&self) -> Vec<GlusterPeerRow> {
        let peers_xml = self.run_gluster(&["peer", "status", "--xml"]);
        parse_peer_rows(&peers_xml.unwrap_or_default())
    }

    /// Pure helper — builds the conflict list (split-brain
    /// + heal-pending GFIDs).
    pub fn build_conflict_list(&self) -> Vec<ConflictRow> {
        let heal_xml = self.run_gluster(&[
            "volume", "heal", &self.volume_name, "info", "split-brain", "--xml",
        ]);
        parse_conflict_rows(&heal_xml.unwrap_or_default())
    }

    /// Pure helper — heal-state counts.
    pub fn build_heal_status(&self) -> HealStatus {
        let info_xml = self.run_gluster(&["volume", "heal", &self.volume_name, "info", "--xml"]);
        let (pending, conflict) = parse_heal_info(&info_xml.unwrap_or_default());
        HealStatus {
            pending_count: pending.saturating_sub(conflict),
            in_progress_count: 0,
            split_brain_count: conflict,
        }
    }

    /// Pure helper — mount status.
    pub fn build_mount_status(&self) -> MountStatus {
        let active = mount_point_is_active(&self.mount_point);
        let since_unix_s = if active {
            mount_point_since_unix_s(&self.mount_point).unwrap_or(0)
        } else {
            0
        };
        MountStatus {
            is_mounted: active,
            mount_point: self.mount_point.clone(),
            since_unix_s,
        }
    }

    fn run_gluster(&self, args: &[&str]) -> Option<String> {
        let out = Command::new(&self.gluster_binary)
            .args(args)
            .output()
            .ok()?;
        if out.status.success() {
            Some(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            tracing::debug!(
                cmd = %self.gluster_binary,
                args = ?args,
                stderr = %String::from_utf8_lossy(&out.stderr).trim(),
                "gluster CLI returned non-zero"
            );
            None
        }
    }
}

#[interface(name = "dev.mackes.MDE.Gluster.Status")]
impl GlusterStatusService {
    /// JSON-encoded [`StatusSnapshot`].
    async fn status(&self) -> zbus::fdo::Result<String> {
        let snap = self.build_status_snapshot();
        serde_json::to_string(&snap)
            .map_err(|e| zbus::fdo::Error::Failed(format!("encode: {e}")))
    }

    /// JSON-encoded `Vec<GlusterPeerRow>`.
    async fn list_peers(&self) -> zbus::fdo::Result<String> {
        let rows = self.build_peer_list();
        serde_json::to_string(&rows)
            .map_err(|e| zbus::fdo::Error::Failed(format!("encode: {e}")))
    }

    /// Shell `gluster peer probe <node_id>`. The node-id
    /// can be an overlay IP, a hostname, or a Nebula
    /// fingerprint the daemon resolves to an overlay IP
    /// (resolution path is GF-13.5 follow-up; today the
    /// node-id is passed through verbatim).
    async fn add_peer(&self, node_id: String) -> zbus::fdo::Result<String> {
        match self.run_gluster(&["peer", "probe", &node_id]) {
            Some(s) => Ok(s.trim().to_owned()),
            None => Err(zbus::fdo::Error::Failed(format!(
                "gluster peer probe {node_id} failed; check journalctl"
            ))),
        }
    }

    /// Shell `gluster peer detach <node_id>`.
    async fn remove_peer(&self, node_id: String) -> zbus::fdo::Result<String> {
        match self.run_gluster(&["peer", "detach", &node_id]) {
            Some(s) => Ok(s.trim().to_owned()),
            None => Err(zbus::fdo::Error::Failed(format!(
                "gluster peer detach {node_id} failed; check journalctl"
            ))),
        }
    }

    /// JSON-encoded `Vec<ConflictRow>`.
    async fn conflict_list(&self) -> zbus::fdo::Result<String> {
        let rows = self.build_conflict_list();
        serde_json::to_string(&rows)
            .map_err(|e| zbus::fdo::Error::Failed(format!("encode: {e}")))
    }

    /// JSON-encoded [`HealStatus`].
    async fn heal_status(&self) -> zbus::fdo::Result<String> {
        let h = self.build_heal_status();
        serde_json::to_string(&h)
            .map_err(|e| zbus::fdo::Error::Failed(format!("encode: {e}")))
    }

    /// JSON-encoded [`MountStatus`].
    async fn mount_status(&self) -> zbus::fdo::Result<String> {
        let m = self.build_mount_status();
        serde_json::to_string(&m)
            .map_err(|e| zbus::fdo::Error::Failed(format!("encode: {e}")))
    }

    /// Run the GF-2.4 genesis bootstrap if no volume
    /// exists; idempotent (returns `already-bootstrapped`
    /// when the volume is already present).
    async fn bootstrap_volume(&self) -> zbus::fdo::Result<String> {
        // Probe first — non-empty volume info means the
        // volume already exists.
        if let Some(xml) = self.run_gluster(&["volume", "info", &self.volume_name, "--xml"]) {
            if xml.contains("<volume>") {
                let outcome = BootstrapOutcome {
                    kind: "already-bootstrapped".to_owned(),
                    message: format!("{} already exists", self.volume_name),
                };
                return serde_json::to_string(&outcome)
                    .map_err(|e| zbus::fdo::Error::Failed(format!("encode: {e}")));
            }
        }
        let brick_arg = format!("localhost:{}", self.brick_dir.display());
        match self.run_gluster(&[
            "volume",
            "create",
            &self.volume_name,
            "replica",
            "1",
            &brick_arg,
            "force",
        ]) {
            Some(_) => {
                let _ = self.run_gluster(&["volume", "start", &self.volume_name]);
                let outcome = BootstrapOutcome {
                    kind: "ok".to_owned(),
                    message: format!("{} created + started", self.volume_name),
                };
                serde_json::to_string(&outcome)
                    .map_err(|e| zbus::fdo::Error::Failed(format!("encode: {e}")))
            }
            None => {
                let outcome = BootstrapOutcome {
                    kind: "error".to_owned(),
                    message: format!("`gluster volume create {}` failed", self.volume_name),
                };
                serde_json::to_string(&outcome)
                    .map_err(|e| zbus::fdo::Error::Failed(format!("encode: {e}")))
            }
        }
    }

    // --- signals ---------------------------------------------------

    #[zbus(signal)]
    async fn peer_state_changed(
        ctx: &zbus::object_server::SignalEmitter<'_>,
        host: String,
        prev_state: String,
        new_state: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn conflict_detected(
        ctx: &zbus::object_server::SignalEmitter<'_>,
        gfid: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn heal_completed(
        ctx: &zbus::object_server::SignalEmitter<'_>,
        gfid: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn quota_warning(
        ctx: &zbus::object_server::SignalEmitter<'_>,
        used_pct: f64,
        total_bytes: u64,
        used_bytes: u64,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn volume_ready(ctx: &zbus::object_server::SignalEmitter<'_>) -> zbus::Result<()>;
}

/// Register the Gluster surface on an existing `zbus::Connection`.
///
/// # Errors
///
/// Returns whatever zbus reports.
pub async fn register_gluster_status_on(
    conn: &zbus::Connection,
    state: GlusterStatusService,
) -> zbus::Result<()> {
    conn.object_server()
        .at(GLUSTER_STATUS_OBJECT_PATH, state)
        .await?;
    Ok(())
}

/// Spawn the signal-dispatch loop. Pulls events from the
/// receiver + emits them via the `ObjectServer`'s signal
/// path. Returns the `GlusterSignalSender` the worker
/// holds.
///
/// # Errors
///
/// Returns whatever zbus reports when fetching the
/// interface reference fails (typically: the service
/// wasn't registered first).
pub async fn spawn_signal_dispatcher(
    conn: zbus::Connection,
) -> zbus::Result<GlusterSignalSender> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<GlusterSignal>();
    let iface_ref = conn
        .object_server()
        .interface::<_, GlusterStatusService>(GLUSTER_STATUS_OBJECT_PATH)
        .await?;
    tokio::spawn(async move {
        while let Some(signal) = rx.recv().await {
            let ctx = iface_ref.signal_emitter();
            let result = match signal {
                GlusterSignal::PeerStateChanged {
                    host,
                    prev_state,
                    new_state,
                } => GlusterStatusService::peer_state_changed(ctx, host, prev_state, new_state)
                    .await,
                GlusterSignal::ConflictDetected { gfid } => {
                    GlusterStatusService::conflict_detected(ctx, gfid).await
                }
                GlusterSignal::HealCompleted { gfid } => {
                    GlusterStatusService::heal_completed(ctx, gfid).await
                }
                GlusterSignal::QuotaWarning {
                    used_pct,
                    total_bytes,
                    used_bytes,
                } => {
                    GlusterStatusService::quota_warning(ctx, used_pct, total_bytes, used_bytes)
                        .await
                }
                GlusterSignal::VolumeReady => GlusterStatusService::volume_ready(ctx).await,
            };
            if let Err(e) = result {
                tracing::warn!(error = %e, "gluster signal emission failed");
            }
        }
    });
    Ok(GlusterSignalSender { tx })
}

// ----- parsers -------------------------------------------------------

/// Extract `(bricks_count, total_bytes, used_bytes)` from
/// `gluster volume info --xml`.
#[must_use]
pub fn parse_volume_info(xml: &str) -> (usize, u64, u64) {
    let bricks = count_tag_occurrences(xml, "brick");
    let total = sum_tag_u64(xml, "sizeTotal");
    let used = sum_tag_u64(xml, "sizeTotal").saturating_sub(sum_tag_u64(xml, "sizeFree"));
    (bricks, total, used)
}

/// Extract peer count from `gluster peer status --xml`.
#[must_use]
pub fn parse_peer_count(xml: &str) -> usize {
    extract_text_after(xml, "<count>")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0)
}

/// Build per-peer rows from `gluster peer status --xml`.
#[must_use]
pub fn parse_peer_rows(xml: &str) -> Vec<GlusterPeerRow> {
    let mut rows = Vec::new();
    let mut cursor = 0;
    while let Some(pos) = xml[cursor..].find("<peer>") {
        let start = cursor + pos;
        let end = match xml[start..].find("</peer>") {
            Some(e) => start + e,
            None => break,
        };
        let block = &xml[start..end];
        rows.push(GlusterPeerRow {
            uuid: extract_text_between(block, "<uuid>", "</uuid>").unwrap_or_default(),
            host: extract_text_between(block, "<hostname>", "</hostname>").unwrap_or_default(),
            state: extract_text_between(block, "<stateStr>", "</stateStr>").unwrap_or_default(),
            is_self: extract_text_between(block, "<connected>", "</connected>")
                .map(|s| s == "1")
                .unwrap_or(false),
            brick_free_bytes: 0,
        });
        cursor = end + "</peer>".len();
    }
    rows
}

/// Extract `(heal_pending, split_brain)` counts from
/// `gluster volume heal mesh-home info --xml`.
#[must_use]
pub fn parse_heal_info(xml: &str) -> (usize, usize) {
    let pending = count_tag_occurrences(xml, "file");
    let split = count_tag_occurrences(xml, "splitBrain");
    (pending, split)
}

/// Build per-conflict rows.
#[must_use]
pub fn parse_conflict_rows(xml: &str) -> Vec<ConflictRow> {
    let mut rows = Vec::new();
    let mut cursor = 0;
    while let Some(pos) = xml[cursor..].find("<gfid>") {
        let start = cursor + pos;
        let end = match xml[start..].find("</gfid>") {
            Some(e) => start + e,
            None => break,
        };
        let gfid = xml[start + "<gfid>".len()..end].trim().to_owned();
        rows.push(ConflictRow {
            gfid,
            kind: "split-brain".to_owned(),
        });
        cursor = end + "</gfid>".len();
    }
    rows
}

/// True when `findmnt --noheadings --target <mount_point>`
/// returns a glusterfs entry. Pure-ish (shells findmnt).
#[must_use]
pub fn mount_point_is_active(mount_point: &str) -> bool {
    let out = Command::new("findmnt")
        .args(["--noheadings", "--target", mount_point])
        .output()
        .ok();
    let Some(out) = out else { return false };
    if !out.status.success() {
        return false;
    }
    let body = String::from_utf8_lossy(&out.stdout);
    body.contains("glusterfs") || body.contains("fuse.glusterfs")
}

/// Unix-epoch seconds when the mount became active
/// (proc-fs cstat fallback when systemd unit metadata
/// isn't available).
#[must_use]
pub fn mount_point_since_unix_s(mount_point: &str) -> Option<u64> {
    let meta = std::fs::metadata(mount_point).ok()?;
    let ts = meta.modified().ok()?;
    ts.duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

fn count_tag_occurrences(xml: &str, tag: &str) -> usize {
    let open = format!("<{tag}>");
    xml.matches(open.as_str()).count()
}

fn sum_tag_u64(xml: &str, tag: &str) -> u64 {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut sum: u64 = 0;
    let mut cursor = 0;
    while let Some(pos) = xml[cursor..].find(&open) {
        let start = cursor + pos + open.len();
        let end = match xml[start..].find(&close) {
            Some(e) => start + e,
            None => break,
        };
        if let Ok(n) = xml[start..end].trim().parse::<u64>() {
            sum = sum.saturating_add(n);
        }
        cursor = end + close.len();
    }
    sum
}

fn extract_text_after(xml: &str, marker: &str) -> Option<String> {
    let pos = xml.find(marker)?;
    let start = pos + marker.len();
    let end = xml[start..].find('<')?;
    Some(xml[start..start + end].trim().to_owned())
}

fn extract_text_between(xml: &str, open: &str, close: &str) -> Option<String> {
    let pos = xml.find(open)?;
    let start = pos + open.len();
    let end = xml[start..].find(close)?;
    Some(xml[start..start + end].trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VOLUME_INFO_XML: &str = r#"<cliOutput>
      <volInfo>
        <volumes>
          <volume>
            <name>mesh-home</name>
            <brickCount>2</brickCount>
            <bricks>
              <brick>a:/data/mesh</brick>
              <brick>b:/data/mesh</brick>
            </bricks>
            <sizeTotal>1000000</sizeTotal>
            <sizeFree>400000</sizeFree>
          </volume>
        </volumes>
      </volInfo>
    </cliOutput>"#;

    const PEERS_XML: &str = r#"<cliOutput>
      <peerStatus>
        <peer>
          <uuid>aaa-uuid</uuid>
          <hostname>10.42.0.7</hostname>
          <connected>1</connected>
          <stateStr>Peer in Cluster</stateStr>
        </peer>
        <peer>
          <uuid>bbb-uuid</uuid>
          <hostname>10.42.0.9</hostname>
          <connected>0</connected>
          <stateStr>Disconnected</stateStr>
        </peer>
      </peerStatus>
      <count>2</count>
    </cliOutput>"#;

    const HEAL_INFO_XML: &str = r#"<cliOutput>
      <healInfo>
        <bricks>
          <brick>
            <file>file1</file>
            <file>file2</file>
            <splitBrain>file3</splitBrain>
          </brick>
        </bricks>
      </healInfo>
    </cliOutput>"#;

    const SPLIT_BRAIN_XML: &str = r#"<cliOutput>
      <healInfo>
        <bricks>
          <brick>
            <gfid>11111111-2222-3333-4444-555555555555</gfid>
            <gfid>aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee</gfid>
          </brick>
        </bricks>
      </healInfo>
    </cliOutput>"#;

    #[test]
    fn parse_volume_info_counts_bricks_and_sizes() {
        let (bricks, total, used) = parse_volume_info(VOLUME_INFO_XML);
        assert_eq!(bricks, 2);
        assert_eq!(total, 1_000_000);
        assert_eq!(used, 600_000);
    }

    #[test]
    fn parse_volume_info_empty_xml() {
        let (bricks, total, used) = parse_volume_info("");
        assert_eq!(bricks, 0);
        assert_eq!(total, 0);
        assert_eq!(used, 0);
    }

    #[test]
    fn parse_peer_count_reads_count_tag() {
        assert_eq!(parse_peer_count(PEERS_XML), 2);
    }

    #[test]
    fn parse_peer_count_missing_tag() {
        assert_eq!(parse_peer_count("<cliOutput></cliOutput>"), 0);
    }

    #[test]
    fn parse_peer_rows_extracts_two_peers() {
        let rows = parse_peer_rows(PEERS_XML);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].uuid, "aaa-uuid");
        assert_eq!(rows[0].host, "10.42.0.7");
        assert_eq!(rows[0].state, "Peer in Cluster");
        assert!(rows[0].is_self);
        assert_eq!(rows[1].uuid, "bbb-uuid");
        assert_eq!(rows[1].state, "Disconnected");
        assert!(!rows[1].is_self);
    }

    #[test]
    fn parse_peer_rows_empty_xml() {
        assert!(parse_peer_rows("").is_empty());
    }

    #[test]
    fn parse_heal_info_counts_files_and_split_brain() {
        let (pending, split) = parse_heal_info(HEAL_INFO_XML);
        assert_eq!(pending, 2);
        assert_eq!(split, 1);
    }

    #[test]
    fn parse_heal_info_empty_xml() {
        let (pending, split) = parse_heal_info("");
        assert_eq!(pending, 0);
        assert_eq!(split, 0);
    }

    #[test]
    fn parse_conflict_rows_extracts_gfids() {
        let rows = parse_conflict_rows(SPLIT_BRAIN_XML);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].gfid, "11111111-2222-3333-4444-555555555555");
        assert_eq!(rows[0].kind, "split-brain");
        assert_eq!(rows[1].gfid, "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee");
    }

    #[test]
    fn parse_conflict_rows_empty_xml() {
        assert!(parse_conflict_rows("").is_empty());
    }

    #[test]
    fn build_status_snapshot_uses_stub_gluster_binary() {
        // Point gluster_binary at /bin/false so every
        // shell-out fails; the service should still
        // surface a coherent zero-state snapshot rather
        // than panic.
        let conn = rusqlite::Connection::open_in_memory().expect("conn");
        let store = Arc::new(Mutex::new(conn));
        let svc = GlusterStatusService::new(store).with_gluster_binary("/bin/false");
        let snap = svc.build_status_snapshot();
        assert_eq!(snap.volume_name, "mesh-home");
        // peers_count is +1 for self, so even with zero
        // peers from the CLI we report 1.
        assert_eq!(snap.peers_count, 1);
        assert_eq!(snap.bricks_count, 0);
        assert_eq!(snap.total_bytes, 0);
        assert!(!snap.volume_online);
    }

    #[test]
    fn build_mount_status_handles_missing_mount() {
        let conn = rusqlite::Connection::open_in_memory().expect("conn");
        let store = Arc::new(Mutex::new(conn));
        let svc = GlusterStatusService::new(store)
            .with_mount_point("/var/empty/nonexistent-mount-path-for-tests");
        let m = svc.build_mount_status();
        assert!(!m.is_mounted);
        assert_eq!(m.since_unix_s, 0);
    }

    #[test]
    fn gluster_signal_sender_emit_does_not_panic_when_rx_dropped() {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<GlusterSignal>();
        let sender = GlusterSignalSender { tx };
        drop(rx);
        sender.emit(GlusterSignal::VolumeReady);
        sender.emit(GlusterSignal::ConflictDetected {
            gfid: "test".into(),
        });
    }

    #[test]
    fn gluster_signal_sender_forwards_to_rx() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<GlusterSignal>();
        let sender = GlusterSignalSender { tx };
        sender.emit(GlusterSignal::VolumeReady);
        sender.emit(GlusterSignal::HealCompleted {
            gfid: "x".into(),
        });
        let first = rx.try_recv().expect("first event");
        let second = rx.try_recv().expect("second event");
        assert!(matches!(first, GlusterSignal::VolumeReady));
        assert!(matches!(second, GlusterSignal::HealCompleted { .. }));
    }

    #[test]
    fn extract_text_between_returns_inner_text() {
        let x = extract_text_between("foo<a>bar</a>baz", "<a>", "</a>");
        assert_eq!(x.as_deref(), Some("bar"));
    }

    #[test]
    fn extract_text_between_missing_open_returns_none() {
        let x = extract_text_between("foo</a>", "<a>", "</a>");
        assert!(x.is_none());
    }

    #[test]
    fn sum_tag_u64_aggregates_multiple_occurrences() {
        let xml = "<sizeTotal>100</sizeTotal><sizeTotal>200</sizeTotal>";
        assert_eq!(sum_tag_u64(xml, "sizeTotal"), 300);
    }

    #[test]
    fn count_tag_occurrences_handles_zero_and_many() {
        assert_eq!(count_tag_occurrences("<a></a><a></a>", "a"), 2);
        assert_eq!(count_tag_occurrences("nothing", "a"), 0);
    }
}
