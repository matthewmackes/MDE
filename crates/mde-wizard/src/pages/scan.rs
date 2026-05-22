//! Scan page — environment probe (CPU/RAM/disk/distro/Wayland).
//!
//! Read-only — no state mutation. The user sees a snapshot of
//! the host and clicks Next.

use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScanReport {
    pub cpu_cores: usize,
    pub ram_mb: u64,
    pub disk_free_gb: u64,
    pub fedora_release: String,
    pub wayland_session: bool,
    pub hostname: String,
}

impl ScanReport {
    /// Run the probe — best-effort. Any individual field that
    /// can't be read falls back to a 0/empty value; the wizard
    /// still proceeds.
    #[must_use]
    pub fn probe() -> Self {
        Self {
            cpu_cores: read_cpu_cores(),
            ram_mb: read_ram_mb(),
            disk_free_gb: 0, // statvfs requires libc; deferred to runtime
            fedora_release: read_fedora_release(),
            wayland_session: std::env::var("WAYLAND_DISPLAY").is_ok(),
            hostname: read_hostname(),
        }
    }

    /// Format the report for display.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        vec![
            format!("Hostname:     {}", self.hostname),
            format!("Fedora:       {}", self.fedora_release),
            format!("CPU cores:    {}", self.cpu_cores),
            format!("RAM:          {} MB", self.ram_mb),
            format!(
                "Wayland:      {}",
                if self.wayland_session { "yes" } else { "no" }
            ),
        ]
    }
}

fn read_cpu_cores() -> usize {
    std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(0)
}

fn read_ram_mb() -> u64 {
    parse_meminfo_total_kb(Path::new("/proc/meminfo")).unwrap_or(0) / 1024
}

/// Pure helper — parse `MemTotal:` line from `/proc/meminfo`
/// content into kB.
#[must_use]
pub fn parse_meminfo_total_kb_str(content: &str) -> Option<u64> {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            return rest.trim().split_whitespace().next()?.parse().ok();
        }
    }
    None
}

fn parse_meminfo_total_kb(path: &Path) -> Option<u64> {
    let content = std::fs::read_to_string(path).ok()?;
    parse_meminfo_total_kb_str(&content)
}

fn read_fedora_release() -> String {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|content| parse_version_id(&content))
        .unwrap_or_else(|| "(unknown)".into())
}

/// Pure helper — extract `VERSION_ID=...` from /etc/os-release.
#[must_use]
pub fn parse_version_id(content: &str) -> Option<String> {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("VERSION_ID=") {
            return Some(rest.trim().trim_matches('"').to_string());
        }
    }
    None
}

fn read_hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "fedora".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_meminfo_total_kb_extracts_value() {
        let content = "MemTotal:       16384000 kB\nMemFree:        8192000 kB\n";
        assert_eq!(parse_meminfo_total_kb_str(content), Some(16_384_000));
    }

    #[test]
    fn parse_meminfo_returns_none_when_absent() {
        let content = "MemFree:        8192000 kB\n";
        assert_eq!(parse_meminfo_total_kb_str(content), None);
    }

    #[test]
    fn parse_version_id_extracts_value() {
        let content = r#"NAME="Fedora Linux"
VERSION_ID=44
PRETTY_NAME="Fedora Linux 44"
"#;
        assert_eq!(parse_version_id(content), Some("44".into()));
    }

    #[test]
    fn parse_version_id_strips_quotes() {
        let content = "VERSION_ID=\"44\"\n";
        assert_eq!(parse_version_id(content), Some("44".into()));
    }

    #[test]
    fn parse_version_id_returns_none_when_absent() {
        assert_eq!(parse_version_id("NAME=Fedora"), None);
    }

    #[test]
    fn probe_does_not_panic() {
        let _report = ScanReport::probe();
    }

    #[test]
    fn lines_emits_five_rows() {
        let report = ScanReport {
            cpu_cores: 8,
            ram_mb: 16384,
            disk_free_gb: 200,
            fedora_release: "44".into(),
            wayland_session: true,
            hostname: "lab-01".into(),
        };
        let lines = report.lines();
        assert_eq!(lines.len(), 5);
        assert!(lines.iter().any(|l| l.contains("lab-01")));
        assert!(lines.iter().any(|l| l.contains("Wayland:      yes")));
        assert!(lines.iter().any(|l| l.contains("8")));
    }

    #[test]
    fn lines_marks_wayland_no_when_absent() {
        let report = ScanReport {
            wayland_session: false,
            ..Default::default()
        };
        let lines = report.lines();
        assert!(lines.iter().any(|l| l.contains("Wayland:      no")));
    }
}
