//! `mde-bus` binary — first foundation pass (BUS-1.1 + BUS-1.6 + BUS-1.10).
//!
//! This entry point ships END-TO-END per §0.12: it is invocable, it
//! does real work for every subcommand it advertises, and the
//! `daemon` mode actually runs an idle loop that responds to
//! shutdown signals. Subsequent BUS-1.* tasks layer broker
//! supervision (BUS-1.2), mDNS (BUS-1.3), persistence (BUS-1.4),
//! subscription manifest (BUS-1.7), and the full publish/tail
//! verbs (BUS-1.8) on top.
//!
//! Subcommands available in this pass:
//! - `mde-bus daemon` — initialise registry + seed defaults + idle.
//! - `mde-bus topic list` — print every known topic to stdout.
//! - `mde-bus render <template>` — render a Tera template against
//!   the live mesh variables (BUS-1.10 acceptance exit).
//!
//! Use `RUST_LOG=mde_bus=debug,info` for verbose tracing.

use std::time::Duration;

use clap::{Parser, Subcommand};

use mde_bus::{seed, template::Renderer, topic::Registry};

#[derive(Parser, Debug)]
#[command(
    name = "mde-bus",
    version,
    about = "Mackes Bus — mesh-wide notification + clipboard pub/sub bus"
)]
struct Cli {
    /// Subcommand. When omitted, behaves as `daemon`.
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Run the bus daemon. Seeds default topics on first launch,
    /// then idles. Exits cleanly on SIGINT / SIGTERM.
    Daemon,
    /// Topic operations.
    Topic {
        #[command(subcommand)]
        op: TopicOp,
    },
    /// Render a Tera template against live mesh variables and print
    /// the result. Useful for `mde-bus publish --template …` plumbing
    /// and for debugging mesh-variable resolution from the CLI.
    Render {
        /// The template body. Use single quotes in the shell to
        /// avoid `{{` getting eaten.
        template: String,
    },
}

#[derive(Subcommand, Debug)]
enum TopicOp {
    /// Print every known topic, one per line.
    List,
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("mde_bus=info,warn"));
    let fmt = tracing_subscriber::fmt::layer()
        .json()
        .with_target(true)
        .with_current_span(false);
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    tracing_subscriber::registry().with(filter).with(fmt).init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let cli = Cli::parse();
    match cli.cmd.unwrap_or(Cmd::Daemon) {
        Cmd::Daemon => run_daemon().await,
        Cmd::Topic { op } => run_topic(op),
        Cmd::Render { template } => run_render(&template),
    }
}

fn build_seeded_registry() -> anyhow::Result<Registry> {
    let mut reg = Registry::new();
    let created = seed::seed_defaults(&mut reg)?;
    tracing::info!(
        topics_seeded = created,
        topics_total = reg.len(),
        "registry initialised"
    );
    Ok(reg)
}

async fn run_daemon() -> anyhow::Result<()> {
    let reg = build_seeded_registry()?;
    tracing::info!(
        topics = reg.len(),
        "mackes-bus daemon ready; awaiting shutdown signal"
    );
    // Heartbeat tick — every 60s log a single line so operators can
    // see the daemon is alive in `journalctl -u mde-bus`.
    let mut ticker = tokio::time::interval(Duration::from_secs(60));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("SIGINT received; shutting down");
                break;
            }
            _ = sigterm.recv() => {
                tracing::info!("SIGTERM received; shutting down");
                break;
            }
            _ = ticker.tick() => {
                tracing::info!(topics = reg.len(), "heartbeat");
            }
        }
    }
    Ok(())
}

fn run_topic(op: TopicOp) -> anyhow::Result<()> {
    let reg = build_seeded_registry()?;
    match op {
        TopicOp::List => {
            for t in reg.iter() {
                println!("{}\t{:?}\t{}", t.name, t.priority_default, t.description);
            }
        }
    }
    Ok(())
}

fn run_render(template: &str) -> anyhow::Result<()> {
    let r = Renderer::new();
    let out = r.render(template)?;
    println!("{out}");
    Ok(())
}
