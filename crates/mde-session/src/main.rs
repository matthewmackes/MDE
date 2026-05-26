//! `mde-session` — v2.0.0 Phase D.1 (skeleton) + D.4 (lock) + D.7
//! (retires mackes-wm / mackes-enforce-session).
//!
//! User-session orchestrator. systemd's mde-session.service execs
//! this binary at login; it:
//!
//!   1. Reads the v1.x → v2.0.0 config-path migrator's marker (Phase
//!      0.5) — runs the migrator if it hasn't yet.
//!   2. Re-applies persisted settings sidecars (Phase C
//!      $XDG_CACHE_HOME/mde/*) via the matching applier modules.
//!   3. Registers the `dev.mackes.MDE.Session` zbus surface
//!      (Logout / Restart / Shutdown / Lock / SaveLayout).
//!   4. Execs the compositor (sway) and waits.
//!   5. On SIGTERM / SIGINT cleanly tears down: signals the zbus
//!      server, waits for sway to exit, exits 0.
//!
//! Iced + libcosmic for the logout / restart / shutdown dialog
//! (Phase D.2) is intentionally NOT pulled in here — that's a
//! separate process (`mde-logout-dialog`) the Session surface can
//! exec when needed. Keeping this binary Iced-free means the user-
//! session unit stays tiny (~250 LOC) + boots fast.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod autostart;
mod lock;
mod session;
mod theme_pump;

use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::Context;
use tokio::process::Command;
use tokio::signal::unix::{signal, SignalKind};

/// MDE-shipped sway config. Falls back here when the operator has no
/// per-user override at `~/.config/sway/config` (the failure mode that
/// landed operators in stock Fedora sway on fresh installs).
const SYSTEM_SWAY_CONFIG: &str = "/usr/share/mde/sway/config";

/// Compositor command. Defaults to `sway` (wayland feature) or `i3`
/// (x11 feature); override via `$MDE_COMPOSITOR` for development.
fn compositor_cmd() -> String {
    mackesd_core::env_with_legacy_fallback("MDE_COMPOSITOR", "MACKES_COMPOSITOR")
        .unwrap_or_else(default_compositor)
}

#[cfg(not(feature = "x11"))]
fn default_compositor() -> String {
    "sway".to_owned()
}

#[cfg(feature = "x11")]
fn default_compositor() -> String {
    "i3".to_owned()
}

fn user_sway_config_path() -> Option<PathBuf> {
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))?;
    Some(base.join("sway").join("config"))
}

/// Pure helper: pick the `-c <path>` args sway should be invoked with.
/// Empty vec = "let sway resolve its config the default way" — returned
/// for non-sway compositors, when the user already has
/// `~/.config/sway/config`, or when the system fallback is also absent.
fn sway_config_args(
    compositor: &str,
    user_config: Option<&Path>,
    system_config: &Path,
) -> Vec<String> {
    if compositor != "sway" {
        return Vec::new();
    }
    if let Some(p) = user_config {
        if p.exists() {
            return Vec::new();
        }
    }
    if !system_config.exists() {
        return Vec::new();
    }
    vec![
        "-c".to_string(),
        system_config.to_string_lossy().into_owned(),
    ]
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();
    tracing::info!("mde-session: starting");

    // 0. Portal-37 — apply the MDE-Dark + Intel One Mono settings to
    //    every GTK / Qt6 config file the pump owns before
    //    autostarted apps come up. Idempotent; never fatal.
    let changed = theme_pump::apply();
    if !changed.is_empty() {
        tracing::info!(count = changed.len(), "theme-pump: settings refreshed");
    }

    // 1. Read autostart entries the user has explicitly enabled and
    //    spawn each as a detached child. Hidden=true overlays
    //    suppress system-wide entries.
    autostart::launch_user_autostart().await;

    // 2. Register the dev.mackes.MDE.Session zbus interface so the
    //    panel + Workbench can drive Logout / Restart / Lock.
    let session = session::SessionState::new();
    let _conn = session::register_zbus(session.clone())
        .await
        .context("registering dev.mackes.MDE.Session")?;

    // 3. Exec the compositor.
    let cmp = compositor_cmd();
    let user_cfg = user_sway_config_path();
    let extra_args = sway_config_args(
        &cmp,
        user_cfg.as_deref(),
        Path::new(SYSTEM_SWAY_CONFIG),
    );
    if !extra_args.is_empty() {
        tracing::info!(
            "mde-session: no ~/.config/sway/config — falling back to {SYSTEM_SWAY_CONFIG}",
        );
    }
    tracing::info!("mde-session: starting compositor {cmp}");
    let mut child = Command::new(&cmp)
        .args(&extra_args)
        .stdin(Stdio::null())
        .spawn()
        .with_context(|| format!("spawning {cmp}"))?;

    let mut sigterm = signal(SignalKind::terminate()).context("installing SIGTERM handler")?;
    let mut sigint = signal(SignalKind::interrupt()).context("installing SIGINT handler")?;

    tokio::select! {
        _ = sigterm.recv() => {
            tracing::info!("mde-session: SIGTERM received; killing compositor");
        }
        _ = sigint.recv() => {
            tracing::info!("mde-session: SIGINT received; killing compositor");
        }
        res = child.wait() => {
            match res {
                Ok(status) => tracing::info!("mde-session: compositor exited {status}"),
                Err(e)     => tracing::error!("mde-session: wait failed: {e}"),
            }
            return Ok(());
        }
    }
    let _ = child.start_kill();
    let _ = child.wait().await;
    tracing::info!("mde-session: exiting");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sway_config_args_empty_for_non_sway_compositor() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let system = tmp.path().join("config");
        std::fs::write(&system, "").unwrap();
        let user = tmp.path().join("user-missing");
        assert!(sway_config_args("i3", Some(&user), &system).is_empty());
        assert!(sway_config_args("cage", Some(&user), &system).is_empty());
    }

    #[test]
    fn sway_config_args_empty_when_user_config_exists() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let user = tmp.path().join("user-config");
        std::fs::write(&user, "").unwrap();
        let system = tmp.path().join("system-config");
        std::fs::write(&system, "").unwrap();
        assert!(sway_config_args("sway", Some(&user), &system).is_empty());
    }

    #[test]
    fn sway_config_args_empty_when_system_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let user = tmp.path().join("user-missing");
        let system = tmp.path().join("system-missing");
        assert!(sway_config_args("sway", Some(&user), &system).is_empty());
    }

    #[test]
    fn sway_config_args_returns_c_flag_when_user_missing_and_system_present() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let user = tmp.path().join("user-missing");
        let system = tmp.path().join("system-config");
        std::fs::write(&system, "").unwrap();
        let args = sway_config_args("sway", Some(&user), &system);
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], "-c");
        assert_eq!(args[1], system.to_string_lossy());
    }

    #[test]
    fn sway_config_args_returns_c_flag_when_no_user_path_at_all() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let system = tmp.path().join("system-config");
        std::fs::write(&system, "").unwrap();
        let args = sway_config_args("sway", None, &system);
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], "-c");
    }

    #[test]
    fn system_sway_config_constant_points_at_install_path() {
        assert_eq!(SYSTEM_SWAY_CONFIG, "/usr/share/mde/sway/config");
    }
}
