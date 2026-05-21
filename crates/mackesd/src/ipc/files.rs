//! `dev.mackes.MDE.Shell.{Inbox,Outbox,Downloads,FileOperations}` +
//! `dev.mackes.MDE.Fleet.Files` — file-transfer surfaces served by
//! mackesd that the MDE-Files panel (Phase 2.3 DBusBackend) calls
//! over zbus.
//!
//! v2.0.0 Phase 2.4 (locked 2026-05-19) — Phase A ships the schemas;
//! handler bodies return `Err(zbus::fdo::Error::Failed("…not
//! implemented…"))` until Phase G wires them to the live transfer
//! engine.

#![cfg(feature = "async-services")]

use zbus::interface;

// ---- dev.mackes.MDE.Shell.Inbox -----------------------------------

/// Object exposed at `/dev/mackes/MDE/Shell/Inbox`.
#[derive(Debug, Default, Clone)]
pub struct InboxService;

/// Stable D-Bus interface name.
pub const INBOX_INTERFACE: &str = "dev.mackes.MDE.Shell.Inbox";
/// Object path.
pub const INBOX_OBJECT_PATH: &str = "/dev/mackes/MDE/Shell/Inbox";

#[interface(name = "dev.mackes.MDE.Shell.Inbox")]
impl InboxService {
    /// JSON array of inbox `FileRow`s (newest first).
    async fn list(&self) -> zbus::fdo::Result<String> {
        Err(zbus::fdo::Error::Failed(
            "Inbox.List — wired in v2.0.0 Phase G".into(),
        ))
    }

    /// Mark one inbox entry as opened.
    async fn mark_opened(&self, _id: &str) -> zbus::fdo::Result<()> {
        Err(zbus::fdo::Error::Failed(
            "Inbox.MarkOpened — wired in v2.0.0 Phase G".into(),
        ))
    }

    /// Signal: a new inbox row landed (id, peer, label).
    #[zbus(signal)]
    pub async fn item_arrived(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        id: &str,
        peer: &str,
        label: &str,
    ) -> zbus::Result<()>;
}

// ---- dev.mackes.MDE.Shell.Outbox ----------------------------------

/// Object exposed at `/dev/mackes/MDE/Shell/Outbox`.
#[derive(Debug, Default, Clone)]
pub struct OutboxService;

pub const OUTBOX_INTERFACE: &str = "dev.mackes.MDE.Shell.Outbox";
pub const OUTBOX_OBJECT_PATH: &str = "/dev/mackes/MDE/Shell/Outbox";

#[interface(name = "dev.mackes.MDE.Shell.Outbox")]
impl OutboxService {
    /// JSON array of outbox `FileRow`s.
    async fn list(&self) -> zbus::fdo::Result<String> {
        Err(zbus::fdo::Error::Failed(
            "Outbox.List — wired in v2.0.0 Phase G".into(),
        ))
    }

    /// Cancel an in-flight upload by op_id.
    async fn cancel(&self, _op_id: u64) -> zbus::fdo::Result<()> {
        Err(zbus::fdo::Error::Failed(
            "Outbox.Cancel — wired in v2.0.0 Phase G".into(),
        ))
    }
}

// ---- dev.mackes.MDE.Shell.Downloads -------------------------------

/// Object exposed at `/dev/mackes/MDE/Shell/Downloads`.
#[derive(Debug, Default, Clone)]
pub struct DownloadsService;

pub const DOWNLOADS_INTERFACE: &str = "dev.mackes.MDE.Shell.Downloads";
pub const DOWNLOADS_OBJECT_PATH: &str = "/dev/mackes/MDE/Shell/Downloads";

#[interface(name = "dev.mackes.MDE.Shell.Downloads")]
impl DownloadsService {
    /// JSON array of completed downloads (newest first).
    async fn list(&self) -> zbus::fdo::Result<String> {
        Err(zbus::fdo::Error::Failed(
            "Downloads.List — wired in v2.0.0 Phase G".into(),
        ))
    }

    /// Reveal one download in the file manager.
    async fn reveal(&self, _id: &str) -> zbus::fdo::Result<()> {
        Err(zbus::fdo::Error::Failed(
            "Downloads.Reveal — wired in v2.0.0 Phase G".into(),
        ))
    }
}

// ---- dev.mackes.MDE.Shell.FileOperations --------------------------

/// Object exposed at `/dev/mackes/MDE/Shell/FileOperations`.
#[derive(Debug, Default, Clone)]
pub struct FileOperationsService;

pub const FILE_OPERATIONS_INTERFACE: &str = "dev.mackes.MDE.Shell.FileOperations";
pub const FILE_OPERATIONS_OBJECT_PATH: &str = "/dev/mackes/MDE/Shell/FileOperations";

#[interface(name = "dev.mackes.MDE.Shell.FileOperations")]
impl FileOperationsService {
    /// Send the given sources to one or more destinations. The
    /// `selector` is the same destination-grammar mde-files renders
    /// (peer:, group:, role:, site:). Returns the new op_id.
    async fn send_to(
        &self,
        _sources_json: &str,
        _selector: &str,
        _mode: &str,
        _conflict: &str,
    ) -> zbus::fdo::Result<u64> {
        Err(zbus::fdo::Error::Failed(
            "FileOperations.SendTo — wired in v2.0.0 Phase G".into(),
        ))
    }

