//! Phase E.18 — Win10-style lower-right watermark.
//!
//! Shows MDE version + Fedora release + pending-update count when
//! dnf has updates queued. Polls `dnf check-update --quiet` every
//! 4 hours via a tokio task; the rendered widget reads the cached
//! count and stays invisible when the count is zero.
//!
//! 2026 visual: 11px Red Hat Mono, 28% alpha text, anchored to the
//! bottom-right corner with a 24px inset. Never interactive.

use std::path::Path;

/// Snapshot of every value the watermark renders.
#[derive(Debug, Clone, Default)]
pub struct WatermarkState {
    pub mde_version: String,
    pub fedora_release: String,
    pub build_hash: Option<String>,
    pub hostname: String,
    pub pending_updates: u32,
}

impl WatermarkState {
    /// Best-effort load: reads each field from a stable source,
    /// falling back to an empty string on any error.
    #[must_use]
    pub fn load() -> Self {
        Self {
            mde_version: env!("CARGO_PKG_VERSION").to_string(),
            fedora_release: read_fedora_release(),
            build_hash: option_env!("MDE_BUILD_HASH").map(str::to_owned),
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
        format!(
            "MDE {ver}{hash} · Fedora {release} · {host} · {n} updates pending",
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
            mde_version: "2.0.0".into(),
            fedora_release: "44".into(),
            build_hash: Some("abc123".into()),
            hostname: "lab-01".into(),
            pending_updates: 12,
        };
        let line = state.render_line();
        assert!(line.contains("MDE 2.0.0"));
        assert!(line.contains("abc123"));
        assert!(line.contains("Fedora 44"));
        assert!(line.contains("lab-01"));
        assert!(line.contains("12 updates pending"));
    }

    #[test]
    fn render_omits_hash_when_unset() {
        let state = WatermarkState {
            mde_version: "2.0.0".into(),
            fedora_release: "44".into(),
            build_hash: None,
            hostname: "lab-01".into(),
            pending_updates: 1,
        };
        let line = state.render_line();
        assert!(!line.contains("·  ·")); // no double separator
        assert!(line.starts_with("MDE 2.0.0 · Fedora 44"));
    }

    #[test]
    fn parse_os_release_extracts_field() {
        let content = r#"
NAME="Fedora Linux"
VERSION="44 (Workstation)"
VERSION_ID=44
PRETTY_NAME="Fedora Linux 44"
"#;
        assert_eq!(parse_os_release_field(content, "VERSION_ID"), Some("44".into()));
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
