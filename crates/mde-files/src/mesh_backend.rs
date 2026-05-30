//! Mesh-first backend — talks to mackesd's live `dev.mackes.MDE.
//! Nebula.Status` D-Bus surface and exposes a mesh-flavoured API
//! the UI binds against (peer roster, overlay status).
//!
//! Why this exists. The previous `DBusBackend` talked to
//! `dev.mackes.MDE.Fleet.Files`, which returns `[]` for
//! `ListPeer(_)` because mackesd doesn't maintain a per-peer file
//! index. For a mesh-first manager the right source of truth is
//! Nebula.Status — peer reachability + overlay IPs + handshake
//! age. Live as of NF-Bundle-0 (v2.5).
//!
//! No new D-Bus surface is added — everything here is a thin
//! client over what mackesd already serves.

#![cfg(feature = "dbus")]

use std::time::Duration;

use serde::Deserialize;
use tokio::runtime::Runtime;
use zbus::{Connection, Proxy};

/// Well-known bus name mackesd registers.
pub const BUS_NAME: &str = "org.mackes.mackesd";

/// Object path for `dev.mackes.MDE.Nebula.Status` (mirror of
/// `mackesd_core::ipc::nebula::NEBULA_STATUS_OBJECT_PATH`).
pub const NEBULA_STATUS_OBJECT_PATH: &str = "/dev/mackes/MDE/Nebula/Status";

/// Interface name for `dev.mackes.MDE.Nebula.Status`.
pub const NEBULA_STATUS_INTERFACE: &str = "dev.mackes.MDE.Nebula.Status";

/// Errors a mesh-backend call can surface. `Unavailable` is the
/// common case (mackesd not running); `Decode` only fires when
/// the daemon's JSON shape drifts past what this client parses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshError {
    /// mackesd isn't on the session bus, or the call timed out
    /// before a reply arrived.
    Unavailable(String),
    /// Daemon replied but the JSON didn't deserialize.
    Decode(String),
}

impl std::fmt::Display for MeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(s) => write!(f, "mesh unavailable: {s}"),
            Self::Decode(s) => write!(f, "mesh decode failed: {s}"),
        }
    }
}

impl std::error::Error for MeshError {}

// ----- wire types (mirror mackesd's IPC structs) ----------------

/// Nebula `Status()` snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NebulaStatus {
    pub is_lighthouse: bool,
    pub ca_epoch: i64,
    pub peer_count: usize,
    pub mesh_id: String,
    pub active_transport: String,
}

/// Nebula `ListPeers()` row.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NebulaPeer {
    pub node_id: String,
    pub name: String,
    pub overlay_ip: String,
    pub fingerprint: String,
    pub cert_epoch: i64,
    pub cert_expires_at: i64,
    /// "online" / "idle" / "offline"
    pub reachable: String,
}

/// Nebula `SelfNode()` snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NebulaSelfNode {
    pub node_id: String,
    pub host: String,
    pub role: String,
    pub cert_epoch: i64,
    pub cert_expires_at: i64,
    pub overlay_ip: String,
    pub mesh_id: String,
}

// ----- merged shape the UI binds against -----------------------

/// One row in the mesh-first peer list — Nebula peer reachability
/// + overlay identity.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MeshPeer {
    /// Stable peer node-id (Nebula's `node_id`).
    pub node_id: String,
    /// Display name (Nebula's `name`, fallback hostname).
    pub host: String,
    /// 10.42.x.x — Nebula overlay IP.
    pub overlay_ip: String,
    /// "online" / "idle" / "offline" — from Nebula.
    pub reachable: String,
    /// Last cert epoch the lighthouse signed for this peer.
    pub cert_epoch: i64,
}

// ----- backend client ------------------------------------------

/// Cheap-to-construct mesh-backend client. Connection + tokio
/// runtime are opened once at start-up; each call blocks the
/// caller until the daemon replies (with a per-call timeout so
/// the UI thread never freezes for the dbus-defaults 25 s).
pub struct MeshBackend {
    rt: Runtime,
    connection: Connection,
    /// Per-call timeout — keeps the GUI thread snappy when
    /// mackesd is busy.
    call_timeout: Duration,
}

impl MeshBackend {
    /// Default connect timeout.
    pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_millis(800);
    /// Default per-method call timeout.
    pub const DEFAULT_CALL_TIMEOUT: Duration = Duration::from_millis(750);

