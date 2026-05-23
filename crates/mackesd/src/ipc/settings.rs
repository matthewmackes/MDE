//! `dev.mackes.MDE.Settings` — DBus surface for the settings store.
//!
//! Phase A ships the interface decoration and a service struct that
//! holds no state yet. Phase C wires it through to
//! `crate::settings::{apply, current}` + the SQLite `settings` table.
//!
//! v2.0.0 Phase 0.4 rebrand — interface name moved from
//! `org.mackes.Settings`. Backward-compat alias .service file ships
//! under the old name for one release; see `data/dbus-1/services/`.

#![cfg(feature = "async-services")]

use zbus::interface;

/// Object exposed at `/dev/mackes/MDE/Settings` on the session bus.
#[derive(Debug, Default, Clone)]
pub struct SettingsService;

/// Stable D-Bus name used by Phase 0.4-onward callers.
pub const SERVICE_NAME: &str = "dev.mackes.MDE.Settings";

/// Object-path under [`SERVICE_NAME`].
pub const OBJECT_PATH: &str = "/dev/mackes/MDE/Settings";

#[interface(name = "dev.mackes.MDE.Settings")]
impl SettingsService {
    /// Read a setting by dot-notated key. Returns the JSON-encoded
    /// value as a string. v2.0.0 Phase C.10 — wired through to
    /// `crate::settings::current()`.
    async fn get(&self, key: &str) -> zbus::fdo::Result<String> {
        let parsed: crate::settings::SettingKey = key
            .parse()
            .map_err(|e| zbus::fdo::Error::InvalidArgs(format!("{e}")))?;
        let value = crate::settings::current(parsed)
            .map_err(|e| zbus::fdo::Error::Failed(format!("{e:#}")))?;
        serde_json::to_string(&value).map_err(|e| zbus::fdo::Error::Failed(format!("ser: {e}")))
    }

    /// Write a setting by dot-notated key. `value_json` is the
    /// JSON-encoded payload. Routes through `crate::settings::apply()`
    /// which validates value shape, persists, and runs the applier.
    async fn set(&self, key: &str, value_json: &str) -> zbus::fdo::Result<()> {
        let parsed_key: crate::settings::SettingKey = key
            .parse()
            .map_err(|e| zbus::fdo::Error::InvalidArgs(format!("{e}")))?;
        let value: crate::settings::SettingValue = serde_json::from_str(value_json)
            .map_err(|e| zbus::fdo::Error::InvalidArgs(format!("value_json: {e}")))?;
        crate::settings::apply(parsed_key, &value)
            .map_err(|e| zbus::fdo::Error::Failed(format!("{e:#}")))
    }

    /// Enumerate every known setting key (dot-notated string form).
    /// Returns immediately — no DB hit, no I/O.
    async fn list_keys(&self) -> Vec<String> {
        crate::settings::SettingKey::all()
            .iter()
            .map(|k| k.as_str().to_string())
            .collect()
    }

    /// Snapshot every current value. Returns a JSON-encoded
    /// [`crate::settings::Snapshot`] suitable for round-tripping
    /// through [`Self::restore`]. Best-effort: keys whose `current()`
    /// errors (e.g. brightnessctl missing in a container) are
    /// skipped silently.
    async fn snapshot(&self) -> zbus::fdo::Result<String> {
        let mut snap = crate::settings::Snapshot::default();
        for &key in crate::settings::SettingKey::all() {
            if let Ok(v) = crate::settings::current(key) {
                snap.values.insert(key.as_str().to_string(), v);
            }
        }
        snap.captured_at = Some(chrono::Utc::now());
        serde_json::to_string_pretty(&snap)
            .map_err(|e| zbus::fdo::Error::Failed(format!("ser: {e}")))
    }

    /// Restore from a snapshot JSON. Each value re-applies through
    /// `crate::settings::apply()`; the first failure aborts the
    /// restore (so the operator sees an actionable error).
    async fn restore(&self, snapshot_json: &str) -> zbus::fdo::Result<()> {
        let snap: crate::settings::Snapshot = serde_json::from_str(snapshot_json)
            .map_err(|e| zbus::fdo::Error::InvalidArgs(format!("snapshot_json: {e}")))?;
        for (key_str, value) in &snap.values {
            let key: crate::settings::SettingKey = key_str
                .parse()
                .map_err(|e| zbus::fdo::Error::InvalidArgs(format!("{e}")))?;
            crate::settings::apply(key, value)
                .map_err(|e| zbus::fdo::Error::Failed(format!("{key_str}: {e:#}")))?;
        }
        Ok(())
    }

    /// Signal: a setting changed. `key` is the dot-notated key.
    /// Emitted by the applier path after a successful Set or
    /// reconcile push.
    #[zbus(signal)]
    pub async fn changed(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        key: &str,
    ) -> zbus::Result<()>;
}

/// Register the [`SettingsService`] on the session bus at the
/// canonical well-known name + object path. The returned
/// [`zbus::Connection`] must stay alive for the daemon's lifetime
/// — drop it and the dbus surface goes away.
///
/// AF-2.3.a wire-up (2026-05-23): the per-user mackesd needs to
/// own this name so the workbench's
/// [`mde_workbench::DBusBackend`] can land `Get`/`Set` calls on
/// `dev.mackes.MDE.Settings` rather than no-owner-of-name
/// failures.
///
/// # Errors
///
/// Returns whatever zbus reports (typical failure mode is
/// `NameAlreadyAcquired` if another mackesd is already running on
/// the same session bus; callers degrade gracefully).
pub async fn register_settings(state: SettingsService) -> zbus::Result<zbus::Connection> {
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
    async fn list_keys_returns_every_setting_key() {
        let svc = SettingsService::default();
        let keys = svc.list_keys().await;
        assert_eq!(keys.len(), crate::settings::SettingKey::all().len());
        assert!(keys.iter().any(|k| k == "theme.accent"));
        assert!(keys.iter().any(|k| k == "power.profile"));
    }

    #[tokio::test]
    async fn get_rejects_unknown_key() {
        let svc = SettingsService::default();
        let err = svc.get("never.a.real.key").await.unwrap_err();
        assert!(format!("{err}").contains("unknown setting key"));
    }

    #[tokio::test]
    async fn set_rejects_malformed_value_json() {
        let svc = SettingsService::default();
        let err = svc.set("theme.name", "{not json}").await.unwrap_err();
        assert!(format!("{err}").contains("value_json"));
    }

    #[tokio::test]
    async fn service_name_and_object_path_constants() {
        assert_eq!(SERVICE_NAME, "dev.mackes.MDE.Settings");
        assert_eq!(OBJECT_PATH, "/dev/mackes/MDE/Settings");
    }
}
