//! Phase E.18 — Win10-style lower-right watermark.
//!
//! Shows MDE version + Fedora release + pending-update count when
//! dnf has updates queued. Polls `dnf check-update --quiet` every
//! 4 hours via a tokio task; the rendered widget reads the cached
//! count and stays invisible when the count is zero.
//!
//! 2026 visual: 11px Red Hat Mono, 28% alpha text, anchored to the
//! bottom-right corner with a 24px inset. Never interactive.
//!
//! **Sync with legacy GTK watermark (v2.0.3)**: the build-hash and
//! build-date strings come from `/usr/share/mde/build-{hash,date}`
//! — the same source-of-truth files that `mackes-panel/src/
//! watermark.rs` reads. Both panel surfaces report identical
//! identity so operators can't see two different builds claimed.

use std::path::Path;

/// Snapshot of every value the watermark renders.
#[derive(Debug, Clone, Default)]
pub struct WatermarkState {
    pub mde_version: String,
    pub fedora_release: String,
    pub build_hash: Option<String>,
    /// UTC build date in `YYYY-MM-DD` form. `None` on dev checkouts
    /// where `/usr/share/mde/build-date` doesn't exist (the RPM
    /// `%install` step writes it). Synced with the legacy GTK
    /// watermark via the same file (v2.0.3).
    pub build_date: Option<String>,
    pub hostname: String,
    pub pending_updates: u32,
}

impl WatermarkState {
    /// Best-effort load: reads each field from a stable source,
    /// falling back to an empty string on any error.
    ///
    /// `build_hash` and `build_date` come from
    /// `/usr/share/mde/build-{hash,date}` — the same files the
    /// legacy GTK `mackes-panel` watermark consumes, so both
    /// surfaces report identical identity (v2.0.3 sync fix).
    /// `MDE_BUILD_HASH` env (set by build.rs in dev) wins over the
    /// file when both exist; this keeps `cargo run` builds showing
    /// the live hash even when an installed RPM also wrote a file.
    #[must_use]
    pub fn load() -> Self {
        Self {
            mde_version: env!("CARGO_PKG_VERSION").to_string(),
            fedora_release: read_fedora_release(),
            build_hash: option_env!("MDE_BUILD_HASH")
                .map(str::to_owned)
                .or_else(|| read_build_file_for_hash()),
            build_date: read_build_file_for_date(),
            hostname: read_hostname(),
            pending_updates: read_pending_update_count(),
        }
    }

    /// Single-line label rendered onto the panel. Empty when no
    /// updates are pending — the rendered widget hides on empty.
    #[must_use]
    pub fn render_line(&self) -> String {
        if self.pending_updates == 0 {
            return String::new();
        }
        let hash = self
            .build_hash
            .as_deref()
            .map(|h| format!(" · {h}"))
            .unwrap_or_default();
        let date = self
            .build_date
            .as_deref()
            .map(|d| format!(" · Built {d}"))
            .unwrap_or_default();
        format!(
            "MDE {ver}{hash}{date} · Fedora {release} · {host} · {n} updates pending",
            ver = self.mde_version,
            release = self.fedora_release,
            host = self.hostname,
            n = self.pending_updates,
        )
    }
}

fn read_fedora_release() -> String {
    read_os_release_field("VERSION_ID").unwrap_or_else(|| "44".to_string())
}

fn read_os_release_field(key: &str) -> Option<String> {
    let content = std::fs::read_to_string("/etc/os-release").ok()?;
    parse_os_release_field(&content, key)
}

/// Pure parser — pulls `KEY="value"` lines out of /etc/os-release
/// shape strings. Exposed for tests.
#[must_use]
pub fn parse_os_release_field(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix(&format!("{key}=")) {
            let trimmed = rest.trim().trim_matches('"');
            return Some(trimmed.to_string());
        }
    }
    None
}

fn read_hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "fedora".to_string())
}

fn read_pending_update_count() -> u32 {
    // Cached count file (written by the dnf-update worker, lands at
    // E.18 worker integration). Returns 0 if absent.
    let cache_path = dirs::cache_dir()
        .map(|d| d.join("mde/dnf-updates.count"))
        .unwrap_or_default();
    parse_count_file(&cache_path)
}

/// Read `/usr/share/mde/build-hash` (RPM `%install`-written). Synced
/// with the legacy GTK watermark — both panels consume the same file
/// so they can't drift on which build is reported.
fn read_build_file_for_hash() -> Option<String> {
    read_build_meta(&[
        "/usr/share/mde/build-hash",
        "/usr/share/mackes-shell/build-hash",
        "build-hash",
    ])
}

/// Read `/usr/share/mde/build-date` (RPM `%install`-written UTC
/// `YYYY-MM-DD`). `None` on dev checkouts.
fn read_build_file_for_date() -> Option<String> {
    read_build_meta(&[
        "/usr/share/mde/build-date",
        "/usr/share/mackes-shell/build-date",
        "build-date",
    ])
}

/// Pure helper — walk the candidate paths and return the first non-
/// empty trimmed content. Exposed for tests.
#[must_use]
pub fn read_build_meta(candidates: &[&str]) -> Option<String> {
    for c in candidates {
        if let Ok(s) = std::fs::read_to_string(c) {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_owned());
            }
        }
    }
    None
}

