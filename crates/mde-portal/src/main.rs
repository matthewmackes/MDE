//! `mde-portal` — v6.0 unified shell (Portal-3: font + icon theme).
//!
//! Single Rust binary that hosts all six Wayland surfaces:
//!   Dock / Portal-compact / Portal-full / Lock / Theater / Mesh-wallpaper
//!
//! Portal-3 adds the font stack: Intel One Mono as the Iced default
//! font (resolved via fontconfig, installed by `intel-one-mono-fonts`),
//! Symbols Nerd Font Mono as icon-glyph fallback, and the Material
//! Symbols icon system wired via `mde_theme::mde_icon` /
//! `fonts::resolve_icon`.
//! Portal-4 onward uses `resolve_icon` for nav-button glyphs.
//!
//! Portal-2 ships the Dock (AllScreens, 56 px, theme-adaptive bg).
//! Portal-1 ships `dev.mackes.MDE.Portal` D-Bus registration.
//!
//! **Supervision:** `mde-portal.service` (systemd user unit) is
//! `WantedBy=graphical-session.target` so the session manager starts
//! and restarts it automatically. mackesd provides the data surfaces
//! (Nebula.Status, Gluster.Status, Shell.*) that the portal consumes,
//! and can call `dev.mackes.MDE.Portal.{Goto,Focus,Lock,ToggleDND}`
//! for daemon-driven events (idle-lock, mesh alerts, etc.).

#![forbid(unsafe_code)]

use anyhow::Context as _;
use clap::Parser;

mod app;
// Portal-31 — startup scan of ~/.local/share/mde/cards/ + one-line summary log.
pub mod card_index;
pub mod dbus;
// Portal-3 — font + Material Symbols icon theme layer.
pub mod fonts;
// Portal-9.a — sysfs status polling (battery / network / backlight).
pub mod status;
// Portal-14.a (R4-Q22, 2026-05-27) — typewriter reveal primitive.
// Pure helpers consumed by BUS-2.2 + Portal-57.c + future breadcrumb
// segments to char-by-char-reveal new content.
pub mod typewriter;
// Portal-14.d (R4-Q91, 2026-05-27) — breath-line gradient sweep.
// Pure helpers consumed by the Dock view to render the 15-second
// hue-sweep baseline below the breadcrumb row.
pub mod breath_line;
// Portal-14.c (R4-Q60, 2026-05-27) — marquee scroll for long labels.
// Pure helpers consumed by breadcrumb segments to scroll labels
// exceeding the segment's viewport.
pub mod marquee;
// Portal-35 — `mde://` URI scheme parser + action dispatcher.
pub mod uri;
// Portal-5 — swayipc workspace integration.
pub mod workspace;

/// CLI surface for `mde-portal`.
#[derive(Parser, Debug)]
#[command(
    name = "mde-portal",
    about = "MDE Portal — unified shell (Dock + Portal-compact + Portal-full + surfaces)"
)]
struct Cli {
    /// Register D-Bus + exit without opening any Wayland surface.
    /// Used by CI / `mackesd portal-smoke` to verify the bus name.
    #[arg(long, default_value_t = false)]
    headless: bool,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("MDE_PORTAL_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("mde_portal=info,warn")),
        )
        .json()
        .init();

    let cli = Cli::parse();

    // Portal-31 — confirm the local card store is reachable and log
    // a one-line summary. Runs unconditionally so headless smoke
    // tests also exercise the import.
    card_index::log_summary_at_startup();

    if cli.headless {
        return run_headless();
    }

    run_layershell()
}

/// Headless mode: register D-Bus, confirm the bus name is reachable, exit.
fn run_headless() -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("building tokio runtime for headless mode")?;

    rt.block_on(async {
        let state = dbus::PortalState::new();
        let _conn = dbus::register(state)
            .await
            .context("registering dev.mackes.MDE.Portal")?;
        tracing::info!(
            object_path = "/dev/mackes/MDE/Portal",
            "mde-portal: D-Bus registered (headless); exiting"
        );
        anyhow::Ok(())
    })
}

/// Normal mode: register D-Bus in a background thread and launch the
/// Iced layer-shell Dock surface.
fn run_layershell() -> anyhow::Result<()> {
    // Spin up a separate multi-thread tokio runtime for the zbus
    // connection so D-Bus dispatch doesn't contend with the Iced
    // render thread.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("building tokio runtime")?;

    let state = dbus::PortalState::new();
    let _conn = rt.block_on(dbus::register(state)).context("registering dev.mackes.MDE.Portal")?;

    // Keep the tokio runtime alive for the process lifetime so zbus
    // tasks continue processing incoming D-Bus method calls while Iced
    // owns the main thread.
    let _rt_thread = std::thread::spawn(move || {
        rt.block_on(std::future::pending::<()>());
    });

    app::run()
        .map_err(|e| anyhow::anyhow!("running mde-portal layer-shell application: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_headless_flag() {
        let cli = Cli::try_parse_from(["mde-portal", "--headless"])
            .expect("--headless flag should parse");
        assert!(cli.headless);
    }

    #[test]
    fn cli_headless_defaults_to_false() {
        let cli = Cli::try_parse_from(["mde-portal"]).expect("no-arg parse should work");
        assert!(!cli.headless);
    }
}
