//! Health surface (Phase 12.1.3).
//!
//! `HealthReport` is the value type returned by `mackesd healthz`
//! (CLI subcommand) and `mackesd_core::healthz()` (library function
//! the panel imports for the status cluster).
//!
//! Per the 12.1.3 lock the same data surfaces in both places — the
//! CLI prints it as JSON, the library returns the typed struct.

use serde::{Deserialize, Serialize};

/// Top-level health report. Each field is independently reportable
/// so a probe failure on one doesn't poison the others.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Schema version. Bump when the shape changes; the panel uses
    /// this to fall back gracefully if a newer `mackesd` reports a
    /// shape it doesn't recognize yet.
    pub schema: u32,
    /// Is this peer currently the leader? See 12.A.5.
    pub is_leader: bool,
    /// Most recent applied revision (`r-YYYY-MM-DD-NNNN` form).
    /// `None` when the store has never accepted a deploy.
    pub applied_revision: Option<String>,
    /// Count of rows in the `nodes` table (mesh size from this peer's
    /// perspective).
    pub node_count: u32,
    /// Count of rows whose `last_heartbeat` is within the healthy
    /// threshold (per 12.3.3).
    pub healthy_nodes: u32,
    /// Count of rows whose `last_heartbeat` missed exactly one cycle.
    pub degraded_nodes: u32,
    /// Count of rows whose `last_heartbeat` missed 3+ cycles.
    pub unreachable_nodes: u32,
    /// Audit chain status. `true` = `audit::verify()` returned
    /// `Intact`. `false` = the most recent verify reported a break.
    pub audit_chain_intact: bool,
    /// Mackesd version (Cargo package version).
    pub version: String,
}

impl HealthReport {
    /// Current schema version. Bump alongside any breaking field
    /// change. Add a fallback path on the panel side before bumping
    /// so older readers degrade gracefully.
    pub const CURRENT_SCHEMA: u32 = 1;

    /// Build a default report for a fresh peer that has no data
    /// yet. Used by `mackesd healthz` on a just-installed system
    /// before the first reconcile tick.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            schema: Self::CURRENT_SCHEMA,
            is_leader: false,
            applied_revision: None,
            node_count: 0,
            healthy_nodes: 0,
            degraded_nodes: 0,
            unreachable_nodes: 0,
            audit_chain_intact: true,
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }

    /// JSON one-liner for `mackesd healthz`. Stable shape — every
    /// field always present, no schema-conditional keys.
    ///
    /// # Errors
    /// Returns `serde_json::Error` only on out-of-memory while
    /// serializing — never on schema-shape issues.
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Build a report from the live SQLite store. Reads the most-
    /// recent revision whose state is `applied` (or one of the
    /// downstream-applied states), counts rows in `nodes` keyed by
    /// the `health` column, and returns the shape `mackesd healthz`
    /// + `dev.mackes.MDE.Shell.healthz()` both serve.
    ///
    /// Leadership + audit-chain status are best-effort: the report
    /// degrades gracefully (`is_leader: false`, `audit_chain_intact:
    /// true`) when the live signals aren't observable from a read-
    /// only store handle. Downstream wire-up (12.3.3 leader, 12.6.3
    /// audit) updates those two fields when their workers ship.
    ///
    /// # Errors
    ///
    /// Returns whatever rusqlite reports when the schema queries
    /// fail. A fresh-installed store with no rows is not an error
    /// — every count is just `0`.
    pub fn compute(conn: &rusqlite::Connection) -> rusqlite::Result<Self> {
        let applied_revision: Option<String> = conn
            .query_row(
                "SELECT revision_id FROM desired_config \
                 WHERE state IN ('applied', 'verified') \
                 ORDER BY revision_id DESC LIMIT 1",
                [],
                |r| r.get::<_, i64>(0).map(|n| n.to_string()),
            )
            .ok();

        let node_count: u32 = conn
            .query_row("SELECT COUNT(*) FROM nodes", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0)
            .try_into()
            .unwrap_or(u32::MAX);

        let healthy_nodes: u32 = count_nodes_by_health(conn, "healthy");
        let degraded_nodes: u32 = count_nodes_by_health(conn, "degraded");
        let unreachable_nodes: u32 = count_nodes_by_health(conn, "unreachable");

        Ok(Self {
            schema: Self::CURRENT_SCHEMA,
            is_leader: false,
            applied_revision,
            node_count,
            healthy_nodes,
            degraded_nodes,
            unreachable_nodes,
            audit_chain_intact: true,
            version: env!("CARGO_PKG_VERSION").to_owned(),
        })
    }
}

