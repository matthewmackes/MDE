//! `DBusBackend` — wire-level zbus client for mded's
//! `dev.mackes.MDE.Shell.*` + `dev.mackes.MDE.Fleet.Files`
//! surfaces (Phase 2.4 schemas).
//!
//! v2.0.0 Phase 2.3 — gated behind the `dbus` cargo feature so the
//! headless DemoBackend smoke build (panel-boot-in-200 ms gate)
//! doesn't pull tokio + zbus.
//!
//! Phase-2.3 scope: wire types + parsers + the runtime-backed
//! `Connection` wrapper. The full `impl Backend for DBusBackend`
//! waits on Phase G:
//!
//! 1. mded's handlers in `crates/mackesd/src/ipc/files.rs` today
//!    return `Err(Failed("Phase G"))` — no live data lands yet.
//! 2. `crate::model::{Peer, SelfNode, FileRow}` use `&'static str`
//!    fields (the demo-data scaffold contract). The trait impl
//!    needs the model to migrate to owned `String` or leak strings
//!    permanently; deferred until the model swap lands in Phase G.
//!
//! Meanwhile this module ships the testable bits: the wire types,
//! the parsers (covered by 10 unit tests), the connect path
//! (compile-checked behind `--features dbus`), and the selector +
//! mode + conflict-policy enum bridges so Phase G can drop straight
//! into place.

#![cfg(feature = "dbus")]

use serde::Deserialize;
use tokio::runtime::Runtime;
use zbus::{Connection, Proxy};

use crate::backend::{BackendError, ConflictPolicy, Destination, SendMode};

/// D-Bus destination — well-known bus name registered by mackesd.
pub const BUS_NAME: &str = "org.mackes.mackesd";

/// Phase 2.4 object paths (mirror of
/// `crates/mackesd/src/ipc/files.rs` constants).
pub const FLEET_FILES_OBJECT_PATH: &str = "/dev/mackes/MDE/Fleet/Files";
pub const SHELL_INBOX_OBJECT_PATH: &str = "/dev/mackes/MDE/Shell/Inbox";
pub const SHELL_OUTBOX_OBJECT_PATH: &str = "/dev/mackes/MDE/Shell/Outbox";
pub const SHELL_DOWNLOADS_OBJECT_PATH: &str = "/dev/mackes/MDE/Shell/Downloads";
pub const SHELL_FILE_OPERATIONS_OBJECT_PATH: &str = "/dev/mackes/MDE/Shell/FileOperations";

pub const FLEET_FILES_INTERFACE: &str = "dev.mackes.MDE.Fleet.Files";
pub const SHELL_INBOX_INTERFACE: &str = "dev.mackes.MDE.Shell.Inbox";
pub const SHELL_OUTBOX_INTERFACE: &str = "dev.mackes.MDE.Shell.Outbox";
pub const SHELL_DOWNLOADS_INTERFACE: &str = "dev.mackes.MDE.Shell.Downloads";
pub const SHELL_FILE_OPERATIONS_INTERFACE: &str = "dev.mackes.MDE.Shell.FileOperations";

/// Wire-format `SelfNode` as mded encodes it.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct WireSelfNode {
    pub host: String,
    pub role: String,
    pub region: String,
}

/// Wire-format `Peer` row.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct WirePeer {
    pub name: String,
    pub addr: String,
    pub kind: String,
    pub status: String,
}

/// Wire-format `FileRow`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct WireFileRow {
    pub name: String,
    pub size: u64,
    pub mime: String,
    pub peer: String,
    pub modified_ms: i64,
}

/// Wire-format audit row.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct WireAudit {
    pub op_id: u64,
    pub kind: String,
    pub source: String,
    pub destination: String,
    pub mode: String,
    pub bytes: u64,
    pub at_ms: i64,
    pub ok: bool,
}

/// Tokio-runtime-backed bridge wrapping a zbus connection. Phase
/// 2.3 ships the connect + per-call dispatch; the
/// `impl Backend for DBusBackend` lands in Phase G when the model
/// owns its strings.
pub struct DBusBackend {
    rt: Runtime,
    connection: Connection,
}

