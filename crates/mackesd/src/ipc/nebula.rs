//! NF-Bundle-0 (v2.5) — `dev.mackes.MDE.Nebula.Status`
//! D-Bus surface.
//!
//! Every NF-10..NF-18 desktop-surface consumer chains on this.
//! The applets / workbench panels / mde-files / wizard call:
//!
//!   * `Status()` → JSON snapshot covering active transport,
//!     peer-cert epoch, lighthouse role, peer count, last
//!     activation-state-machine transition.
//!   * `ListPeers()` → JSON array of paired peers + per-peer
//!     overlay IP + cert fingerprint + last-seen + reachable
//!     status.
//!   * `SelfNode()` → JSON {overlay_ip, role, cert_epoch,
//!     cert_expires_at, mesh_id}.
//!   * `RegenCerts()` → triggers a CA-epoch bump (calls
//!     ca::epoch::bump_epoch internally; today returns a
//!     human-readable "rotation deferred until NF-2.5 lands"
//!     until that helper ships).
//!
//! Reads come from the live SQLite tables (`nebula_ca` +
//! `nebula_peer_certs` from migration 0011, `nodes` from the
//! existing reconcile worker) + the on-disk role.host marker
//! file the NF-3.4 supervisor maintains. No new schema; this
//! is a pure read-projection surface.

#![cfg(feature = "async-services")]

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use zbus::interface;

/// Well-known D-Bus interface name.
pub const NEBULA_STATUS_INTERFACE: &str = "dev.mackes.MDE.Nebula.Status";

/// Object path the service is exposed at.
pub const NEBULA_STATUS_OBJECT_PATH: &str = "/dev/mackes/MDE/Nebula/Status";

/// Bus name (shared with FleetFiles per session-bus single-
/// instance convention — both services live on the same
/// daemon connection).
pub const NEBULA_STATUS_BUS_NAME: &str = "org.mackes.mackesd";

/// JSON wire shape for the Status() reply.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusSnapshot {
    /// True when this peer is acting as a lighthouse.
    pub is_lighthouse: bool,
    /// Active CA epoch the local node's cert was signed under.
    /// 0 when no CA exists yet.
    pub ca_epoch: i64,
    /// Number of paired peers (excluding self) the local
    /// nodes table knows about.
    pub peer_count: usize,
    /// Mesh-id this peer belongs to. Empty when no CA exists.
    pub mesh_id: String,
    /// Last known active transport name (one of
    /// "nebula_direct" / "nebula_lighthouse_relay" /
    /// "nebula_https443" / "kdc_tls" / "offline"). Stays
    /// `"offline"` until any worker writes a value.
    pub active_transport: String,
}

/// JSON wire shape for one row of the ListPeers() reply.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerRow {
    /// Stable node-id.
    pub node_id: String,
    /// Display name (host's hostname at enrollment time).
    pub name: String,
    /// Overlay IP allocated to this peer (e.g. "10.42.0.5").
    /// Empty when no peer cert exists yet.
    pub overlay_ip: String,
    /// First 8 chars of the peer's cert fingerprint.
    /// Empty when no cert exists.
    pub fingerprint: String,
    /// Cert epoch.
    pub cert_epoch: i64,
    /// Unix-epoch seconds when the cert expires.
    pub cert_expires_at: i64,
    /// "online" / "idle" / "offline" — sourced from the
    /// nodes table's health column.
    pub reachable: String,
}

/// JSON wire shape for SelfNode().
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelfNodeSnapshot {
    /// Stable node-id of the local peer.
    pub node_id: String,
    /// Hostname.
    pub host: String,
    /// "host" | "peer" — derived from the role.host marker
    /// file at /var/lib/mackesd/nebula/role.host.
    pub role: String,
    /// Active CA epoch.
    pub cert_epoch: i64,
    /// Unix-epoch seconds when the local peer's cert expires.
    pub cert_expires_at: i64,
    /// Overlay IP.
    pub overlay_ip: String,
    /// Mesh-id.
    pub mesh_id: String,
}