    /// Roll back a completed op by op_id.
    async fn rollback(&self, _op_id: u64) -> zbus::fdo::Result<u64> {
        Err(zbus::fdo::Error::Failed(
            "FileOperations.Rollback — wired in v2.0.0 Phase G".into(),
        ))
    }

    /// JSON-encoded audit log (newest first, capped at `limit`).
    async fn audit_log(&self, _limit: u32) -> zbus::fdo::Result<String> {
        Err(zbus::fdo::Error::Failed(
            "FileOperations.AuditLog — wired in v2.0.0 Phase G".into(),
        ))
    }

    /// Signal: an op state changed (id, kind, ok).
    #[zbus(signal)]
    pub async fn op_completed(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        op_id: u64,
        kind: &str,
        ok: bool,
    ) -> zbus::Result<()>;
}

// ---- dev.mackes.MDE.Fleet.Files -----------------------------------

/// Object exposed at `/dev/mackes/MDE/Fleet/Files`.
#[derive(Debug, Default, Clone)]
pub struct FleetFilesService;

pub const FLEET_FILES_INTERFACE: &str = "dev.mackes.MDE.Fleet.Files";
pub const FLEET_FILES_OBJECT_PATH: &str = "/dev/mackes/MDE/Fleet/Files";

#[interface(name = "dev.mackes.MDE.Fleet.Files")]
impl FleetFilesService {
    /// JSON array of `Peer` rows from the live mesh roster.
    async fn peers(&self) -> zbus::fdo::Result<String> {
        Err(zbus::fdo::Error::Failed(
            "Fleet.Files.Peers — wired in v2.0.0 Phase G".into(),
        ))
    }

    /// JSON-encoded `SelfNode`.
    async fn self_node(&self) -> zbus::fdo::Result<String> {
        Err(zbus::fdo::Error::Failed(
            "Fleet.Files.SelfNode — wired in v2.0.0 Phase G".into(),
        ))
    }

    /// JSON array of `FileRow` entries visible under `peer:<name>`.
    async fn list_peer(&self, _peer: &str) -> zbus::fdo::Result<String> {
        Err(zbus::fdo::Error::Failed(
            "Fleet.Files.ListPeer — wired in v2.0.0 Phase G".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_inbox_interface_lock() {
        assert_eq!(INBOX_INTERFACE, "dev.mackes.MDE.Shell.Inbox");
        assert_eq!(INBOX_OBJECT_PATH, "/dev/mackes/MDE/Shell/Inbox");
    }

    #[test]
    fn shell_outbox_interface_lock() {
        assert_eq!(OUTBOX_INTERFACE, "dev.mackes.MDE.Shell.Outbox");
        assert_eq!(OUTBOX_OBJECT_PATH, "/dev/mackes/MDE/Shell/Outbox");
    }

    #[test]
    fn shell_downloads_interface_lock() {
        assert_eq!(DOWNLOADS_INTERFACE, "dev.mackes.MDE.Shell.Downloads");
        assert_eq!(DOWNLOADS_OBJECT_PATH, "/dev/mackes/MDE/Shell/Downloads");
    }

    #[test]
    fn shell_file_operations_interface_lock() {
        assert_eq!(
            FILE_OPERATIONS_INTERFACE,
            "dev.mackes.MDE.Shell.FileOperations"
        );
        assert_eq!(
            FILE_OPERATIONS_OBJECT_PATH,
            "/dev/mackes/MDE/Shell/FileOperations"
        );
    }

    #[test]
    fn fleet_files_interface_lock() {
        assert_eq!(FLEET_FILES_INTERFACE, "dev.mackes.MDE.Fleet.Files");
        assert_eq!(FLEET_FILES_OBJECT_PATH, "/dev/mackes/MDE/Fleet/Files");
    }

    #[tokio::test]
    async fn inbox_list_is_unimplemented_phase_a() {
        let s = InboxService;
        let err = s.list().await.unwrap_err();
        assert!(format!("{err}").contains("Phase G"));
    }

    #[tokio::test]
    async fn outbox_list_is_unimplemented_phase_a() {
        let s = OutboxService;
        let err = s.list().await.unwrap_err();
        assert!(format!("{err}").contains("Phase G"));
    }

    #[tokio::test]
    async fn downloads_list_is_unimplemented_phase_a() {
        let s = DownloadsService;
        let err = s.list().await.unwrap_err();
        assert!(format!("{err}").contains("Phase G"));
    }

    #[tokio::test]
    async fn file_ops_send_to_is_unimplemented_phase_a() {
        let s = FileOperationsService;
        let err = s.send_to("[]", "all", "copy", "ask").await.unwrap_err();
        assert!(format!("{err}").contains("Phase G"));
    }

    #[tokio::test]
    async fn fleet_files_peers_is_unimplemented_phase_a() {
        let s = FleetFilesService;
        let err = s.peers().await.unwrap_err();
        assert!(format!("{err}").contains("Phase G"));
    }
}
