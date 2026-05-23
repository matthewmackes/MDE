//! Session-prefs applier — AF-2.3.a follow-on (2026-05-23).
//!
//! Workbench writes `session.save_on_exit`, `session.lock_on_suspend`,
//! and `session.auto_save` via `dev.mackes.MDE.Settings.Set`. The
//! applier persists each boolean into `~/.cache/mde/session-prefs.json`
//! (the Phase F.6 sidecar contract documented on the workbench panel)
//! so `mde-session` can read it back on next login.
//!
//! The JSON file is the source of truth; `current()` reads it back so
//! the Workbench picker shows what's actually persisted rather than
//! whatever's in the SQLite `settings` row.
//!
//! XDG cache home resolution mirrors `autostart.rs`'s pattern — honor
//! `$XDG_CACHE_HOME`, fall back to `$HOME/.cache`, fall back to `.`.

use std::path::PathBuf;

use super::{SettingKey, SettingValue};

/// The on-disk shape `mde-session` reads at login.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub struct SessionPrefs {
    /// Save the running session (open windows / apps) on logout.
    #[serde(default)]
    pub save_on_exit: bool,
    /// Lock the screen automatically when the system suspends.
    #[serde(default)]
    pub lock_on_suspend: bool,
    /// Snapshot the session every N seconds during the running login
    /// so a crash can be recovered.
    #[serde(default)]
    pub auto_save: bool,
}

/// Resolve `~/.cache/mde/`, honoring `$XDG_CACHE_HOME`.
#[must_use]
pub fn session_prefs_dir() -> PathBuf {
    if let Ok(s) = std::env::var("XDG_CACHE_HOME") {
        if !s.is_empty() {
            return PathBuf::from(s).join("mde");
        }
    }
    if let Some(home) = dirs::home_dir() {
        return home.join(".cache").join("mde");
    }
    PathBuf::from(".")
}

/// Full path to the sidecar file `mde-session` reads.
#[must_use]
pub fn session_prefs_path() -> PathBuf {
    session_prefs_dir().join("session-prefs.json")
}

/// Apply a `session.*` setting by rewriting the sidecar JSON.
///
/// # Errors
///
/// Returns an error when the key isn't a session key, the value
/// doesn't deserialize as a bool, or the sidecar can't be written.
pub fn apply(key: SettingKey, value: &SettingValue) -> anyhow::Result<()> {
    let flag: bool = value.to_serde()?;
    let mut prefs = load_prefs();
    match key {
        SettingKey::SessionSaveOnExit => prefs.save_on_exit = flag,
        SettingKey::SessionLockOnSuspend => prefs.lock_on_suspend = flag,
        SettingKey::SessionAutoSave => prefs.auto_save = flag,
        _ => anyhow::bail!("session: {key} is not a session key"),
    }
    write_prefs(&prefs)
}

/// Read the boolean for the matching session key out of the sidecar.
///
/// # Errors
///
/// Returns an error when the key isn't a session key.
pub fn current(key: SettingKey) -> anyhow::Result<SettingValue> {
    let prefs = load_prefs();
    let flag = match key {
        SettingKey::SessionSaveOnExit => prefs.save_on_exit,
        SettingKey::SessionLockOnSuspend => prefs.lock_on_suspend,
        SettingKey::SessionAutoSave => prefs.auto_save,
        _ => anyhow::bail!("session: {key} is not a session key"),
    };
    SettingValue::from_serde(&flag)
}