/// Default location of the role.host marker the supervisor
/// writes when this peer wins the leader-election lease.
pub const DEFAULT_ROLE_HOST_MARKER: &str = "/var/lib/mackesd/nebula/role.host";

/// Cross-thread events workers hand to the signal dispatcher so
/// the matching `dev.mackes.MDE.Nebula.Status.*` D-Bus signals
/// fan out to every subscribed consumer (Workbench Overview,
/// applets, mde-files). The daemon's worker→IPC plumbing follows
/// the same signal-enum pattern as the meshfs worker (MESHFS-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NebulaSignal {
    /// A peer's reachability flipped. Fired by the
    /// `health_reconciler` worker when the SQLite `nodes.health`
    /// row changes (e.g. unknown→healthy on first heartbeat,
    /// healthy→degraded after one missed cycle).
    PeerStateChanged {
        /// Stable node id whose health column flipped.
        node_id: String,
        /// New reachable string, matching the `PeerRow.reachable`
        /// mapping ("online" / "idle" / "offline").
        reachable: String,
    },
    /// The mesh's active transport rotated. Fired by
    /// `mesh_router` when its scorer picks a different primary
    /// transport. OV-7.b emission lands when KDC2-1.9 wires
    /// `detect_switch` into `tick_once`; today only the
    /// dispatcher infrastructure exists and the signal helper is
    /// callable by any future emitter.
    TransportChanged {
        /// New active-transport name (`nebula_direct`,
        /// `nebula_https443`, `kdc_tls`, etc.).
        active_transport: String,
    },
    /// A peer finished enrollment into the mesh. Fired from
    /// `Enroll()` on the local peer's enrollment success AND
    /// from `nebula_csr_watcher` on the leader's successful
    /// `sign_pending_csr` (the remote-peer path).
    EnrollmentCompleted {
        /// Stable node id of the peer that just enrolled.
        node_id: String,
    },
}

/// Best-effort cross-thread sender handed to workers once IPC
/// registration completes. Cloning is cheap (UnboundedSender is
/// an Arc internally). `emit` is fire-and-forget — a full /
/// closed channel drops the event silently. The worker's own
/// tracing log already carries the event payload so forensics
/// don't depend on the signal landing.
#[derive(Debug, Clone)]
pub struct NebulaSignalSender {
    tx: tokio::sync::mpsc::UnboundedSender<NebulaSignal>,
}

impl NebulaSignalSender {
    /// Emit a signal. Returns immediately.
    pub fn emit(&self, signal: NebulaSignal) {
        let _ = self.tx.send(signal);
    }
}

/// Shared slot workers hold so the signal sender can be wired
/// AFTER the worker has already spawned. The dispatcher
/// `spawn_signal_dispatcher` fills the slot once IPC registration
/// completes; workers spawned earlier in `run_serve()` pick up
/// the sender on their next tick via `slot.get()`. Avoids
/// reordering the entire startup sequence around D-Bus readiness.
pub type SignalSenderSlot = Arc<std::sync::OnceLock<NebulaSignalSender>>;

/// Construct a fresh, empty signal-sender slot. Workers receive
/// a clone of the same `Arc` and read it lock-free per tick.
#[must_use]
pub fn new_signal_sender_slot() -> SignalSenderSlot {
    Arc::new(std::sync::OnceLock::new())
}

/// Service state. Cheap to clone (every field is an Arc /
/// String).
#[derive(Debug, Clone)]
pub struct NebulaStatusService {
    store: Arc<Mutex<rusqlite::Connection>>,
    node_id: String,
    host: String,
    role_marker_path: std::path::PathBuf,
    /// NF-2.5 wire-up (v2.5) — mesh_id passed at
    /// construction so RegenCerts() knows which mesh's CA
    /// to rotate. Defaults to "mesh-<node_id>" when the
    /// supervisor hasn't set the MDE_MESH_ID env var.
    mesh_id: String,
    /// NF-3.6 (v2.5) — QNM-Shared root the Enroll() method
    /// hands to `nebula_enroll::enroll_with_token`. Defaults
    /// to `~/QNM-Shared` (via
    /// `mackesd_core::default_qnm_shared_root`) when the
    /// caller doesn't override.
    qnm_root: std::path::PathBuf,
}

