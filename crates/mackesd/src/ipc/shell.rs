//! `dev.mackes.MDE.Shell` — top-level shell control (health, version,
//! worker pool status). Phase A: schema only.
//!
//! v2.0.0 Phase 0.4 rebrand — interface name moved from
//! `org.mackes.Shell` to `dev.mackes.MDE.Shell`. Backward-compat
//! alias .service file ships under the old name for one release; see
//! `data/dbus-1/services/`.

#![cfg(feature = "async-services")]

use zbus::interface;

/// Object exposed at `/dev/mackes/MDE/Shell`.
#[derive(Debug, Default, Clone)]
pub struct ShellService;

/// Stable D-Bus name used by Phase 0.4-onward callers. The legacy
/// `org.mackes.Shell` alias ships through one v2.x line for
/// backward-compat.
pub const SERVICE_NAME: &str = "dev.mackes.MDE.Shell";

/// Object-path under [`SERVICE_NAME`]. Matches the
/// reverse-slash convention zbus picks by default.
pub const OBJECT_PATH: &str = "/dev/mackes/MDE/Shell";

#[interface(name = "dev.mackes.MDE.Shell")]
impl ShellService {
    /// Compiled crate version (`CARGO_PKG_VERSION`).
    async fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// JSON-encoded [`crate::health::HealthReport`]. Today returns
    /// the same `HealthReport::empty()` baseline the `mackesd healthz`
    /// CLI prints so the workbench's panel-sync surface can render
    /// consistently between the two consumers. Live-supervisor
    /// populated fields ride downstream wire-up in
    /// `mackesd_core::healthz_live` (12.3.3 heartbeats, 12.5.1 drift
    /// detector).
    async fn healthz(&self) -> zbus::fdo::Result<String> {
        let report = crate::health::HealthReport::empty();
        report
            .to_json_line()
            .map_err(|e| zbus::fdo::Error::Failed(format!("healthz serialize: {e}")))
    }

    /// List currently-spawned worker names. Returns the static set
    /// every `mackesd serve` invocation registers today (see
    /// `bin/mackesd.rs`'s supervisor block); per-instance dynamic
    /// status rides the live supervisor handle in a follow-up.
    async fn workers(&self) -> zbus::fdo::Result<Vec<String>> {
        Ok(vec![
            "clipboard".into(),
            "mdns".into(),
            "fs_sync".into(),
            "heartbeat".into(),
            "mesh_router".into(),
            "stun_gather".into(),
            "notification_relay".into(),
            "kdc_host".into(),
        ])
    }
}

/// Register the [`ShellService`] on the session bus at the canonical
/// well-known name + object path. The returned [`zbus::Connection`]
/// must stay alive for the daemon's lifetime.
///
/// Workbench's panel-sync surface (v4.0.1 panel.toml status section)
/// calls `dev.mackes.MDE.Shell.healthz()` on this bus name to render
/// "Synced to revision N at HH:MM by peer-X".
///
/// # Errors
///
/// Returns whatever zbus reports (typical failure mode is
/// `NameAlreadyAcquired` if another mackesd is already running on the
/// same session bus; callers degrade gracefully).
pub async fn register_shell(state: ShellService) -> zbus::Result<zbus::Connection> {
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
    async fn version_matches_crate() {
        let svc = ShellService;
        assert_eq!(svc.version().await, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn service_name_carries_mde_namespace() {
        assert_eq!(SERVICE_NAME, "dev.mackes.MDE.Shell");
        assert!(SERVICE_NAME.starts_with("dev.mackes.MDE."));
    }

    #[test]
    fn object_path_mirrors_service_name_segments() {
        assert_eq!(OBJECT_PATH, "/dev/mackes/MDE/Shell");
    }

    #[tokio::test]
    async fn healthz_returns_json_health_report() {
        let svc = ShellService;
        let line = svc.healthz().await.expect("healthz");
        let report: crate::health::HealthReport =
            serde_json::from_str(&line).expect("parse");
        assert_eq!(report.schema, crate::health::HealthReport::CURRENT_SCHEMA);
        assert_eq!(report.version, env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn workers_lists_supervisor_worker_set() {
        let svc = ShellService;
        let names = svc.workers().await.expect("workers");
        assert!(names.iter().any(|n| n == "fs_sync"));
        assert!(names.iter().any(|n| n == "mesh_router"));
        assert!(names.iter().any(|n| n == "kdc_host"));
    }
}