fn count_nodes_by_health(conn: &rusqlite::Connection, label: &str) -> u32 {
    conn.query_row(
        "SELECT COUNT(*) FROM nodes WHERE health = ?",
        [label],
        |r| r.get::<_, i64>(0),
    )
    .unwrap_or(0)
    .try_into()
    .unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_report_has_schema_1() {
        let r = HealthReport::empty();
        assert_eq!(r.schema, 1);
        assert!(!r.is_leader);
        assert!(r.applied_revision.is_none());
        assert_eq!(r.node_count, 0);
    }

    #[test]
    fn json_round_trips() {
        let r = HealthReport::empty();
        let line = r.to_json_line().expect("serialize");
        let back: HealthReport = serde_json::from_str(&line).expect("parse");
        assert_eq!(back.schema, r.schema);
        assert_eq!(back.is_leader, r.is_leader);
        assert_eq!(back.version, r.version);
    }

    #[test]
    fn version_string_matches_cargo() {
        let r = HealthReport::empty();
        assert_eq!(r.version, env!("CARGO_PKG_VERSION"));
    }

    fn open_memdb() -> rusqlite::Connection {
        crate::store::open_in_memory().expect("open in-memory db")
    }

    #[test]
    fn compute_on_fresh_db_returns_empty_baseline() {
        let conn = open_memdb();
        let r = HealthReport::compute(&conn).expect("compute");
        assert_eq!(r.schema, HealthReport::CURRENT_SCHEMA);
        assert_eq!(r.node_count, 0);
        assert_eq!(r.healthy_nodes, 0);
        assert_eq!(r.degraded_nodes, 0);
        assert_eq!(r.unreachable_nodes, 0);
        assert!(r.applied_revision.is_none());
        assert!(!r.is_leader);
        assert!(r.audit_chain_intact);
    }

    #[test]
    fn compute_counts_nodes_by_health_label() {
        let conn = open_memdb();
        // Three nodes: one healthy, two degraded, one unreachable.
        // Use direct SQL because the higher-level enrollment APIs
        // pull in network state.
        for (id, health) in [
            ("n-1", "healthy"),
            ("n-2", "degraded"),
            ("n-3", "degraded"),
            ("n-4", "unreachable"),
        ] {
            conn.execute(
                "INSERT INTO nodes (node_id, name, public_key, enrolled_at, health) \
                 VALUES (?, ?, 'pk', '2026-05-23T00:00:00Z', ?)",
                [id, id, health],
            )
            .expect("insert node");
        }
        let r = HealthReport::compute(&conn).expect("compute");
        assert_eq!(r.node_count, 4);
        assert_eq!(r.healthy_nodes, 1);
        assert_eq!(r.degraded_nodes, 2);
        assert_eq!(r.unreachable_nodes, 1);
    }

    #[test]
    fn compute_picks_most_recent_applied_revision() {
        let conn = open_memdb();
        // Insert 3 revisions: draft / applied / verified. The most-
        // recent (verified by id-DESC) should win.
        conn.execute(
            "INSERT INTO desired_config (author, message, spec_json, state, created_at) \
             VALUES ('op', 'm', '{}', 'draft', '2026-05-23T00:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO desired_config (author, message, spec_json, state, created_at) \
             VALUES ('op', 'm', '{}', 'applied', '2026-05-23T01:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO desired_config (author, message, spec_json, state, created_at) \
             VALUES ('op', 'm', '{}', 'verified', '2026-05-23T02:00:00Z')",
            [],
        )
        .unwrap();
        let r = HealthReport::compute(&conn).expect("compute");
        // Most-recent by revision_id (3) is the verified one.
        assert_eq!(r.applied_revision.as_deref(), Some("3"));
    }

    #[test]
    fn compute_skips_draft_only_revisions() {
        let conn = open_memdb();
        conn.execute(
            "INSERT INTO desired_config (author, message, spec_json, state, created_at) \
             VALUES ('op', 'm', '{}', 'draft', '2026-05-23T00:00:00Z')",
            [],
        )
        .unwrap();
        let r = HealthReport::compute(&conn).expect("compute");
        // Draft alone doesn't count as applied.
        assert!(r.applied_revision.is_none());
    }
}
