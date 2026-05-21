//! `mde-panel` binary entry — Phase E.1 skeleton.
//!
//! Launches the Iced panel application. Phase E.2 will wrap this
//! with a `wlr-layer-shell-v1` anchor so the panel pins to the
//! bottom edge with a 40 px exclusive zone.
//!
//! CLI surface (lands per-port):
//! - `--apple-menu`  → Phase E.12 popover
//! - `--expose`      → Phase E.4.4 grid
//! - `--drawer`      → Phase E.8 quick-actions drawer
//! - `--recover`     → Phase E.24 birthright rollback CLI
//! - `--root-menu`   → Phase E.14 wallpaper-area right-click
//! - `--focus <slug>` → Phase E.15 status-cluster click hand-off
//!
//! The skeleton accepts these flags but routes every flag (except
//! `--recover`) into the same Iced app for now — per-port
//! implementations swap in dedicated sub-binaries later.

#![forbid(unsafe_code)]

use clap::Parser;
use tracing::info;

#[derive(Parser, Debug)]
#[command(
    name = "mde-panel",
    about = "Mackes Desktop Environment (MDE) panel — Iced top bar + bottom dock"
)]
struct Cli {
    /// Open the apple-menu popover (Phase E.12).
    #[arg(long, conflicts_with_all = ["expose", "drawer", "recover", "root_menu", "focus"])]
    apple_menu: bool,

    /// Open the exposé grid (Phase E.4.4).
    #[arg(long, conflicts_with_all = ["apple_menu", "drawer", "recover", "root_menu", "focus"])]
    expose: bool,

    /// Open the quick-actions drawer (Phase E.8).
    #[arg(long, conflicts_with_all = ["apple_menu", "expose", "recover", "root_menu", "focus"])]
    drawer: bool,

    /// Print the birthright-rollback preview and exit (Phase E.24).
    #[arg(long, conflicts_with_all = ["apple_menu", "expose", "drawer", "root_menu", "focus"])]
    recover: bool,

    /// Open the wallpaper-area right-click menu (Phase E.14).
    #[arg(long = "root-menu", conflicts_with_all = ["apple_menu", "expose", "drawer", "recover", "focus"])]
    root_menu: bool,

    /// Hand a focus slug to the Workbench (E.15 click target).
    #[arg(long, conflicts_with_all = ["apple_menu", "expose", "drawer", "recover", "root_menu"])]
    focus: Option<String>,
}

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("MDE_PANEL_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("mde_panel=info,warn")),
        )
        .json()
        .init();

    let cli = Cli::parse();

    if cli.recover {
        info!("mde-panel --recover — Phase E.24 stub (no rollback payload yet)");
        println!("mde-panel --recover: rollback preview unavailable until Phase E.24 lands.");
        return Ok(());
    }

    if cli.apple_menu || cli.expose || cli.drawer || cli.root_menu || cli.focus.is_some() {
        info!(
            apple_menu = cli.apple_menu,
            expose = cli.expose,
            drawer = cli.drawer,
            root_menu = cli.root_menu,
            focus = cli.focus.as_deref().unwrap_or(""),
            "subcommand requested — Phase E port pending; falling through to main app"
        );
    }

    info!("starting Iced panel app");
    mde_panel::App::run()
}