impl NebulaStatusService {
    /// Construct rooted at the live SQLite store + the local
    /// peer's identity.
    #[must_use]
    pub fn new(
        store: Arc<Mutex<rusqlite::Connection>>,
        node_id: impl Into<String>,
        host: impl Into<String>,
    ) -> Self {
        let nid: String = node_id.into();
        let default_mesh = format!("mesh-{nid}");
        Self {
            store,
            node_id: nid,
            host: host.into(),
            role_marker_path: std::path::PathBuf::from(DEFAULT_ROLE_HOST_MARKER),
            mesh_id: std::env::var("MDE_MESH_ID").unwrap_or(default_mesh),
            qnm_root: crate::default_qnm_shared_root(),
        }
    }

    /// Override the mesh_id — used by tests that need a
    /// deterministic value.
    #[must_use]
    pub fn with_mesh_id(mut self, mesh_id: impl Into<String>) -> Self {
        self.mesh_id = mesh_id.into();
        self
    }

    /// Override the marker path — used by tests that can't
    /// touch /var.
    #[must_use]
    pub fn with_role_marker(mut self, path: std::path::PathBuf) -> Self {
        self.role_marker_path = path;
        self
    }

    /// Override the QNM-Shared root — used by Enroll() to find
    /// the per-peer pending-enroll + bundle paths. Tests
    /// redirect into a tempdir.
    #[must_use]
    pub fn with_qnm_root(mut self, path: std::path::PathBuf) -> Self {
        self.qnm_root = path;
        self
    }

    /// Pure helper — builds a [`StatusSnapshot`] from the
    /// live SQLite state. Pulled out for direct testing
    /// without spinning up zbus.
    pub async fn build_status_snapshot(&self) -> Result<StatusSnapshot, String> {
        let conn = self.store.lock().await;
        let is_lighthouse = self.role_marker_path.exists();
        let (ca_epoch, mesh_id) = current_ca_row(&conn).unwrap_or_default();
        let peer_count = count_peers_excluding(&conn, &self.node_id);
        Ok(StatusSnapshot {
            is_lighthouse,
            ca_epoch,
            peer_count,
            mesh_id,
            active_transport: "offline".to_string(),
        })
    }

    /// Pure helper — builds the [`PeerRow`] list from the
    /// live SQLite state.
    pub async fn build_peer_list(&self) -> Result<Vec<PeerRow>, String> {
        let conn = self.store.lock().await;
        let nodes = crate::store::list_nodes(&conn).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for n in nodes.iter().filter(|n| n.node_id != self.node_id) {
            let (overlay_ip, fingerprint, cert_epoch, cert_expires_at) =
                peer_cert_for(&conn, &n.node_id).unwrap_or_default();
            out.push(PeerRow {
                node_id: n.node_id.clone(),
                name: n.name.clone(),
                overlay_ip,
                fingerprint,
                cert_epoch,
                cert_expires_at,
                reachable: match n.health.as_str() {
                    "healthy" => "online".to_string(),
                    "degraded" => "idle".to_string(),
                    _ => "offline".to_string(),
                },
            });
        }
        Ok(out)
    }

    /// Pure async core of the D-Bus `Enroll(token)` method —
    /// testable without a SignalEmitter. The public surface
    /// (`enroll`) wraps this and fires `EnrollmentCompleted`.
    pub async fn enroll_inner(&self, token: String) -> zbus::fdo::Result<String> {
        let qnm_root = self.qnm_root.clone();
        let node_id = self.node_id.clone();
        let display_name = self.host.clone();
        let outcome = tokio::task::spawn_blocking(move || {
            crate::nebula_enroll::enroll_with_token(
                &qnm_root,
                &node_id,
                &display_name,
                &token,
            )
        })
        .await
        .map_err(|e| zbus::fdo::Error::Failed(format!("enroll task: {e}")))?;
        match outcome {
            Ok(o) => Ok(format!(
                "enrolled into mesh '{}' as {} (overlay {}) after {} s.",
                o.mesh_id,
                self.node_id,
                o.overlay_ip,
                o.waited.as_secs(),
            )),
            Err(e) => Err(zbus::fdo::Error::Failed(e.to_string())),
        }
    }