    /// Connect to the session bus + verify mackesd is reachable.
    /// Identical handshake to `DBusBackend::connect_with_timeout`
    /// so the failure mode matches what existing callers expect.
    pub fn connect_with_timeout(timeout: Duration) -> Result<Self, MeshError> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|e| MeshError::Unavailable(format!("tokio runtime: {e}")))?;
        let connection = rt.block_on(async {
            tokio::time::timeout(timeout, Connection::session())
                .await
                .map_err(|_| MeshError::Unavailable("session bus: timeout".into()))?
                .map_err(|e| MeshError::Unavailable(format!("session bus: {e}")))
        })?;
        let alive: bool = rt.block_on(async {
            let dbus_proxy = Proxy::new(
                &connection,
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus",
            )
            .await
            .map_err(|e| MeshError::Unavailable(format!("dbus proxy: {e}")))?;
            let res: zbus::Result<bool> = tokio::time::timeout(timeout, async {
                dbus_proxy
                    .call_method("NameHasOwner", &(BUS_NAME,))
                    .await?
                    .body()
                    .deserialize::<bool>()
            })
            .await
            .map_err(|_| zbus::Error::Failure("NameHasOwner timeout".into()))
            .and_then(|x| x);
            res.map_err(|e| MeshError::Unavailable(format!("NameHasOwner: {e}")))
        })?;
        if !alive {
            return Err(MeshError::Unavailable(format!(
                "{BUS_NAME} not on the session bus"
            )));
        }
        Ok(Self {
            rt,
            connection,
            call_timeout: Self::DEFAULT_CALL_TIMEOUT,
        })
    }

    /// Convenience — connect with the default timeout.
    pub fn connect() -> Result<Self, MeshError> {
        Self::connect_with_timeout(Self::DEFAULT_CONNECT_TIMEOUT)
    }

    /// Override the per-call timeout. Tests use this to keep the
    /// suite fast.
    #[must_use]
    pub fn with_call_timeout(mut self, t: Duration) -> Self {
        self.call_timeout = t;
        self
    }

    /// Live Nebula overlay status.
    pub fn nebula_status(&self) -> Result<NebulaStatus, MeshError> {
        let raw = self.call_string(
            NEBULA_STATUS_OBJECT_PATH,
            NEBULA_STATUS_INTERFACE,
            "Status",
        )?;
        parse_nebula_status(&raw)
            .ok_or_else(|| MeshError::Decode(format!("nebula_status: {raw}")))
    }

    /// Live Nebula peer roster.
    pub fn nebula_peers(&self) -> Result<Vec<NebulaPeer>, MeshError> {
        let raw = self.call_string(
            NEBULA_STATUS_OBJECT_PATH,
            NEBULA_STATUS_INTERFACE,
            "ListPeers",
        )?;
        parse_nebula_peers(&raw)
            .ok_or_else(|| MeshError::Decode(format!("nebula_peers: {raw}")))
    }

    /// Live Nebula self-node snapshot.
    pub fn nebula_self_node(&self) -> Result<NebulaSelfNode, MeshError> {
        let raw = self.call_string(
            NEBULA_STATUS_OBJECT_PATH,
            NEBULA_STATUS_INTERFACE,
            "SelfNode",
        )?;
        parse_nebula_self_node(&raw)
            .ok_or_else(|| MeshError::Decode(format!("nebula_self_node: {raw}")))
    }

    /// Live peer list — Nebula peer roster converted to the mesh shape.
    pub fn mesh_peers(&self) -> Result<Vec<MeshPeer>, MeshError> {
        let peers = self.nebula_peers().unwrap_or_default();
        Ok(peers.into_iter().map(nebula_peer_to_mesh_peer).collect())
    }

    fn call_string(
        &self,
        path: &str,
        iface: &str,
        method: &str,
    ) -> Result<String, MeshError> {
        self.rt.block_on(async {
            let proxy = Proxy::new(&self.connection, BUS_NAME, path, iface)
                .await
                .map_err(|e| MeshError::Unavailable(format!("proxy {iface}: {e}")))?;
            let res: zbus::Result<String> = tokio::time::timeout(self.call_timeout, async {
                proxy
                    .call_method(method, &())
                    .await?
                    .body()
                    .deserialize::<String>()
            })
            .await
            .map_err(|_| zbus::Error::Failure(format!("{iface}.{method} timeout")))
            .and_then(|x| x);
            res.map_err(|e| MeshError::Unavailable(format!("{iface}.{method}: {e}")))
        })
    }
}

