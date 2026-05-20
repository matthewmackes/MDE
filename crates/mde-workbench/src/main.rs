//! `mde-workbench` binary entry — single-instance handshake
//! + Iced launch.
//!
//! CB-1.13 contract: every invocation either becomes the primary
//! workbench process or hands its `--focus <slug>` argument off
//! to the already-running primary via the
//! `dev.mackes.MDE.Shell.Workbench.Focus` method.

use std::process::ExitCode;

use clap::Parser;
use mde_workbench::{
    App, BUS_NAME, INTERFACE_NAME, OBJECT_PATH, PendingFocus, PrimaryStatus,
    WorkbenchService, decide_primary_status,
};
use tracing::{debug, error, info};
use zbus::fdo::{DBusProxy, RequestNameFlags};
use zbus::{Connection, names::WellKnownName, proxy};

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

/// Client-side proxy generated from [`INTERFACE_NAME`] —
/// `mde-workbench --focus <slug>` calls this on an
/// already-running primary.
#[proxy(
    interface = "dev.mackes.MDE.Shell.Workbench",
    default_service = "dev.mackes.MDE.Workbench",
    default_path = "/dev/mackes/MDE/Workbench"
)]
trait Workbench {
    fn focus(&self, slug: &str) -> zbus::Result<()>;
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
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
            register_workbench_service(&runtime, conn)
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
/// connection (the connection stays alive in either branch —
/// the hand-off path uses it for the [`WorkbenchProxy::focus`]
/// call, the primary path keeps it for serving Focus requests).
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

/// Sibling-process branch — call Focus on the running primary
/// and exit. Returns `ExitCode::SUCCESS` when the hand-off
/// succeeded, `2` when the bus call itself failed.
fn hand_off_to_running(
    runtime: &tokio::runtime::Runtime,
    focus: &Option<String>,
) -> ExitCode {
    let slug = focus.clone().unwrap_or_default();
    info!(%slug, "primary workbench already running — handing off Focus");
    let result = runtime.block_on(async {
        let conn = Connection::session().await?;
        let proxy = WorkbenchProxy::new(&conn).await?;
        proxy.focus(&slug).await
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("hand-off Focus call failed: {e}");
            ExitCode::from(2)
        }
    }
}

/// Primary-process branch — register the
/// [`INTERFACE_NAME`] interface on the live connection at
/// [`OBJECT_PATH`] so the [`PendingFocus`] slot fills whenever
/// a sibling invocation calls Focus.
fn register_workbench_service(
    runtime: &tokio::runtime::Runtime,
    conn: Connection,
) -> Result<(), ()> {
    runtime.block_on(async {
        conn.object_server()
            .at(OBJECT_PATH, WorkbenchService)
            .await
            .map_err(|e| {
                error!("failed to register {INTERFACE_NAME} at {OBJECT_PATH}: {e}");
            })?;
        info!(%OBJECT_PATH, %INTERFACE_NAME, "primary workbench D-Bus surface registered");
        // Leak the connection — its background tokio tasks must
        // outlive this function. The runtime itself is leaked
        // by the caller via Box::leak.
        Box::leak(Box::new(conn));
        Ok(())
    })
}