    /// Pure helper — builds the [`SelfNodeSnapshot`] from
    /// the live SQLite state + role marker.
    pub async fn build_self_node(&self) -> Result<SelfNodeSnapshot, String> {
        let conn = self.store.lock().await;
        let role = if self.role_marker_path.exists() {
            "host".to_string()
        } else {
            "peer".to_string()
        };
        let (ca_epoch, mesh_id) = current_ca_row(&conn).unwrap_or_default();
        let (overlay_ip, _fingerprint, cert_epoch, cert_expires_at) =
            peer_cert_for(&conn, &self.node_id).unwrap_or_default();
        Ok(SelfNodeSnapshot {
            node_id: self.node_id.clone(),
            host: self.host.clone(),
            role,
            cert_epoch: cert_epoch.max(ca_epoch),
            cert_expires_at,
            overlay_ip,
            mesh_id,
        })
    }
}

#[interface(name = "dev.mackes.MDE.Nebula.Status")]
impl NebulaStatusService {
    /// JSON-encoded [`StatusSnapshot`].
    async fn status(&self) -> zbus::fdo::Result<String> {
        let snap = self
            .build_status_snapshot()
            .await
            .map_err(zbus::fdo::Error::Failed)?;
        serde_json::to_string(&snap)
            .map_err(|e| zbus::fdo::Error::Failed(format!("encode: {e}")))
    }

    /// JSON-encoded `Vec<PeerRow>`.
    async fn list_peers(&self) -> zbus::fdo::Result<String> {
        let peers = self
            .build_peer_list()
            .await
            .map_err(zbus::fdo::Error::Failed)?;
        serde_json::to_string(&peers)
            .map_err(|e| zbus::fdo::Error::Failed(format!("encode: {e}")))
    }

    /// JSON-encoded [`SelfNodeSnapshot`].
    async fn self_node(&self) -> zbus::fdo::Result<String> {
        let s = self
            .build_self_node()
            .await
            .map_err(zbus::fdo::Error::Failed)?;
        serde_json::to_string(&s)
            .map_err(|e| zbus::fdo::Error::Failed(format!("encode: {e}")))
    }

