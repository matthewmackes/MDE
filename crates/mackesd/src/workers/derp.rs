//! v2.0.0 Phase B.11 — DERP relay control helpers.
//!
//! Rust port of the status-side surface from `mackes/mesh_derp.py`.
//! Covers the read-only checks the Workbench Mesh VPN panel uses to
//! render the "DERP relay" status row. Install / start / stop /
//! uninstall paths require root and route through the privileged
//! `mackes.admin_session.AdminSession` Python layer for now; the
//! v2.0.0 cut moves those through `dev.mackes.MDE.Fleet` (Phase
//! G.4 already shipped) so the Workbench panel can call them
//! without invoking polkit per-action.

use std::path::Path;

/// Canonical install path the v1.x line uses. Matches DERPER_BIN in
/// `mackes/mesh_derp.py`.
pub const DERPER_BIN: &str = "/usr/local/bin/derper";

/// systemd unit name the v1.x line registers for the DERP daemon.
pub const DERPER_UNIT: &str = "mackes-derper";

/// Return `true` when the derper binary is installed AND executable.
/// File check only — no subprocess. Mirrors `mesh_derp.is_installed()`.
#[must_use]
pub fn is_installed() -> bool {
    is_installed_at(Path::new(DERPER_BIN))
}

/// Same as `is_installed()` but lets callers point at a custom path
/// (used by tests + the install-from-source flow's verification).
#[must_use]
pub fn is_installed_at(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else { return false };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = meta.permissions().mode();
        // Owner-executable bit.
        return (mode & 0o100) != 0;
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Pure helper: parse `systemctl is-active <unit>` output and decide
/// whether the unit is active. Returns `true` only for the literal
/// "active" — `failed` / `inactive` / `activating` / etc → `false`.
#[must_use]
pub fn parse_is_active(stdout: &str, exit_code: i32) -> bool {
    exit_code == 0 && stdout.trim() == "active"
}

/// Return `true` when the DERP unit is currently active via
/// `systemctl is-active mackes-derper`.
#[must_use]
pub fn is_running() -> bool {
    let Ok(out) = std::process::Command::new("systemctl")
        .args(["is-active", DERPER_UNIT])
        .output()
    else {
        return false;
    };
    let code = out.status.code().unwrap_or(-1);
    parse_is_active(&String::from_utf8_lossy(&out.stdout), code)
}

/// Render the DERP map JSON for the locked Mackes-region defaults.
/// Pure function — caller writes the result to disk via the
/// privileged AdminSession path.
#[must_use]
pub fn render_derp_map(region_id: u32, region_name: &str, hostname: &str) -> String {
    // Matches mackes/mesh_derp.py::render_derp_map shape.
    let body = serde_json::json!({
        "Regions": {
            region_id.to_string(): {
                "RegionID":   region_id,
                "RegionCode": region_name.to_lowercase(),
                "RegionName": region_name,
                "Nodes": [{
                    "Name":     "1a",
                    "RegionID": region_id,
                    "HostName": hostname,
                    "STUNOnly": false,
                }]
            }
        }
    });
    serde_json::to_string_pretty(&body).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_is_active_matches_only_literal_active() {
        assert!(parse_is_active("active\n", 0));
        assert!(parse_is_active("  active  ", 0));
        assert!(!parse_is_active("active", 1)); // wrong exit code
        assert!(!parse_is_active("inactive\n", 0));
        assert!(!parse_is_active("failed\n", 0));
        assert!(!parse_is_active("activating\n", 0));
        assert!(!parse_is_active("", 0));
    }

    #[test]
    fn is_installed_at_returns_false_for_missing_path() {
        let nope = std::path::Path::new("/does/not/exist/derper-nope");
        assert!(!is_installed_at(nope));
    }

    #[test]
    fn render_derp_map_carries_region_and_host() {
        let json = render_derp_map(901, "Mackes", "derp.example.com");
        assert!(json.contains("\"RegionID\": 901"));
        assert!(json.contains("\"RegionName\": \"Mackes\""));
        assert!(json.contains("\"HostName\": \"derp.example.com\""));
        assert!(json.contains("\"RegionCode\": \"mackes\""));
    }

    #[test]
    fn render_derp_map_handles_lowercase_code_conversion() {
        let json = render_derp_map(902, "Lab", "derp.local");
        assert!(json.contains("\"RegionCode\": \"lab\""));
    }

    #[test]
    fn render_derp_map_emits_nodes_array() {
        let json = render_derp_map(901, "Mackes", "h.example.com");
        // STUNOnly false → all DERP traffic is mirrored, not stun-only.
        assert!(json.contains("\"STUNOnly\": false"));
        assert!(json.contains("\"Name\": \"1a\""));
    }
}
