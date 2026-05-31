//! `mde-workbench` binary entry — single-instance handshake
//! + Iced launch.
//!
//! CB-1.13 contract: every invocation either becomes the primary
//! workbench process or hands its `--focus <slug>` argument off
//! to the already-running primary.
//!
//! DBUS-3 (Q96/EPIC-RETIRE-DBUS): the single-instance NAME
//! (`dev.mackes.MDE.Workbench`) is still owned on D-Bus — name
//! ownership is inherently a D-Bus/socket primitive (finding #3
//! documented exception). The `focus` hand-off itself migrated to
//! the Bus action topic `action/shell/workbench-focus` with the
//! 40 ms interactive poll (finding #1).

use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use mde_bus::hooks::config::Priority;
use mde_bus::rpc::{request_with_interval, INTERACTIVE_POLL_INTERVAL};
use mde_workbench::{
    decide_primary_status, serve_focus_bus, App, PendingFocus, PrimaryStatus, ACTION_TOPIC,
    BUS_NAME,
};
use tracing::{debug, error, info};
use zbus::fdo::{DBusProxy, RequestNameFlags};
use zbus::{names::WellKnownName, Connection};

#[derive(Parser, Debug)]
#[command(
    name = "mde-workbench",
    about = "Mackes Desktop Environment (MDE) Workbench"
)]
struct Cli {
    /// Open the workbench at the named panel
    /// (e.g. `--focus network.mesh_ssh`).
    #[arg(long)]
    focus: Option<String>,
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli = Cli::parse();
    let initial_focus = cli.focus.clone().unwrap_or_default();

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            error!("failed to build tokio runtime: {e}");
            return ExitCode::from(2);
        }
    };

    // Single-instance handshake — block on tokio for the bus
    // round-trips, then either hand off and exit, or spawn the
    // long-running zbus connection alongside the Iced loop.
    let status = match runtime.block_on(acquire_primary()) {
        Ok((status, conn)) => {
            if status == PrimaryStatus::Existing {
                drop(conn);
                return hand_off_to_running(&runtime, &cli.focus);
            }
            start_primary_focus_responder(conn)
        }
        Err(e) => {
            // Couldn't reach the session bus at all — fall back
            // to launching the workbench without single-instance
            // protection so the user isn't dead-in-the-water
            // when D-Bus is missing (e.g. early-boot recovery
            // shells). Log loudly.
            error!(
                "session bus unreachable ({e}); launching workbench without \
                 single-instance protection"
            );
            Err(())
        }
    };

    if status.is_err() {
        info!("continuing without D-Bus single-instance protection");
    }

    // Iced takes over the main thread — keep the tokio runtime
    // (and the zbus connection it owns) alive for the lifetime
    // of the process via a leaked handle.
    let _runtime = Box::leak(Box::new(runtime));

    if !initial_focus.is_empty() {
        PendingFocus::submit(initial_focus);
    }

    match App::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("iced runtime error: {e}");
            ExitCode::from(1)
        }
    }
}

/// Connect to the session bus and try to acquire [`BUS_NAME`]
/// with `DoNotQueue`, returning the resulting status + the live
/// connection. The connection is the single-instance primitive —
/// the primary path keeps it alive to retain name ownership; the
/// sibling path drops it and hands off the focus slug over the Bus.
async fn acquire_primary() -> zbus::Result<(PrimaryStatus, Connection)> {
    let conn = Connection::session().await?;
    let dbus = DBusProxy::new(&conn).await?;
    let wk = WellKnownName::try_from(BUS_NAME)?;
    let reply = dbus
        .request_name(wk, RequestNameFlags::DoNotQueue.into())
        .await?;
    let status = decide_primary_status(reply);
    debug!(?status, %BUS_NAME, "single-instance handshake complete");
    Ok((status, conn))
}

/// Sibling-process branch — publish the `--focus <slug>` request on
/// the Bus action topic the running primary serves, then exit.
/// Uses the 40 ms interactive poll so the round-trip is imperceptible
/// (finding #1). Returns `ExitCode::SUCCESS` when the primary
/// acknowledged, `2` when the Bus call itself failed.
fn hand_off_to_running(runtime: &tokio::runtime::Runtime, focus: &Option<String>) -> ExitCode {
    let slug = focus.clone().unwrap_or_default();
    info!(%slug, "primary workbench already running — handing off focus over the Bus");
    let Some(bus_root) = mde_bus::default_data_dir() else {
        error!("no Bus data dir; cannot hand off focus");
        return ExitCode::from(2);
    };
    let persist = match mde_bus::persist::Persist::open(bus_root) {
        Ok(p) => p,
        Err(e) => {
            error!("opening Bus store for focus hand-off: {e}");
            return ExitCode::from(2);
        }
    };
    let result = runtime.block_on(request_with_interval(
        &persist,
        ACTION_TOPIC,
        Priority::Default,
        None,
        Some(slug.as_str()),
        Duration::from_secs(2),
        INTERACTIVE_POLL_INTERVAL,
    ));
    match result {
        Ok(_reply) => ExitCode::SUCCESS,
        Err(e) => {
            error!("focus hand-off over the Bus failed: {e}");
            ExitCode::from(2)
        }
    }
}

/// Primary-process branch — keep the D-Bus connection alive (so we
/// retain ownership of [`BUS_NAME`], the single-instance primitive)
/// and spawn the Bus focus responder so the [`PendingFocus`] slot
/// fills whenever a sibling invocation publishes to
/// `action/shell/workbench-focus`. The responder runs on its own
/// thread because `Persist` (rusqlite) isn't `Send`.
fn start_primary_focus_responder(conn: Connection) -> Result<(), ()> {
    // No object is served on the connection anymore — only the name
    // matters. Leak it so its background tokio tasks (which keep the
    // name owned) outlive this function.
    Box::leak(Box::new(conn));
    std::thread::Builder::new()
        .name("workbench-focus-bus".into())
        .spawn(|| {
            let Some(bus_root) = mde_bus::default_data_dir() else {
                error!("workbench focus responder: no Bus data dir; --focus hand-off unavailable");
                return;
            };
            match mde_bus::persist::Persist::open(bus_root) {
                Ok(persist) => serve_focus_bus(&persist, || false),
                Err(e) => error!("workbench focus responder: opening Bus store: {e}"),
            }
        })
        .map(|_| {
            info!("primary workbench focus responder started on the Bus");
        })
        .map_err(|e| error!("spawning workbench focus responder thread: {e}"))
}