fn load_prefs() -> SessionPrefs {
    let path = session_prefs_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return SessionPrefs::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn write_prefs(prefs: &SessionPrefs) -> anyhow::Result<()> {
    let dir = session_prefs_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|e| anyhow::anyhow!("session: mkdir {} failed: {e}", dir.display()))?;
    let path = session_prefs_path();
    let text = serde_json::to_string_pretty(prefs)
        .map_err(|e| anyhow::anyhow!("session: serialize prefs failed: {e}"))?;
    std::fs::write(&path, text)
        .map_err(|e| anyhow::anyhow!("session: write {} failed: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Mutex, OnceLock};
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn with_xdg_cache<R>(tmp: &std::path::Path, body: impl FnOnce() -> R) -> R {
        let lock = ENV_LOCK.get_or_init(|| Mutex::new(()));
        let _g = lock.lock().unwrap_or_else(|e| e.into_inner());
        let prev = std::env::var_os("XDG_CACHE_HOME");
        std::env::set_var("XDG_CACHE_HOME", tmp);
        let r = body();
        match prev {
            Some(v) => std::env::set_var("XDG_CACHE_HOME", v),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }
        r
    }

    #[test]
    fn prefs_path_lives_under_xdg_cache_home_mde() {
        let tmp = tempfile::tempdir().expect("tempdir");
        with_xdg_cache(tmp.path(), || {
            let path = session_prefs_path();
            assert!(path.starts_with(tmp.path()));
            assert!(path.ends_with("session-prefs.json"));
            assert!(path.to_string_lossy().contains("/mde/"));
        });
    }

    #[test]
    fn apply_then_current_round_trips_save_on_exit() {
        let tmp = tempfile::tempdir().expect("tempdir");
        with_xdg_cache(tmp.path(), || {
            let value = SettingValue::from_serde(&true).unwrap();
            apply(SettingKey::SessionSaveOnExit, &value).expect("apply");
            let got = current(SettingKey::SessionSaveOnExit).expect("current");
            let flag: bool = got.to_serde().expect("de");
            assert!(flag);
        });
    }

    #[test]
    fn apply_writes_all_three_keys_without_clobbering_each_other() {
        let tmp = tempfile::tempdir().expect("tempdir");
        with_xdg_cache(tmp.path(), || {
            apply(
                SettingKey::SessionSaveOnExit,
                &SettingValue::from_serde(&true).unwrap(),
            )
            .unwrap();
            apply(
                SettingKey::SessionLockOnSuspend,
                &SettingValue::from_serde(&false).unwrap(),
            )
            .unwrap();
            apply(
                SettingKey::SessionAutoSave,
                &SettingValue::from_serde(&true).unwrap(),
            )
            .unwrap();
            let prefs = load_prefs();
            assert!(prefs.save_on_exit);
            assert!(!prefs.lock_on_suspend);
            assert!(prefs.auto_save);
        });
    }

    #[test]
    fn current_returns_false_when_file_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        with_xdg_cache(tmp.path(), || {
            let got = current(SettingKey::SessionSaveOnExit).expect("current");
            let flag: bool = got.to_serde().unwrap();
            assert!(!flag);
        });
    }

    #[test]
    fn apply_rejects_non_session_key() {
        let value = SettingValue::from_serde(&true).unwrap();
        let r = apply(SettingKey::ThemeName, &value);
        assert!(r.is_err());
    }

    #[test]
    fn apply_rejects_non_bool_value() {
        let tmp = tempfile::tempdir().expect("tempdir");
        with_xdg_cache(tmp.path(), || {
            let value = SettingValue::from_serde(&"not-a-bool").unwrap();
            let r = apply(SettingKey::SessionSaveOnExit, &value);
            assert!(r.is_err());
        });
    }

    #[test]
    fn current_rejects_non_session_key() {
        let r = current(SettingKey::ThemeName);
        assert!(r.is_err());
    }

    #[test]
    fn load_prefs_tolerates_corrupt_json() {
        let tmp = tempfile::tempdir().expect("tempdir");
        with_xdg_cache(tmp.path(), || {
            let mde = tmp.path().join("mde");
            std::fs::create_dir_all(&mde).unwrap();
            std::fs::write(mde.join("session-prefs.json"), "{not json").unwrap();
            let prefs = load_prefs();
            assert_eq!(prefs, SessionPrefs::default());
        });
    }
}