// ----- pure helpers --------------------------------------------

/// Parse the JSON returned by `dev.mackes.MDE.Nebula.Status::Status`.
#[must_use]
pub fn parse_nebula_status(raw: &str) -> Option<NebulaStatus> {
    serde_json::from_str(raw).ok()
}

/// Parse the JSON returned by `dev.mackes.MDE.Nebula.Status::ListPeers`.
#[must_use]
pub fn parse_nebula_peers(raw: &str) -> Option<Vec<NebulaPeer>> {
    serde_json::from_str(raw).ok()
}

/// Parse the JSON returned by `dev.mackes.MDE.Nebula.Status::SelfNode`.
#[must_use]
pub fn parse_nebula_self_node(raw: &str) -> Option<NebulaSelfNode> {
    serde_json::from_str(raw).ok()
}

/// Convert a `NebulaPeer` into the UI-facing `MeshPeer` shape.
#[must_use]
pub fn nebula_peer_to_mesh_peer(n: NebulaPeer) -> MeshPeer {
    MeshPeer {
        node_id: n.node_id,
        host: n.name,
        overlay_ip: n.overlay_ip,
        reachable: n.reachable,
        cert_epoch: n.cert_epoch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_nebula_status_decodes_real_shape() {
        let raw = r#"{
            "is_lighthouse": true,
            "ca_epoch": 3,
            "peer_count": 5,
            "mesh_id": "mesh-abc",
            "active_transport": "nebula_direct"
        }"#;
        let s = parse_nebula_status(raw).expect("parse");
        assert!(s.is_lighthouse);
        assert_eq!(s.ca_epoch, 3);
        assert_eq!(s.peer_count, 5);
        assert_eq!(s.mesh_id, "mesh-abc");
    }

    #[test]
    fn parse_nebula_status_returns_none_on_garbage() {
        assert!(parse_nebula_status("not json").is_none());
    }

    #[test]
    fn parse_nebula_peers_decodes_array() {
        let raw = r#"[
            {"node_id":"peer:pine","name":"pine","overlay_ip":"10.42.0.5","fingerprint":"abc","cert_epoch":3,"cert_expires_at":0,"reachable":"online"},
            {"node_id":"peer:birch","name":"birch","overlay_ip":"10.42.0.6","fingerprint":"def","cert_epoch":3,"cert_expires_at":0,"reachable":"offline"}
        ]"#;
        let rows = parse_nebula_peers(raw).expect("parse");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].overlay_ip, "10.42.0.5");
        assert_eq!(rows[1].reachable, "offline");
    }

    #[test]
    fn parse_nebula_self_node_decodes() {
        let raw = r#"{
            "node_id":"peer:anvil","host":"anvil","role":"host",
            "cert_epoch":3,"cert_expires_at":1234567890,
            "overlay_ip":"10.42.0.1","mesh_id":"mesh-abc"
        }"#;
        let s = parse_nebula_self_node(raw).expect("parse");
        assert_eq!(s.overlay_ip, "10.42.0.1");
        assert_eq!(s.role, "host");
    }

    #[test]
    fn nebula_peer_to_mesh_peer_maps_fields() {
        let n = NebulaPeer {
            node_id: "peer:pine".into(),
            name: "pine".into(),
            overlay_ip: "10.42.0.5".into(),
            fingerprint: "f".into(),
            cert_epoch: 3,
            cert_expires_at: 0,
            reachable: "online".into(),
        };
        let mp = nebula_peer_to_mesh_peer(n);
        assert_eq!(mp.node_id, "peer:pine");
        assert_eq!(mp.host, "pine");
        assert_eq!(mp.overlay_ip, "10.42.0.5");
        assert_eq!(mp.reachable, "online");
        assert_eq!(mp.cert_epoch, 3);
    }

    #[test]
    fn mesh_error_display_carries_context() {
        let e = MeshError::Unavailable("session bus closed".into());
        assert!(format!("{e}").contains("session bus closed"));
        let e = MeshError::Decode("bad JSON".into());
        assert!(format!("{e}").contains("bad JSON"));
    }

    #[test]
    fn const_paths_mirror_mackesd() {
        assert_eq!(NEBULA_STATUS_INTERFACE, "dev.mackes.MDE.Nebula.Status");
        assert_eq!(NEBULA_STATUS_OBJECT_PATH, "/dev/mackes/MDE/Nebula/Status");
        assert_eq!(BUS_NAME, "org.mackes.mackesd");
    }
}