impl DBusBackend {
    /// Connect to the session bus + open a `Connection` reused by
    /// every Backend call.
    ///
    /// # Errors
    ///
    /// Returns `BackendError::Rejected` if the runtime fails to
    /// build or the connection cannot be opened.
    pub fn connect() -> Result<Self, BackendError> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|e| BackendError::Rejected(format!("tokio runtime: {e}")))?;
        let connection = rt
            .block_on(Connection::session())
            .map_err(|e| BackendError::Rejected(format!("session bus: {e}")))?;
        Ok(Self { rt, connection })
    }

    /// Call a method that returns a JSON string body. Surfaces
    /// errors as `BackendError::Rejected` with a human-readable
    /// reason for the audit log.
    ///
    /// # Errors
    ///
    /// Returns `BackendError::Rejected` if the proxy can't be
    /// constructed, the method call fails, or the body fails to
    /// decode.
    pub fn call_string_method(
        &self,
        path: &str,
        iface: &str,
        method: &str,
    ) -> Result<String, BackendError> {
        self.rt.block_on(async {
            let proxy = Proxy::new(&self.connection, BUS_NAME, path, iface)
                .await
                .map_err(|e| BackendError::Rejected(format!("proxy {iface}: {e}")))?;
            proxy
                .call_method(method, &())
                .await
                .map_err(|e| BackendError::Rejected(format!("{iface}.{method}: {e}")))?
                .body()
                .deserialize::<String>()
                .map_err(|e| BackendError::Rejected(format!("decode {iface}.{method}: {e}")))
        })
    }
}

// ---- pure parsers (testable, no I/O) -----------------------------

/// Parse the JSON-encoded SelfNode mded returns.
#[must_use]
pub fn parse_self_node(raw: &str) -> Option<WireSelfNode> {
    serde_json::from_str(raw).ok()
}

/// Parse a JSON array of peers.
#[must_use]
pub fn parse_peers(raw: &str) -> Option<Vec<WirePeer>> {
    serde_json::from_str(raw).ok()
}

/// Parse a JSON array of file rows.
#[must_use]
pub fn parse_files(raw: &str) -> Option<Vec<WireFileRow>> {
    serde_json::from_str(raw).ok()
}

/// Parse the JSON-encoded audit log.
#[must_use]
pub fn parse_audit(raw: &str) -> Option<Vec<WireAudit>> {
    serde_json::from_str(raw).ok()
}

/// Encode a `Destination` into the mded selector grammar.
#[must_use]
pub fn destination_to_selector(d: &Destination) -> String {
    match d {
        Destination::Peer(n) => format!("peer:{n}"),
        Destination::Group(g) => format!("group:{g}"),
        Destination::Role(r) => format!("role:{r}"),
        Destination::Site(s) => format!("site:{s}"),
    }
}

/// Inverse: parse the mded selector grammar.
#[must_use]
pub fn parse_destination(raw: &str) -> Destination {
    if let Some(rest) = raw.strip_prefix("peer:") {
        Destination::Peer(rest.to_string())
    } else if let Some(rest) = raw.strip_prefix("group:") {
        Destination::Group(rest.to_string())
    } else if let Some(rest) = raw.strip_prefix("role:") {
        Destination::Role(rest.to_string())
    } else if let Some(rest) = raw.strip_prefix("site:") {
        Destination::Site(rest.to_string())
    } else {
        Destination::Peer(raw.to_string())
    }
}

#[must_use]
pub fn send_mode_to_str(m: SendMode) -> &'static str {
    match m {
        SendMode::Copy => "copy",
        SendMode::Move => "move",
        SendMode::Sync => "sync",
        SendMode::Deploy => "deploy",
        SendMode::Stage => "stage",
    }
}

#[must_use]
pub fn parse_send_mode(s: &str) -> SendMode {
    match s {
        "move" => SendMode::Move,
        "sync" => SendMode::Sync,
        "deploy" => SendMode::Deploy,
        "stage" => SendMode::Stage,
        _ => SendMode::Copy,
    }
}