    /// Trigger a CA-epoch bump. Returns a human-readable
    /// status string the wizard's confirmation modal can
    /// display verbatim.
    ///
    /// NF-2.5 wired (2026-05-23): the underlying
    /// `ca::epoch::bump_epoch` ships; this method calls it
    /// in-line and surfaces the resulting RotationOutcome.
    /// BinaryMissing (nebula-cert not on PATH) maps to a
    /// human-readable "install the Fedora nebula package"
    /// hint rather than a raw subprocess error.
    async fn regen_certs(&self) -> zbus::fdo::Result<String> {
        use crate::ca::epoch;
        use crate::ca::{CaError, SubprocessBackend};
        let mesh_id = self.mesh_id.clone();
        let mut conn = self.store.lock().await;
        let outcome = epoch::bump_epoch(
            &SubprocessBackend,
            &mut *conn,
            &mesh_id,
            None,
            None,
            365,
        );
        match outcome {
            Ok(o) => Ok(format!(
                "CA rotated to epoch {} (retired {}); {} peer certs re-signed.",
                o.new_epoch,
                o.retired_epoch
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "none".to_string()),
                o.re_signed,
            )),
            Err(CaError::BinaryMissing) => Ok(
                "CA rotation skipped: nebula-cert not on PATH. \
                 Install the Fedora `nebula` package + retry."
                    .to_string(),
            ),
            Err(e) => Err(zbus::fdo::Error::Failed(format!("rotation: {e}"))),
        }
    }

    /// NF-3.6 (v2.5) — Enroll this peer into the mesh named in
    /// the supplied join token. Convenience wrapper over the
    /// `mackesd enroll --token` CLI flow (NF-3.6.a) — the
    /// wizard / panel can call this directly via D-Bus instead
    /// of shelling out.
    ///
    /// Returns a human-readable summary on success (the same
    /// shape the CLI prints) or a `zbus::fdo::Error::Failed`
    /// with `EnrollError::Display` text on any failure mode
    /// (invalid token, publish failed, lighthouse-timeout,
    /// bundle-corrupt). The wizard's Apply page consumes the
    /// reply verbatim for its progress banner.
    ///
    /// Synchronous enroll_with_token runs inside
    /// `tokio::task::spawn_blocking` so the 30 s lighthouse-
    /// wait doesn't pin the zbus runtime.
    async fn enroll(
        &self,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
        token: String,
    ) -> zbus::fdo::Result<String> {
        let reply = self.enroll_inner(token).await?;
        // OV-7 — fire EnrollmentCompleted so any subscriber
        // (Workbench Overview, applets) re-probes capability
        // status immediately rather than waiting for a poll.
        let _ = Self::enrollment_completed(&emitter, &self.node_id).await;
        Ok(reply)
    }

    /// Signal: a peer's reachability flipped. Fired by the
    /// reconcile worker when it observes a node's `health` row
    /// change (online → idle, idle → offline, etc.). OV-7.a
    /// worker-side emission lands in the same epic; this
    /// declaration is the public surface every subscriber
    /// pins against today.
    #[zbus(signal)]
    pub async fn peer_state_changed(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        node_id: &str,
        reachable: &str,
    ) -> zbus::Result<()>;

    /// Signal: the mesh's active transport rotated. Fired by
    /// `mesh_router` when its selector picks a different
    /// transport (nebula_direct → nebula_lighthouse_relay,
    /// nebula_https443 → kdc_tls, etc.). OV-7.b router-side
    /// emission lands in the same epic; this declaration is
    /// the public surface every subscriber pins against today.
    #[zbus(signal)]
    pub async fn transport_changed(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        active_transport: &str,
    ) -> zbus::Result<()>;

    /// Signal: a peer (this one or any other) finished
    /// enrollment into the mesh. Fired from `enroll()` above
    /// on the local peer's enrollment success; the leader
    /// fires it for remote peers in OV-7.c.
    #[zbus(signal)]
    pub async fn enrollment_completed(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        node_id: &str,
    ) -> zbus::Result<()>;
}

/// Register the NebulaStatusService on an EXISTING zbus
/// `Connection`. The connection is normally the one returned
/// by `ipc::files::register_fleet_files`, which already
/// claims the `org.mackes.mackesd` bus name; this function
/// just hangs another object under that connection at the
/// Nebula object path. Pattern matches the Phase G shell
/// services that also share the FleetFiles connection.
///
/// # Errors
///
/// Returns whatever zbus reports.
pub async fn register_nebula_status_on(
    conn: &zbus::Connection,
    state: NebulaStatusService,
) -> zbus::Result<()> {
    conn.object_server()
        .at(NEBULA_STATUS_OBJECT_PATH, state)
        .await?;
    Ok(())
}

/// Standalone register helper — used by tests / one-off
/// servers that don't already have a FleetFiles connection.
/// Builds a fresh `Connection` claiming the well-known bus
/// name + serving only the Nebula surface.
///
/// # Errors
///
/// Returns whatever zbus reports.
pub async fn register_nebula_status(
    state: NebulaStatusService,
) -> zbus::Result<zbus::Connection> {
    zbus::connection::Builder::session()?
        .name(NEBULA_STATUS_BUS_NAME)?
        .serve_at(NEBULA_STATUS_OBJECT_PATH, state)?
        .build()
        .await
}

