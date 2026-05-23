//! Backend abstraction over `dev.mackes.MDE.Settings.Get/Set`.
//!
//! Panels call into a `Arc<dyn Backend>` rather than zbus
//! directly so unit tests can substitute [`DemoBackend`] (an
//! in-memory HashMap) for the real [`DBusBackend`] (live zbus
//! to mackesd). Matches the mde-files Phase 2.1 pattern.
//!
//! CB-1.6 lock: Iced Look & Feel panels read + write `theme.*`
//! and `font.*` keys via `dev.mackes.MDE.Settings`. The
//! interface (in `crates/mackesd/src/ipc/settings.rs`)
//! already ships the Get/Set/Snapshot/Restore/ListKeys
//! methods; this module is the workbench-side adapter.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use zbus::Connection;

/// Errors a [`Backend`] call can return. Kept narrow on
/// purpose — the panel layer maps everything onto a generic
/// "couldn't reach mded" toast rather than discriminating
/// per-fault.
#[derive(Debug, Clone)]
pub enum BackendError {
    /// Setting key isn't registered (DemoBackend) or
    /// `dev.mackes.MDE.Settings.Get` returned a Failed reply.
    UnknownKey(String),
    /// Bus call failed (connection lost, method timeout,
    /// service unavailable). Carries the upstream message
    /// so the panel can surface it in an error state.
    Bus(String),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownKey(k) => write!(f, "unknown setting key: {k}"),
            Self::Bus(msg) => write!(f, "bus error: {msg}"),
        }
    }
}

impl std::error::Error for BackendError {}

/// Async settings backend. Implementations need to be `Send +
/// Sync` because Iced runs the reducer on its own task pool.
#[async_trait]
pub trait Backend: Send + Sync + 'static {
    /// Read the JSON-encoded value for `key`. Empty string is
    /// a valid return when the key is unset (e.g. fresh
    /// install before any apply lands).
    async fn get(&self, key: &str) -> Result<String, BackendError>;

    /// Write `value_json` for `key`. The Phase C appliers run
    /// the side effect (gsettings call, fontconfig rewrite,
    /// etc.) inside `dev.mackes.MDE.Settings.Set`.
    async fn set(&self, key: &str, value_json: &str) -> Result<(), BackendError>;

    /// Read the JSON-encoded `HealthReport` from
    /// `dev.mackes.MDE.Shell.healthz()`. Powers the Look & Feel
    /// → Panel sync-status section (v4.0.1 panel.toml task).
    /// The default impl returns the same `empty()` baseline the
    /// daemon prints when no live data is wired — so DemoBackend
    /// inherits a uniform "no daemon" shape without each impl
    /// hand-rolling the JSON.
    async fn healthz(&self) -> Result<String, BackendError> {
        Ok(EMPTY_HEALTHZ_JSON.to_owned())
    }
}

/// JSON-encoded `HealthReport::empty()` shape — kept here as a
/// string constant so the workbench doesn't need to depend on
/// `mackesd_core` to mint the baseline. Mirrors what
/// `mackesd_core::health::HealthReport::empty().to_json_line()`
/// produces.
pub const EMPTY_HEALTHZ_JSON: &str = "{\
\"schema\":1,\
\"is_leader\":false,\
\"applied_revision\":null,\
\"node_count\":0,\
\"healthy_nodes\":0,\
\"degraded_nodes\":0,\
\"unreachable_nodes\":0,\
\"audit_chain_intact\":true,\
\"version\":\"dev\"\
}";

/// In-memory backend used by unit tests + the workbench's
/// `--demo` invocation (CB-1.6 follow-up). Maintains the same
/// "everything is JSON" contract as the live backend.
#[derive(Debug, Clone, Default)]
pub struct DemoBackend {
    values: Arc<Mutex<HashMap<String, String>>>,
}

impl DemoBackend {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed the backend with a `(key, value_json)` map — useful
    /// for tests that need preset values before the first read.
    #[must_use]
    pub fn with_seed(seed: HashMap<String, String>) -> Self {
        Self {
            values: Arc::new(Mutex::new(seed)),
        }
    }
}

#[async_trait]
impl Backend for DemoBackend {
    async fn get(&self, key: &str) -> Result<String, BackendError> {
        Ok(self
            .values
            .lock()
            .map_err(|e| BackendError::Bus(format!("poisoned mutex: {e}")))?
            .get(key)
            .cloned()
            .unwrap_or_default())
    }

    async fn set(&self, key: &str, value_json: &str) -> Result<(), BackendError> {
        let mut guard = self
            .values
            .lock()
            .map_err(|e| BackendError::Bus(format!("poisoned mutex: {e}")))?;
        guard.insert(key.to_string(), value_json.to_string());
        Ok(())
    }
}

/// `dev.mackes.MDE.Settings` client proxy. Generated from the
/// same interface name + method signatures the service in
/// `crates/mackesd/src/ipc/settings.rs` exposes.
#[zbus::proxy(
    interface = "dev.mackes.MDE.Settings",
    default_service = "dev.mackes.MDE.Settings",
    default_path = "/dev/mackes/MDE/Settings"
)]
trait Settings {
    fn get(&self, key: &str) -> zbus::Result<String>;
    fn set(&self, key: &str, value_json: &str) -> zbus::Result<()>;
}