#[must_use]
pub fn conflict_policy_to_str(c: ConflictPolicy) -> &'static str {
    match c {
        ConflictPolicy::Ask => "ask",
        ConflictPolicy::Skip => "skip",
        ConflictPolicy::Overwrite => "overwrite",
        ConflictPolicy::Rename => "rename",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_self_node_round_trips_basic_shape() {
        let raw = r#"{"host":"anvil","role":"editor","region":"lab"}"#;
        let n = parse_self_node(raw).expect("decoded");
        assert_eq!(n.host, "anvil");
        assert_eq!(n.role, "editor");
        assert_eq!(n.region, "lab");
    }

    #[test]
    fn parse_self_node_returns_none_on_garbage() {
        assert!(parse_self_node("not json").is_none());
    }

    #[test]
    fn parse_peers_decodes_array() {
        let raw = r#"[
            {"name":"pine","addr":"10.0.0.1","kind":"laptop","status":"online"},
            {"name":"birch","addr":"10.0.0.2","kind":"server","status":"offline"}
        ]"#;
        let peers = parse_peers(raw).expect("decoded");
        assert_eq!(peers.len(), 2);
        assert_eq!(peers[0].name, "pine");
        assert_eq!(peers[1].kind, "server");
        assert_eq!(peers[1].status, "offline");
    }

    #[test]
    fn parse_files_decodes_rows() {
        let raw = r#"[
            {"name":"notes.md","size":1234,"mime":"doc","peer":"pine","modified_ms":1715000000000}
        ]"#;
        let rows = parse_files(raw).expect("decoded");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "notes.md");
        assert_eq!(rows[0].size, 1234);
    }

    #[test]
    fn parse_audit_round_trips_basic_row() {
        let raw = r#"[
            {"op_id":42,"kind":"send_to","source":"/tmp/a","destination":"peer:pine","mode":"copy","bytes":4096,"at_ms":1715000000000,"ok":true}
        ]"#;
        let rows = parse_audit(raw).expect("decoded");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].op_id, 42);
        assert_eq!(rows[0].kind, "send_to");
        assert_eq!(rows[0].destination, "peer:pine");
    }

    #[test]
    fn destination_selector_round_trip() {
        for d in [
            Destination::Peer("pine".into()),
            Destination::Group("crew".into()),
            Destination::Role("editor".into()),
            Destination::Site("lab".into()),
        ] {
            let s = destination_to_selector(&d);
            assert_eq!(parse_destination(&s), d);
        }
    }

    #[test]
    fn parse_destination_falls_back_to_peer_on_unknown_prefix() {
        let d = parse_destination("nothing-prefixed");
        assert_eq!(d, Destination::Peer("nothing-prefixed".into()));
    }

    #[test]
    fn send_mode_round_trip() {
        for m in [
            SendMode::Copy,
            SendMode::Move,
            SendMode::Sync,
            SendMode::Deploy,
            SendMode::Stage,
        ] {
            assert_eq!(parse_send_mode(send_mode_to_str(m)), m);
        }
    }

    #[test]
    fn conflict_policy_to_str_covers_all_variants() {
        assert_eq!(conflict_policy_to_str(ConflictPolicy::Ask), "ask");
        assert_eq!(conflict_policy_to_str(ConflictPolicy::Skip), "skip");
        assert_eq!(
            conflict_policy_to_str(ConflictPolicy::Overwrite),
            "overwrite"
        );
        assert_eq!(conflict_policy_to_str(ConflictPolicy::Rename), "rename");
    }

    #[test]
    fn interface_constants_match_mded_phase_2_4() {
        // Cross-check: these must equal the constants in
        // crates/mackesd/src/ipc/files.rs.
        assert_eq!(FLEET_FILES_INTERFACE, "dev.mackes.MDE.Fleet.Files");
        assert_eq!(SHELL_INBOX_INTERFACE, "dev.mackes.MDE.Shell.Inbox");
        assert_eq!(SHELL_OUTBOX_INTERFACE, "dev.mackes.MDE.Shell.Outbox");
        assert_eq!(SHELL_DOWNLOADS_INTERFACE, "dev.mackes.MDE.Shell.Downloads");
        assert_eq!(
            SHELL_FILE_OPERATIONS_INTERFACE,
            "dev.mackes.MDE.Shell.FileOperations"
        );
        assert_eq!(FLEET_FILES_OBJECT_PATH, "/dev/mackes/MDE/Fleet/Files");
        assert_eq!(SHELL_INBOX_OBJECT_PATH, "/dev/mackes/MDE/Shell/Inbox");
        assert_eq!(SHELL_OUTBOX_OBJECT_PATH, "/dev/mackes/MDE/Shell/Outbox");
        assert_eq!(
            SHELL_DOWNLOADS_OBJECT_PATH,
            "/dev/mackes/MDE/Shell/Downloads"
        );
        assert_eq!(
            SHELL_FILE_OPERATIONS_OBJECT_PATH,
            "/dev/mackes/MDE/Shell/FileOperations"
        );
    }
}

// When the `dbus` feature isn't enabled, the entire module is
// gated off by the file-level `#![cfg(feature = "dbus")]`. To keep
// the lib's surface non-empty in both modes, the lib.rs only
// `pub mod dbus_backend`s under the same gate.