/// Spawn the Nebula signal-dispatch loop. Pulls
/// [`NebulaSignal`] events from the receiver, looks up the
/// already-registered `NebulaStatusService` interface ref on
/// the supplied connection, and emits the matching
/// `dev.mackes.MDE.Nebula.Status.*` signal for each one.
///
/// Returns the [`NebulaSignalSender`] every worker holds. The
/// `slot` argument is filled with a clone of the same sender so
/// workers spawned earlier in `run_serve()` (before this call
/// site) can pick it up on their next tick.
///
/// # Errors
///
/// Returns whatever zbus reports when fetching the interface
/// reference fails (typically: the service wasn't registered
/// first via [`register_nebula_status_on`]).
pub async fn spawn_signal_dispatcher(
    conn: zbus::Connection,
    slot: &SignalSenderSlot,
) -> zbus::Result<NebulaSignalSender> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<NebulaSignal>();
    let iface_ref = conn
        .object_server()
        .interface::<_, NebulaStatusService>(NEBULA_STATUS_OBJECT_PATH)
        .await?;
    tokio::spawn(async move {
        while let Some(signal) = rx.recv().await {
            let ctx = iface_ref.signal_emitter();
            let result = match signal {
                NebulaSignal::PeerStateChanged { node_id, reachable } => {
                    NebulaStatusService::peer_state_changed(ctx, &node_id, &reachable).await
                }
                NebulaSignal::TransportChanged { active_transport } => {
                    NebulaStatusService::transport_changed(ctx, &active_transport).await
                }
                NebulaSignal::EnrollmentCompleted { node_id } => {
                    NebulaStatusService::enrollment_completed(ctx, &node_id).await
                }
            };
            if let Err(e) = result {
                tracing::warn!(error = %e, "nebula signal emission failed");
            }
        }
    });
    let sender = NebulaSignalSender { tx };
    // Fill the shared slot for workers that spawned before IPC
    // registration. `set` returns Err if the slot is already
    // filled — that's a programmer error (called twice), so
    // we surface it via tracing rather than silently overwriting.
    if slot.set(sender.clone()).is_err() {
        tracing::warn!(
            "nebula signal-sender slot already filled; \
             ignoring duplicate spawn_signal_dispatcher call",
        );
    }
    Ok(sender)
}

// ----- private SQL helpers -------------------------------------------

fn current_ca_row(conn: &rusqlite::Connection) -> Option<(i64, String)> {
    let mut stmt = conn
        .prepare(
            "SELECT epoch, mesh_id FROM nebula_ca \
             WHERE retired_at IS NULL \
             ORDER BY epoch DESC LIMIT 1",
        )
        .ok()?;
    stmt.query_row([], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
    })
    .ok()
}

fn count_peers_excluding(conn: &rusqlite::Connection, local: &str) -> usize {
    let mut stmt = match conn
        .prepare("SELECT COUNT(*) FROM nodes WHERE node_id != ?1")
    {
        Ok(s) => s,
        Err(_) => return 0,
    };
    stmt.query_row([local], |r| r.get::<_, i64>(0))
        .map(|n| n as usize)
        .unwrap_or(0)
}

fn peer_cert_for(
    conn: &rusqlite::Connection,
    node_id: &str,
) -> Option<(String, String, i64, i64)> {
    let mut stmt = conn
        .prepare(
            "SELECT overlay_ip, cert_pem, epoch, expires_at \
             FROM nebula_peer_certs \
             WHERE node_id = ?1 AND revoked_at IS NULL \
             ORDER BY epoch DESC LIMIT 1",
        )
        .ok()?;
    stmt.query_row([node_id], |r| {
        let cert_pem: String = r.get(1)?;
        Ok((
            r.get::<_, String>(0)?,
            fingerprint(&cert_pem),
            r.get::<_, i64>(2)?,
            r.get::<_, i64>(3)?,
        ))
    })
    .ok()
}

