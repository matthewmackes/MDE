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
}

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

/// v4.0.1 AF-2.3.a (2026-05-23) — file-backed settings store.
/// Persists every `set(key, value_json)` to
/// `$XDG_CONFIG_HOME/mde/workbench-settings.toml` (with a
/// fallback to `$HOME/.config/mde/`); reads happen against
/// the in-memory cache that's populated on construction.
/// Closes the half of AF-2.3.a that doesn't depend on mackesd:
/// settings PERSISTENCE. The cross-mesh PUSH half stays on
/// the DBusBackend route, captured as AF-2.3.b.
///
/// File format: TOML with `[settings]` table whose keys are
/// the same dot-notated setting names the API uses (e.g.
/// `theme.gtk = "Mackes-Dark"`, `font.body = "Geologica"`).
/// Values are stored as TOML strings carrying the JSON
/// serialization so the `value_json` contract from the
/// `Backend` trait round-trips losslessly.
pub struct FileBackend {
    path: std::path::PathBuf,
    values: Arc<Mutex<HashMap<String, String>>>,
}

impl fmt::Debug for FileBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self
            .values
            .lock()
            .map(|g| g.len())
            .unwrap_or(0);
        f.debug_struct("FileBackend")
            .field("path", &self.path)
            .field("keys", &n)
            .finish()
    }
}

impl FileBackend {
    /// Build a FileBackend rooted at the canonical
    /// `~/.config/mde/workbench-settings.toml` path. Loads any
    /// existing file content into the in-memory cache.
    #[must_use]
    pub fn new() -> Self {
        let path = default_settings_path();
        let values = match std::fs::read_to_string(&path) {
            Ok(raw) => Arc::new(Mutex::new(parse_settings(&raw))),
            Err(_) => Arc::new(Mutex::new(HashMap::new())),
        };
        Self { path, values }
    }

    /// Build a FileBackend at an explicit path — used by tests
    /// that need a writable tempfile.
    #[must_use]
    pub fn with_path(path: std::path::PathBuf) -> Self {
        let values = match std::fs::read_to_string(&path) {
            Ok(raw) => Arc::new(Mutex::new(parse_settings(&raw))),
            Err(_) => Arc::new(Mutex::new(HashMap::new())),
        };
        Self { path, values }
    }

    fn flush(&self, values: &HashMap<String, String>) -> Result<(), BackendError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                BackendError::Bus(format!("mkdir {}: {e}", parent.display()))
            })?;
        }
        let raw = serialize_settings(values);
        std::fs::write(&self.path, raw)
            .map_err(|e| BackendError::Bus(format!("write {}: {e}", self.path.display())))?;
        Ok(())
    }
}

impl Default for FileBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Backend for FileBackend {
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
        self.flush(&guard)?;
        Ok(())
    }
}

/// Canonical path for the workbench's persisted-settings file.
#[must_use]
pub fn default_settings_path() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
    base.join("mde").join("workbench-settings.toml")
}

/// Pure parser for the workbench-settings.toml shape. Returns
/// an empty map on garbage (so a corrupt file falls back to
/// defaults, the operator doesn't get a startup error).
#[must_use]
pub fn parse_settings(raw: &str) -> HashMap<String, String> {
    let value: toml::Value = match toml::from_str(raw) {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };
    let mut out = HashMap::new();
    if let Some(tbl) = value.get("settings").and_then(|v| v.as_table()) {
        for (k, v) in tbl {
            if let Some(s) = v.as_str() {
                out.insert(k.clone(), s.to_string());
            }
        }
    }
    out
}

/// Serialise the in-memory map back to TOML. Stable order by
/// key so diffs read cleanly.
#[must_use]
pub fn serialize_settings(values: &HashMap<String, String>) -> String {
    let mut keys: Vec<&String> = values.keys().collect();
    keys.sort();
    let mut out = String::from("# mde-workbench settings — written by AF-2.3.a FileBackend.\n");
    out.push_str("# Do not edit while mde-workbench is running; settings can race.\n\n");
    out.push_str("[settings]\n");
    for k in keys {
        let v = &values[k];
        // TOML basic-string escaping: " and \. Values are JSON
        // strings already so they may contain embedded quotes;
        // escape them.
        let escaped = v.replace('\\', "\\\\").replace('"', "\\\"");
        out.push_str(&format!("\"{k}\" = \"{escaped}\"\n"));
    }
    out
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

    #[test]
    fn parse_settings_handles_empty_input() {
        let m = parse_settings("");
        assert!(m.is_empty());
    }

    #[test]
    fn parse_settings_decodes_known_shape() {
        let raw = r#"
            [settings]
            "theme.gtk" = "\"Mackes-Dark\""
            "font.body" = "\"Geologica\""
        "#;
        let m = parse_settings(raw);
        assert_eq!(m.len(), 2);
        assert_eq!(m.get("theme.gtk").map(String::as_str), Some("\"Mackes-Dark\""));
        assert_eq!(m.get("font.body").map(String::as_str), Some("\"Geologica\""));
    }

    #[test]
    fn parse_settings_returns_empty_for_garbage() {
        assert!(parse_settings("not toml").is_empty());
    }

    #[test]
    fn serialize_settings_round_trips_through_parse() {
        let mut m = HashMap::new();
        m.insert("theme.gtk".to_string(), "\"Mackes-Dark\"".to_string());
        m.insert("font.body".to_string(), "\"Geologica\"".to_string());
        let raw = serialize_settings(&m);
        let back = parse_settings(&raw);
        assert_eq!(back, m);
    }

    #[test]
    fn serialize_settings_escapes_embedded_quotes() {
        let mut m = HashMap::new();
        m.insert("custom.key".to_string(), "{\"name\":\"with\\\"quotes\"}".to_string());
        let raw = serialize_settings(&m);
        // Round-trip should preserve the escaped JSON body.
        let back = parse_settings(&raw);
        assert_eq!(back.get("custom.key"), m.get("custom.key"));
    }

    #[tokio::test]
    async fn file_backend_persists_set_across_construction() {
        let tmp = std::env::temp_dir().join(format!(
            "mde-workbench-test-{}.toml",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&tmp);
        let backend = FileBackend::with_path(tmp.clone());
        backend.set("theme.gtk", "\"Mackes-Dark\"").await.expect("set");
        // Reconstructing from the same path reads the value back.
        let backend2 = FileBackend::with_path(tmp.clone());
        let got = backend2.get("theme.gtk").await.expect("get");
        assert_eq!(got, "\"Mackes-Dark\"");
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn file_backend_unknown_key_returns_empty_string() {
        let tmp = std::env::temp_dir().join(format!(
            "mde-workbench-empty-test-{}.toml",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&tmp);
        let backend = FileBackend::with_path(tmp.clone());
        let got = backend.get("nothing.here").await.expect("get");
        assert_eq!(got, "");
    }

    #[test]
    fn default_settings_path_lands_under_xdg_or_home() {
        let path = default_settings_path();
        // Must end with the canonical filename.
        assert_eq!(
            path.file_name().and_then(|s| s.to_str()),
            Some("workbench-settings.toml")
        );
        // Parent must contain "mde".
        let parent = path.parent().unwrap();
        assert!(parent.ends_with("mde"));
    }
}
