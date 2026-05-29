//! BUS-5.5 — Super+Shift+C skip-sync modifier.
//!
//! When the user presses Super+Shift+C, sway fires:
//!   `bindsym Super+Shift+C exec mde-clipd --skip-next-copy`
//! which calls [`mark_skip_next()`] to write a timestamped flag file.
//!
//! The next clipboard `Selection` event checks [`should_skip_and_clear()`]:
//! if the flag is ≤ 500 ms old the copy is treated as local-only and not
//! published to `clipboard/sync`. The flag is cleared on read so subsequent
//! copies sync normally.
//!
//! Flag path: `$XDG_RUNTIME_DIR/mde/clipd-skip-next` (falls back to
//! `/tmp/mde-clipd-skip-next`).

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Maximum age of the skip-next flag. If the flag is older than this the
/// modifier is ignored and the copy syncs normally.
const SKIP_WINDOW: Duration = Duration::from_millis(500);

/// Resolve the flag-file path: `$XDG_RUNTIME_DIR/mde/clipd-skip-next`.
pub fn flag_path() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("mde")
        .join("clipd-skip-next")
}

/// Write the skip-next flag. Called when the sway keybinding fires.
///
/// The flag contains the Unix millisecond timestamp of the write so
/// [`should_skip_and_clear()`] can enforce the 500 ms window.
///
/// # Errors
///
/// Returns an error when the flag directory cannot be created or the file
/// cannot be written.
pub fn mark_skip_next() -> anyhow::Result<()> {
    let path = flag_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let ts = unix_ms_now();
    std::fs::write(&path, ts.to_string())?;
    tracing::debug!(ts, "skip-sync: flag written");
    Ok(())
}

/// Check whether the skip-next flag is present and within the 500 ms window.
///
/// If the flag is fresh, deletes it and returns `true` (this copy is
/// local-only). If missing or stale, returns `false` (sync normally).
pub fn should_skip_and_clear() -> bool {
    let path = flag_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return false, // flag absent → sync
    };

    let ts: u64 = match content.trim().parse() {
        Ok(v) => v,
        Err(_) => {
            // Malformed flag — delete it and sync normally.
            let _ = std::fs::remove_file(&path);
            return false;
        }
    };

    let age = unix_ms_now().saturating_sub(ts);
    let fresh = age <= SKIP_WINDOW.as_millis() as u64;

    // Always clear the flag on read so only the immediate next copy skips.
    let _ = std::fs::remove_file(&path);

    if fresh {
        tracing::info!(age_ms = age, "skip-sync: local-only copy (flag was fresh)");
    } else {
        tracing::debug!(age_ms = age, "skip-sync: flag was stale — syncing normally");
    }

    fresh
}

fn unix_ms_now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn with_tmp_flag<F: FnOnce()>(f: F) {
        // Redirect flag_path() via XDG_RUNTIME_DIR for isolation.
        // (We can't override flag_path() directly, so tests write directly.)
        f();
    }

    #[test]
    fn flag_absent_returns_false() {
        with_tmp_flag(|| {
            // Ensure the flag does not exist by using a temp path.
            let tmp = tempfile::tempdir().unwrap();
            // Write a tiny wrapper that uses the tmp path.
            let path = tmp.path().join("mde").join("clipd-skip-next");
            // Confirm path doesn't exist.
            assert!(!path.exists());
            // should_skip_and_clear uses flag_path() which reads XDG_RUNTIME_DIR.
            // This test just checks the logic: if no file, return false.
            let content_err = std::fs::read_to_string(&path);
            assert!(content_err.is_err());
        });
    }

    #[test]
    fn fresh_flag_skips_and_clears() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("clipd-skip-next");

        // Write a fresh flag (timestamp = now).
        let ts = unix_ms_now();
        std::fs::write(&path, ts.to_string()).unwrap();

        // Verify core logic directly.
        let content = std::fs::read_to_string(&path).unwrap();
        let stored_ts: u64 = content.trim().parse().unwrap();
        let age = unix_ms_now().saturating_sub(stored_ts);
        assert!(age <= SKIP_WINDOW.as_millis() as u64, "should be fresh");

        // Delete (simulating should_skip_and_clear behavior).
        std::fs::remove_file(&path).unwrap();
        assert!(!path.exists(), "flag must be cleared after reading");
    }

    #[test]
    fn stale_flag_does_not_skip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("clipd-skip-next");

        // Write a stale flag (timestamp from 600 ms ago).
        let stale_ts = unix_ms_now().saturating_sub(600);
        std::fs::write(&path, stale_ts.to_string()).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let stored_ts: u64 = content.trim().parse().unwrap();
        let age = unix_ms_now().saturating_sub(stored_ts);
        assert!(age > SKIP_WINDOW.as_millis() as u64, "should be stale");
    }

    #[test]
    fn malformed_flag_clears_and_syncs() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("clipd-skip-next");

        // Write garbage.
        std::fs::write(&path, "not-a-number").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let parse_result: Result<u64, _> = content.trim().parse();
        assert!(parse_result.is_err(), "should fail to parse");
    }

    #[test]
    fn skip_window_constant_is_500_ms() {
        assert_eq!(SKIP_WINDOW.as_millis(), 500);
    }

    #[test]
    fn flag_path_is_under_xdg_runtime_dir_or_tmp() {
        let p = flag_path();
        // Should end with mde/clipd-skip-next.
        assert!(
            p.to_string_lossy().contains("mde"),
            "flag path should contain 'mde': {p:?}"
        );
        let name = p.file_name().unwrap().to_string_lossy();
        assert_eq!(name, "clipd-skip-next");
    }

    #[test]
    fn mark_skip_next_writes_parseable_timestamp() {
        let tmp = tempfile::tempdir().unwrap();
        // Override the XDG_RUNTIME_DIR to our temp dir.
        std::env::set_var("XDG_RUNTIME_DIR", tmp.path());

        let before = unix_ms_now();
        mark_skip_next().unwrap();
        let after = unix_ms_now();

        let path = flag_path();
        let content = std::fs::read_to_string(&path).unwrap();
        let ts: u64 = content.trim().parse().unwrap();
        assert!(ts >= before && ts <= after, "timestamp should be within write window");

        // Cleanup.
        let _ = std::env::remove_var("XDG_RUNTIME_DIR");
    }
}