/// Pure helper — reads + parses the dnf-updates count file. Exposed
/// for tests.
#[must_use]
pub fn parse_count_file(path: &Path) -> u32 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn render_empty_when_no_pending_updates() {
        let state = WatermarkState::default();
        assert!(state.render_line().is_empty());
    }

    #[test]
    fn render_includes_every_field_when_updates_pending() {
        let state = WatermarkState {
            mde_version: "2.0.3".into(),
            fedora_release: "44".into(),
            build_hash: Some("abc123".into()),
            build_date: Some("2026-05-22".into()),
            hostname: "lab-01".into(),
            pending_updates: 12,
        };
        let line = state.render_line();
        assert!(line.contains("MDE 2.0.3"));
        assert!(line.contains("abc123"));
        assert!(line.contains("Built 2026-05-22"));
        assert!(line.contains("Fedora 44"));
        assert!(line.contains("lab-01"));
        assert!(line.contains("12 updates pending"));
    }

    #[test]
    fn render_omits_hash_when_unset() {
        let state = WatermarkState {
            mde_version: "2.0.3".into(),
            fedora_release: "44".into(),
            build_hash: None,
            build_date: None,
            hostname: "lab-01".into(),
            pending_updates: 1,
        };
        let line = state.render_line();
        assert!(!line.contains("·  ·")); // no double separator
        assert!(line.starts_with("MDE 2.0.3 · Fedora 44"));
        assert!(!line.contains("Built"));
    }

    #[test]
    fn render_includes_build_date_separately_from_hash() {
        // The v2.0.3 sync change splits build date out as its own
        // `· Built YYYY-MM-DD` clause. Lock the ordering: version,
        // then hash, then date, then Fedora/host.
        let state = WatermarkState {
            mde_version: "2.0.3".into(),
            fedora_release: "44".into(),
            build_hash: Some("abc123".into()),
            build_date: Some("2026-05-22".into()),
            hostname: "lab-01".into(),
            pending_updates: 1,
        };
        let line = state.render_line();
        let hash_idx = line.find("abc123").expect("hash present");
        let date_idx = line.find("2026-05-22").expect("date present");
        let fedora_idx = line.find("Fedora").expect("fedora present");
        assert!(hash_idx < date_idx, "hash must come before date in {line}");
        assert!(
            date_idx < fedora_idx,
            "date must come before Fedora in {line}"
        );
    }

    #[test]
    fn render_handles_only_date_no_hash() {
        // Edge case: build-date file exists but build-hash doesn't
        // (RPM install ordering glitch). Render must still produce
        // a coherent line without a stray `· Built` after nothing.
        let state = WatermarkState {
            mde_version: "2.0.3".into(),
            fedora_release: "44".into(),
            build_hash: None,
            build_date: Some("2026-05-22".into()),
            hostname: "lab-01".into(),
            pending_updates: 1,
        };
        let line = state.render_line();
        assert!(line.contains("Built 2026-05-22"));
        assert!(line.starts_with("MDE 2.0.3 · Built 2026-05-22"));
    }

    #[test]
    fn read_build_meta_returns_none_for_missing_paths() {
        let tmp = tempdir().unwrap();
        let absent = tmp.path().join("does-not-exist");
        let absent_s = absent.to_string_lossy().into_owned();
        let paths = [absent_s.as_str()];
        assert_eq!(read_build_meta(&paths), None);
    }

    #[test]
    fn read_build_meta_returns_first_non_empty_candidate() {
        let tmp = tempdir().unwrap();
        let empty = tmp.path().join("empty");
        let real = tmp.path().join("real");
        std::fs::write(&empty, "   \n").unwrap();
        std::fs::write(&real, "2026-05-22\n").unwrap();
        let empty_s = empty.to_string_lossy().into_owned();
        let real_s = real.to_string_lossy().into_owned();
        let paths = [empty_s.as_str(), real_s.as_str()];
        assert_eq!(read_build_meta(&paths), Some("2026-05-22".to_string()));
    }

    #[test]
    fn parse_os_release_extracts_field() {
        let content = r#"
NAME="Fedora Linux"
VERSION="44 (Workstation)"
VERSION_ID=44
PRETTY_NAME="Fedora Linux 44"
"#;
        assert_eq!(
            parse_os_release_field(content, "VERSION_ID"),
            Some("44".into())
        );
        assert_eq!(
            parse_os_release_field(content, "NAME"),
            Some("Fedora Linux".into())
        );
    }

    #[test]
    fn parse_os_release_returns_none_for_missing_key() {
        let content = "NAME=Fedora\n";
        assert_eq!(parse_os_release_field(content, "MISSING"), None);
    }

    #[test]
    fn parse_count_file_returns_zero_when_missing() {
        let tmp = tempdir().unwrap();
        assert_eq!(parse_count_file(&tmp.path().join("absent")), 0);
    }

    #[test]
    fn parse_count_file_parses_integer() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("count");
        std::fs::write(&path, "42\n").unwrap();
        assert_eq!(parse_count_file(&path), 42);
    }

    #[test]
    fn parse_count_file_falls_back_on_garbage() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("count");
        std::fs::write(&path, "not a number").unwrap();
        assert_eq!(parse_count_file(&path), 0);
    }

    #[test]
    fn load_does_not_panic() {
        // Even on a system without /etc/os-release etc., load()
        // returns a valid state.
        let _state = WatermarkState::load();
    }
}
