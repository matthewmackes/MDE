//! v2.0.0 Phase B.11 — performance tuning helpers.
//!
//! Rust port of the read-only + sysfs-probe surface from
//! `mackes/mesh_perf.py`. Covers the queries the Mesh Performance
//! panel (Workbench) renders today:
//!
//!   * `wireguard` kernel module loaded / available
//!   * Current interface MTU from `/sys/class/net/<iface>/mtu`
//!   * GSO + GRO state via `ethtool -k <iface>` parse
//!
//! Write paths (sysctl tuning, MTU change) require root and route
//! through `mackes.admin_session.AdminSession` today; this module
//! ships the read paths the GUI needs without depending on the
//! AdminSession layer.

use std::path::Path;

/// True when the `wireguard` kernel module is currently loaded.
/// Reads `/proc/modules` directly — no subprocess.
#[must_use]
pub fn kernel_module_loaded() -> bool {
    let Ok(text) = std::fs::read_to_string("/proc/modules") else {
        return false;
    };
    text.lines().any(|line| {
        line.split(' ').next().map(str::trim) == Some("wireguard")
    })
}

/// True when `wireguard` can be loaded — either already loaded or
/// installed somewhere `modinfo -n wireguard` can find. Best-effort
/// when modinfo isn't on `$PATH` (rare).
#[must_use]
pub fn kernel_mode_available() -> bool {
    if kernel_module_loaded() {
        return true;
    }
    let Ok(out) = std::process::Command::new("modinfo")
        .args(["-n", "wireguard"])
        .output()
    else {
        return false;
    };
    out.status.success()
        && !String::from_utf8_lossy(&out.stdout).trim().is_empty()
}

/// Read MTU from `/sys/class/net/<iface>/mtu`. None when the
/// interface doesn't exist or the file can't be parsed.
#[must_use]
pub fn current_mtu(iface: &str) -> Option<u32> {
    let path = format!("/sys/class/net/{iface}/mtu");
    let text = std::fs::read_to_string(Path::new(&path)).ok()?;
    text.trim().parse().ok()
}

/// Run `ethtool -k <iface>` and return Some(true) if
/// `generic-segmentation-offload` is on, Some(false) if off, None
/// if ethtool isn't installed or the interface doesn't exist.
#[must_use]
pub fn gso_enabled(iface: &str) -> Option<bool> {
    let out = std::process::Command::new("ethtool")
        .args(["-k", iface])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_gso_state(&String::from_utf8_lossy(&out.stdout))
}

/// Pure helper: parse the `ethtool -k <iface>` output and return
/// whether `generic-segmentation-offload` is on. Lifted out for
/// unit-test coverage without a live interface.
#[must_use]
pub fn parse_gso_state(ethtool_output: &str) -> Option<bool> {
    for line in ethtool_output.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("generic-segmentation-offload:") {
            let s = rest.trim();
            return match s {
                "on" => Some(true),
                "off" => Some(false),
                _ => None,
            };
        }
    }
    None
}

/// Pure helper: parse `/proc/modules` content and return the list
/// of currently-loaded module names. Lifted for unit-test coverage.
#[must_use]
pub fn parse_loaded_modules(proc_modules: &str) -> Vec<String> {
    proc_modules
        .lines()
        .filter_map(|l| l.split(' ').next().map(str::trim).map(String::from))
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_gso_state_reads_on() {
        let out = "\
Features for tailscale0:
generic-segmentation-offload: on
tcp-segmentation-offload: off
        ";
        assert_eq!(parse_gso_state(out), Some(true));
    }

    #[test]
    fn parse_gso_state_reads_off() {
        let out = "generic-segmentation-offload: off\n";
        assert_eq!(parse_gso_state(out), Some(false));
    }

    #[test]
    fn parse_gso_state_returns_none_when_absent() {
        let out = "tcp-segmentation-offload: on\n";
        assert_eq!(parse_gso_state(out), None);
    }

    #[test]
    fn parse_gso_state_returns_none_for_unknown_value() {
        let out = "generic-segmentation-offload: maybe\n";
        assert_eq!(parse_gso_state(out), None);
    }

    #[test]
    fn parse_loaded_modules_picks_first_column() {
        let proc = "\
wireguard 86016 0 - Live 0x0000000000000000
udp_tunnel 24576 1 wireguard, Live 0x0000000000000000

";
        let mods = parse_loaded_modules(proc);
        assert!(mods.contains(&"wireguard".to_string()));
        assert!(mods.contains(&"udp_tunnel".to_string()));
        // Empty line not surfaced.
        assert!(!mods.iter().any(String::is_empty));
    }

    #[test]
    fn parse_loaded_modules_empty_input_returns_empty() {
        assert!(parse_loaded_modules("").is_empty());
    }

    #[test]
    fn current_mtu_returns_none_for_nonexistent_iface() {
        assert!(current_mtu("definitely-not-a-real-iface-12345").is_none());
    }
}
