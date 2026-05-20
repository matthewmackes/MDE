//! v2.0.0 Phase B.11 — NATS server control helpers.
//!
//! Rust port of the status-side surface from `mackes/mesh_nats.py`.
//! Mirrors the shape of [`super::derp`] — same install + running
//! probe pattern, same render-config-pure-function pattern.
//!
//! Install / start / stop / uninstall require root and continue to
//! route through `mackes.admin_session.AdminSession` for now.

use std::path::Path;

/// Canonical install path the v1.x line uses.
pub const NATS_BIN: &str = "/usr/local/bin/nats-server";

/// systemd unit name the v1.x line registers for the NATS daemon.
pub const NATS_UNIT: &str = "mackes-nats";

/// Default JetStream subject used for cross-peer event publishing.
pub const DEFAULT_BUCKET: &str = "mackes-events";

/// Return `true` when the nats-server binary is installed AND
/// executable.
#[must_use]
pub fn is_server_installed() -> bool {
    is_server_installed_at(Path::new(NATS_BIN))
}

/// Same as [`is_server_installed`] but lets callers pass a custom path.
#[must_use]
pub fn is_server_installed_at(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else { return false };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = meta.permissions().mode();
        return (mode & 0o100) != 0;
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Return `true` when the NATS unit is currently active via
/// `systemctl is-active mackes-nats`.
#[must_use]
pub fn is_server_running() -> bool {
    let Ok(out) = std::process::Command::new("systemctl")
        .args(["is-active", NATS_UNIT])
        .output()
    else {
        return false;
    };
    let code = out.status.code().unwrap_or(-1);
    super::derp::parse_is_active(&String::from_utf8_lossy(&out.stdout), code)
}

/// Render the nats-server JetStream config that the Mackes server
/// install would write to disk. Pure function; the privileged
/// AdminSession path drops it at `/etc/mackes-nats.conf`.
#[must_use]
pub fn render_server_config(control_ip: &str) -> String {
    // Matches mackes/mesh_nats.py::_server_config shape.
    format!(
        "# Mackes NATS server (Phase B.11 port). Bind control IP\n\
         # set by the install flow.\n\
         host: \"{control_ip}\"\n\
         port: 4222\n\
         http_port: 8222\n\
         jetstream {{\n    \
             store_dir: \"/var/lib/mackes-nats/jetstream\"\n    \
             max_mem: 64MB\n    \
             max_file: 512MB\n\
         }}\n"
    )
}

/// Resolve the NATS control URL the client side connects to. Returns
/// `"nats://<host>:4222"`. Pure helper for unit-test coverage.
#[must_use]
pub fn control_url(host: &str) -> String {
    format!("nats://{host}:4222")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_server_installed_at_returns_false_for_missing() {
        assert!(!is_server_installed_at(Path::new("/does/not/exist/nats")));
    }

    #[test]
    fn render_server_config_carries_control_ip() {
        let cfg = render_server_config("10.0.0.5");
        assert!(cfg.contains("host: \"10.0.0.5\""));
        assert!(cfg.contains("port: 4222"));
        assert!(cfg.contains("http_port: 8222"));
        assert!(cfg.contains("jetstream"));
        assert!(cfg.contains("store_dir"));
    }

    #[test]
    fn render_server_config_uses_jetstream_block() {
        let cfg = render_server_config("127.0.0.1");
        assert!(cfg.contains("max_mem"));
        assert!(cfg.contains("max_file"));
    }

    #[test]
    fn control_url_renders_loopback() {
        assert_eq!(control_url("127.0.0.1"), "nats://127.0.0.1:4222");
    }

    #[test]
    fn control_url_handles_hostname() {
        assert_eq!(control_url("nats.example.com"),
                   "nats://nats.example.com:4222");
    }

    #[test]
    fn default_bucket_is_mackes_events() {
        assert_eq!(DEFAULT_BUCKET, "mackes-events");
    }
}