/// `dev.mackes.MDE.Shell` client proxy — same shape as
/// `crates/mackesd/src/ipc/shell.rs`'s `ShellService`. Only the
/// `healthz` method is wired today; `workers` lands once a panel
/// surface needs it.
#[zbus::proxy(
    interface = "dev.mackes.MDE.Shell",
    default_service = "dev.mackes.MDE.Shell",
    default_path = "/dev/mackes/MDE/Shell"
)]
trait Shell {
    fn healthz(&self) -> zbus::Result<String>;
}

/// Live backend that talks to mackesd over the session bus.
/// Holds an `Arc<Connection>` so panels can clone the backend
/// cheaply into `Task::perform` futures.
#[derive(Clone)]
pub struct DBusBackend {
    conn: Arc<Connection>,
}

impl fmt::Debug for DBusBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DBusBackend").finish_non_exhaustive()
    }
}

impl DBusBackend {
    #[must_use]
    pub fn new(conn: Arc<Connection>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl Backend for DBusBackend {
    async fn get(&self, key: &str) -> Result<String, BackendError> {
        let proxy = SettingsProxy::new(&self.conn)
            .await
            .map_err(|e| BackendError::Bus(e.to_string()))?;
        proxy
            .get(key)
            .await
            .map_err(|e| BackendError::Bus(e.to_string()))
    }

    async fn set(&self, key: &str, value_json: &str) -> Result<(), BackendError> {
        let proxy = SettingsProxy::new(&self.conn)
            .await
            .map_err(|e| BackendError::Bus(e.to_string()))?;
        proxy
            .set(key, value_json)
            .await
            .map_err(|e| BackendError::Bus(e.to_string()))
    }

    async fn healthz(&self) -> Result<String, BackendError> {
        let proxy = ShellProxy::new(&self.conn)
            .await
            .map_err(|e| BackendError::Bus(e.to_string()))?;
        proxy
            .healthz()
            .await
            .map_err(|e| BackendError::Bus(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn demo_get_returns_empty_string_for_unset_key() {
        let backend = DemoBackend::new();
        assert_eq!(backend.get("theme.name").await.unwrap(), "");
    }

    #[tokio::test]
    async fn demo_set_then_get_round_trips() {
        let backend = DemoBackend::new();
        backend
            .set("theme.name", "\"Adwaita-dark\"")
            .await
            .expect("set ok");
        assert_eq!(backend.get("theme.name").await.unwrap(), "\"Adwaita-dark\"");
    }

    #[tokio::test]
    async fn demo_set_overwrites_existing_value() {
        let backend = DemoBackend::new();
        backend.set("font.name", "\"Inter 11\"").await.unwrap();
        backend.set("font.name", "\"Cantarell 10\"").await.unwrap();
        assert_eq!(backend.get("font.name").await.unwrap(), "\"Cantarell 10\"");
    }

    #[tokio::test]
    async fn demo_with_seed_preloads_values() {
        let mut seed = HashMap::new();
        seed.insert("theme.mode".to_string(), "\"dark\"".to_string());
        let backend = DemoBackend::with_seed(seed);
        assert_eq!(backend.get("theme.mode").await.unwrap(), "\"dark\"");
    }

    #[test]
    fn backend_error_display_is_human_readable() {
        let unk = BackendError::UnknownKey("theme.ghost".into());
        assert!(format!("{unk}").contains("theme.ghost"));
        let bus = BackendError::Bus("timed out".into());
        assert!(format!("{bus}").contains("timed out"));
    }

    #[test]
    fn backend_object_is_send_sync() {
        // Trait-object safety guard — Arc<dyn Backend> is what
        // App stores and Task::perform clones across the iced
        // executor boundary. Compile-time check.
        fn _assert_send_sync<T: Send + Sync + ?Sized>() {}
        _assert_send_sync::<dyn Backend>();
    }

    #[tokio::test]
    async fn demo_backend_clone_shares_underlying_storage() {
        let backend = DemoBackend::new();
        let clone = backend.clone();
        backend.set("theme.mode", "\"auto\"").await.unwrap();
        assert_eq!(clone.get("theme.mode").await.unwrap(), "\"auto\"");
    }

    #[tokio::test]
    async fn default_healthz_returns_empty_baseline_json() {
        let backend = DemoBackend::new();
        let line = backend.healthz().await.unwrap();
        let v: serde_json::Value = serde_json::from_str(&line).expect("parse healthz JSON");
        assert_eq!(v["schema"], 1);
        assert_eq!(v["node_count"], 0);
        assert_eq!(v["is_leader"], false);
        assert!(v["applied_revision"].is_null());
        assert_eq!(v["audit_chain_intact"], true);
    }

    #[test]
    fn empty_healthz_json_constant_parses_clean() {
        let v: serde_json::Value =
            serde_json::from_str(EMPTY_HEALTHZ_JSON).expect("parse empty healthz");
        // Every documented field present, no typos.
        for k in [
            "schema",
            "is_leader",
            "applied_revision",
            "node_count",
            "healthy_nodes",
            "degraded_nodes",
            "unreachable_nodes",
            "audit_chain_intact",
            "version",
        ] {
            assert!(v.get(k).is_some(), "missing key: {k}");
        }
    }
}