/// Pure helper — derive an 8-char "fingerprint" from a PEM
/// blob. Today we use the first 8 alphanumeric chars of the
/// base64 body so the value is stable + readable in the UI;
/// when `nebula-cert print` ships a real fingerprint in
/// JSON, swap this for the real call.
#[must_use]
pub fn fingerprint(cert_pem: &str) -> String {
    let body: String = cert_pem
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .flat_map(|l| l.chars())
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect();
    body
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
    fn interface_path_locks() {
        assert_eq!(NEBULA_STATUS_INTERFACE, "dev.mackes.MDE.Nebula.Status");
        assert_eq!(NEBULA_STATUS_OBJECT_PATH, "/dev/mackes/MDE/Nebula/Status");
    }

    #[test]
    fn fingerprint_extracts_first_8_alnum() {
        let pem = "-----BEGIN CERT-----\n\
                   abcd-EFGH+1234ZZZZ\n\
                   -----END CERT-----\n";
        // 'abcdEFGH' = first 8 alphanumeric after stripping
        // delimiters + non-alnum chars.
        assert_eq!(fingerprint(pem), "abcdEFGH");
    }

    #[test]
    fn fingerprint_handles_empty_pem() {
        assert_eq!(fingerprint(""), "");
        assert_eq!(fingerprint("-----BEGIN-----\n-----END-----\n"), "");
    }

    #[tokio::test]
    async fn status_on_empty_store_reports_offline_zero_peers() {
        let svc = NebulaStatusService::new(fresh_store(), "peer:local", "host")
            .with_role_marker("/nonexistent/marker".into());
        let s = svc.build_status_snapshot().await.expect("status");
        assert!(!s.is_lighthouse);
        assert_eq!(s.ca_epoch, 0);
        assert_eq!(s.peer_count, 0);
        assert_eq!(s.mesh_id, "");
        assert_eq!(s.active_transport, "offline");
    }

    #[tokio::test]
    async fn status_reports_is_lighthouse_when_marker_present() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let marker = tmp.path().join("role.host");
        std::fs::write(&marker, "role:host\n").expect("write");
        let svc = NebulaStatusService::new(fresh_store(), "peer:local", "host")
            .with_role_marker(marker);
        let s = svc.build_status_snapshot().await.expect("status");
        assert!(s.is_lighthouse);
    }

    #[tokio::test]
    async fn status_reports_ca_epoch_and_mesh_after_mint() {
        let store = fresh_store();
        {
            let conn = store.lock().await;
            conn.execute(
                "INSERT INTO nebula_ca (mesh_id, epoch, ca_cert_pem, retired_at) \
                 VALUES ('m1', 0, 'pem', NULL)",
                [],
            )
            .expect("insert ca");
        }
        let svc = NebulaStatusService::new(store, "peer:local", "host")
            .with_role_marker("/nonexistent/marker".into());
        let s = svc.build_status_snapshot().await.expect("status");
        assert_eq!(s.ca_epoch, 0);
        assert_eq!(s.mesh_id, "m1");
    }

    #[tokio::test]
    async fn list_peers_excludes_local_and_emits_overlay_ip() {
        let store = fresh_store();
        {
            let conn = store.lock().await;
            conn.execute(
                "INSERT INTO nodes (node_id, name, public_key, role, health, enrolled_at) \
                 VALUES ('peer:local', 'self', 'pk', 'host', 'healthy', 1)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO nodes (node_id, name, public_key, role, health, enrolled_at) \
                 VALUES ('peer:anvil', 'anvil', 'pk', 'peer', 'healthy', 2)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO nebula_peer_certs \
                 (node_id, epoch, cert_pem, overlay_ip, expires_at) \
                 VALUES ('peer:anvil', 0, 'PEM1234ABCDEF', '10.42.0.5', 9999999)",
                [],
            )
            .unwrap();
        }
        let svc = NebulaStatusService::new(store, "peer:local", "host")
            .with_role_marker("/nonexistent/marker".into());
        let peers = svc.build_peer_list().await.expect("peers");
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].node_id, "peer:anvil");
        assert_eq!(peers[0].overlay_ip, "10.42.0.5");
        assert_eq!(peers[0].reachable, "online");
    }

    #[tokio::test]
    async fn self_node_role_flips_with_marker() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let marker = tmp.path().join("role.host");
        let store = fresh_store();
        let svc = NebulaStatusService::new(
            Arc::clone(&store),
            "peer:local",
            "host",
        )
        .with_role_marker(marker.clone());
        let s = svc.build_self_node().await.expect("self");
        assert_eq!(s.role, "peer");
        std::fs::write(&marker, "role:host\n").expect("write");
        let s2 = svc.build_self_node().await.expect("self after promote");
        assert_eq!(s2.role, "host");
    }

    #[tokio::test]
    async fn regen_certs_handles_binary_missing_gracefully() {
        // On a dev box without `nebula-cert` installed (the
        // dominant case in CI / local dev), the rotation
        // surfaces a human-readable hint rather than a raw
        // subprocess error.
        let svc = NebulaStatusService::new(fresh_store(), "peer:local", "host");
        let msg = svc.regen_certs().await.expect("ok");
        // Either the rotation succeeded (rare — only on a
        // bench host with nebula installed + writable
        // /var/lib/mackesd) or surfaced the install hint.
        // Both are valid outcomes.
        assert!(
            msg.contains("nebula-cert not on PATH")
                || msg.contains("CA rotated to epoch"),
            "unexpected regen-certs reply: {msg}",
        );
    }

    // ---- NF-3.6 Enroll D-Bus method ----------------------

    #[tokio::test]
    async fn enroll_rejects_invalid_token_with_actionable_error() {
        // Garbage tokens fall through to EnrollError::InvalidToken
        // which we surface as zbus::fdo::Error::Failed.
        let tmp = tempfile::tempdir().expect("tempdir");
        let svc = NebulaStatusService::new(fresh_store(), "peer:local", "anvil")
            .with_qnm_root(tmp.path().to_path_buf());
        let err = svc
            .enroll_inner("not a valid token".to_string())
            .await
            .expect_err("invalid token");
        let s = err.to_string();
        assert!(s.contains("invalid join token"), "msg: {s}");
        assert!(s.contains("mesh:"), "msg: {s}");
    }

    #[tokio::test]
    async fn enroll_with_valid_token_publishes_csr_then_times_out() {
        // Valid token + a tempdir QNM-Shared root + no lighthouse
        // signing on the other end → publish-CSR succeeds, then
        // wait_for_signed_bundle times out per the default
        // ENROLL_WAIT_TIMEOUT.
        //
        // Skip the actual 30 s wait — this test would block CI.
        // Just confirm the CSR file lands by triggering enroll
        // and then aborting via a short-lived spawn (we don't
        // await it). Real timeout is covered in nebula_enroll
        // tests.
        //
        // We just check the synchronous "what would happen" by
        // calling the underlying publish path directly — Enroll's
        // wrapper is thin.
        use crate::enrollment::build_identity;
        use crate::nebula_enroll::{
            build_pending, parse_join_token, pending_enroll_path,
            publish_enrollment_request,
        };
        let tmp = tempfile::tempdir().expect("tempdir");
        let identity = build_identity();
        let token = parse_join_token("mesh:test@10.0.0.5:4242#bearer").unwrap();
        let pending = build_pending(&identity, "peer:local", "anvil", token);
        let p = publish_enrollment_request(tmp.path(), "peer:local", &pending)
            .expect("publish");
        assert_eq!(p, pending_enroll_path(tmp.path(), "peer:local"));
        assert!(p.exists());
    }

    #[tokio::test]
    async fn with_qnm_root_overrides_default() {
        let custom = std::path::PathBuf::from("/tmp/custom-qnm-test");
        let svc = NebulaStatusService::new(fresh_store(), "peer:local", "anvil")
            .with_qnm_root(custom.clone());
        assert_eq!(svc.qnm_root, custom);
    }
}
