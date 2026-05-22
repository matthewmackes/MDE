//! `mde-popover` — Iced + wlr-layer-shell popover host.
//!
//! v3.0.2 panel-host wiring: the panel (`mde-panel`) spawns this
//! binary on every clickable zone press. Each popover is a separate
//! layer-shell overlay surface that anchors above the panel edge,
//! dismisses on Esc / outside-click, and exits cleanly when the user
//! commits or cancels.
//!
//! ```text
//!   mde-popover start-menu         # M button → app launcher
//!   mde-popover audio              # ♫ click → volume slider
//!   mde-popover notifications      # bell click → notification list
//!   mde-popover clock              # clock click → calendar
//!   mde-popover network            # network click → connection list
//! ```
//!
//! Only `start-menu` ships today; the rest are stubs that exit 0 so
//! the panel's click handler doesn't error. Per-kind ports land at
//! v3.1 (see v3.0.2 hotfix bundle worklist follow-ups).

#![forbid(unsafe_code)]

mod start_menu;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "mde-popover",
    about = "Mackes Desktop Environment popover overlay surfaces"
)]
struct Cli {
    /// Which popover to mount.
    #[arg(value_enum)]
    kind: Kind,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    StartMenu,
    Audio,
    Notifications,
    Clock,
    Network,
}

fn main() -> iced_layershell::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("MDE_POPOVER_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("mde_popover=info,warn")),
        )
        .json()
        .init();

    let cli = Cli::parse();
    tracing::info!(kind = ?cli.kind, "mde-popover spawned");

    match cli.kind {
        Kind::StartMenu => start_menu::run(),
        Kind::Audio | Kind::Notifications | Kind::Clock | Kind::Network => {
            // Stub kinds — emit a marker line so test harnesses can
            // confirm the dispatch path and exit 0. The full Iced UIs
            // for these land per v3.1 follow-ups.
            tracing::info!(kind = ?cli.kind, "popover kind not yet implemented; exit 0");
            Ok(())
        }
    }
}
