//! `dev.mackes.MDE.Fleet` — fleet control (push setting revisions,
//! list revisions, rollback) served by mackesd.
//!
//! Phase A ships the schema; Phase G (`v2.0.0`) wires it through to
//! the reconcile loop + the `settings` table.
//!
//! v2.0.0 Phase 0.4 rebrand — interface name moved from
//! `org.mackes.Fleet`. Backward-compat alias .service file ships
//! under the old name for one release; see `data/dbus-1/services/`.

#![cfg(feature = "async-services")]

use std::path::PathBuf;

use zbus::interface;

/// Object exposed at `/dev/mackes/MDE/Fleet`.
///
/// Holds the SQLite store path so `list_revisions()` can read from
/// the live `desired_config` table. The default impl (no path)
/// still answers with an empty list so unit tests work without a
/// store on disk.
#[derive(Debug, Default, Clone)]
pub struct FleetService {
    db_path: Option<PathBuf>,
}

impl FleetService {
    /// Bind a live SQLite store path so `list_revisions()` reads
    /// rows instead of returning the empty list.
    #[must_use]
    pub fn with_db_path(mut self, db_path: PathBuf) -> Self {
        self.db_path = Some(db_path);
        self
    }
}

/// Stable D-Bus name used by Phase 0.4-onward callers.
pub const SERVICE_NAME: &str = "dev.mackes.MDE.Fleet";

/// Object-path under [`SERVICE_NAME`].
pub const OBJECT_PATH: &str = "/dev/mackes/MDE/Fleet";

#[interface(name = "dev.mackes.MDE.Fleet")]
impl FleetService {
    /// Push a new desired-config revision targeting a set of peers.
    /// `peers_selector` follows the same grammar as
    /// `mackesd fleet push-setting … --peers <sel>` (e.g.
    /// `"all"`, `"region:lab"`, `"node:laptop-01,desktop-02"`).
    /// Returns the new revision id (`r-YYYY-MM-DD-NNNN`).
    async fn push_revision(
        &self,
        _settings_json: &str,
        _peers_selector: &str,
    ) -> zbus::fdo::Result<String> {
        Err(zbus::fdo::Error::Failed(
            "Fleet.PushRevision — not implemented until v2.0.0 Phase G".into(),
        ))
    }

    /// List revision IDs in descending chronological order. Each
    /// element is the JSON encoding of one [`crate::revisions::RevisionSummary`]
    /// — the panel deserialises and renders the table. `limit` of
    /// 0 means "no cap" (small fleets); positive values cap the
    /// reply so a 10000-row store doesn't blow the bus message
    /// size budget.
    async fn list_revisions(&self, limit: u32) -> zbus::fdo::Result<Vec<String>> {
        let Some(path) = &self.db_path else {
            return Ok(Vec::new());
        };
        let conn = crate::store::open(path)
            .map_err(|e| zbus::fdo::Error::Failed(format!("open store: {e:#}")))?;
        let rows = crate::revisions::list_summaries(&conn, limit)
            .map_err(|e| zbus::fdo::Error::Failed(format!("list revisions: {e}")))?;
        rows.into_iter()
            .map(|r| serde_json::to_string(&r))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| zbus::fdo::Error::Failed(format!("serialize summary: {e}")))
    }

    /// Diff two revisions. Returns a JSON-encoded RevisionDiff.
    async fn diff_revisions(&self, _from: &str, _to: &str) -> zbus::fdo::Result<String> {
        Err(zbus::fdo::Error::Failed(
            "Fleet.DiffRevisions — not implemented until v2.0.0 Phase G".into(),
        ))
    }

    /// Rollback to a given revision (fleet-wide or per-peer based on
    /// selector grammar).
    async fn rollback(&self, _revision_id: &str, _peers_selector: &str) -> zbus::fdo::Result<()> {
        Err(zbus::fdo::Error::Failed(
            "Fleet.Rollback — not implemented until v2.0.0 Phase G".into(),
        ))
    }

    /// Signal: a fleet revision has been applied on this peer.
    #[zbus(signal)]
    pub async fn revision_applied(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        revision_id: &str,
    ) -> zbus::Result<()>;
}

/// Register the [`FleetService`] on the session bus at the canonical
/// well-known name + object path. The returned [`zbus::Connection`]
/// must stay alive for the daemon's lifetime.
///
/// # Errors
///
/// Returns whatever zbus reports (typical failure mode is
/// `NameAlreadyAcquired` if another mackesd is already running on
/// the same session bus; callers degrade gracefully).
pub async fn register_fleet(state: FleetService) -> zbus::Result<zbus::Connection> {
    zbus::connection::Builder::session()?
        .name(SERVICE_NAME)?
        .serve_at(OBJECT_PATH, state)?
        .build()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_revisions_without_db_path_returns_empty_vec() {
        let svc = FleetService::default();
        let rows = svc.list_revisions(0).await.expect("empty list ok");
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn list_revisions_with_db_path_serializes_summaries() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let db_path = tmp.path().join("mackesd.db");
        // Seed two rows so we can lock the descending-id order.
        {
            let conn = crate::store::open(&db_path).expect("open store");
            conn.execute(
                "INSERT INTO desired_config (author, message, spec_json, state, created_at) \
                 VALUES ('alice', 'first', '{}', 'applied', '2026-05-23T00:00:00Z')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO desired_config (author, message, spec_json, state, created_at) \
                 VALUES ('bob', 'second', '{}', 'draft', '2026-05-23T01:00:00Z')",
                [],
            )
            .unwrap();
        }
        let svc = FleetService::default().with_db_path(db_path);
        let rows = svc.list_revisions(0).await.expect("ok");
        assert_eq!(rows.len(), 2);
        let first: crate::revisions::RevisionSummary =
            serde_json::from_str(&rows[0]).expect("parse 0");
        assert_eq!(first.revision_id, "2");
        assert_eq!(first.author, "bob");
        let second: crate::revisions::RevisionSummary =
            serde_json::from_str(&rows[1]).expect("parse 1");
        assert_eq!(second.revision_id, "1");
        assert_eq!(second.author, "alice");
    }

    #[tokio::test]
    async fn list_revisions_limit_caps_reply_length() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let db_path = tmp.path().join("mackesd.db");
        {
            let conn = crate::store::open(&db_path).expect("open store");
            for i in 0..5 {
                conn.execute(
                    "INSERT INTO desired_config (author, message, spec_json, state, created_at) \
                     VALUES (?, 'x', '{}', 'draft', '2026-05-23T00:00:00Z')",
                    [format!("user-{i}")],
                )
                .unwrap();
            }
        }
        let svc = FleetService::default().with_db_path(db_path);
        let rows = svc.list_revisions(2).await.expect("ok");
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn service_name_carries_mde_namespace() {
        assert_eq!(SERVICE_NAME, "dev.mackes.MDE.Fleet");
        assert!(SERVICE_NAME.starts_with("dev.mackes.MDE."));
    }
}
