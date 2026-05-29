//! mde-clipd — MDE clipboard daemon (BUS-5.1/5.2).
//!
//! Connects to the Wayland compositor via `wlr-data-control-unstable-v1`,
//! watches every clipboard change, reads the content via a pipe, and
//! publishes it to the `clipboard/sync` Mackes Bus topic (BUS-5.2).
//! Supervised by the mded `clipd_supervisor` worker.

use std::path::PathBuf;

use anyhow::Context as _;
use clap::Parser;
use tracing::info;
use wayland_client::Connection;

mod blobstore;
mod proto;
mod publish;
mod session;
mod subscribe;

/// mde-clipd — MDE clipboard daemon
#[derive(Parser, Debug)]
#[command(name = "mde-clipd", about = "MDE clipboard daemon (BUS-5)")]
struct Cli {
    /// Tracing filter (e.g. "info", "debug", "mde_clipd=debug").
    #[arg(long, env = "MDE_LOG", default_value = "info")]
    log_level: String,

    /// Bus root directory (default: $XDG_DATA_HOME/mde/bus/).
    #[arg(long, env = "MDE_BUS_ROOT")]
    bus_root: Option<PathBuf>,
}

/// Resolve `$XDG_DATA_HOME` → `$HOME/.local/share` → `/var/lib`.
fn xdg_data_home() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".local/share"))
        })
        .unwrap_or_else(|| PathBuf::from("/var/lib"))
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .with_writer(std::io::stderr)
        .init();

    info!("mde-clipd: starting (BUS-5.2)");

    let data_home = xdg_data_home();
    let bus_root = cli
        .bus_root
        .unwrap_or_else(|| data_home.join("mde").join("bus"));
    let peer_id = publish::local_peer_id();

    let config = session::Config {
        bus_root,
        data_home,
        peer_id,
    };

    let conn = Connection::connect_to_env()
        .context("failed to connect to Wayland — is $WAYLAND_DISPLAY set?")?;

    session::run(&conn, &config)
}
