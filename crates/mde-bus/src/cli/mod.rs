//! BUS-1.8 — `mde-bus` CLI surface.
//!
//! Centralises every operator-facing subcommand the binary
//! exposes. The `mde-bus` entry point is a thin shim over
//! [`run`] — keeps clap setup + tracing init + dispatch in one
//! testable place rather than scattered across `main.rs`.
//!
//! Verbs (one file each):
//!
//! - `publish` — write a message to a topic + forward to the
//!   local ntfy broker. Accepts the body as positional arg,
//!   `--body` flag, or piped stdin (three publish forms per the
//!   BUS-1.8 task body).
//! - `tail` — follow messages on a topic (or wildcard pattern)
//!   by polling the SQLite index since a cursor. Exits cleanly
//!   on Ctrl-C.
//! - `sub` — add / remove / list subscriptions in the per-peer
//!   `~/.local/share/mde/bus/subs.yaml`.
//! - `mute` — add / remove / list mute patterns in the same file.
//! - `history` — print the last N messages on a topic.
//! - `topic` — list every known topic (with priority +
//!   description) or match a wildcard pattern.
//! - `daemon` — run the long-lived bus daemon (broker + mDNS +
//!   subs watcher + hooks listener). Moved here from main.rs
//!   so tests can exercise its skip semantics without exec.
//! - `render` — render a Tera template against live mesh vars.

pub mod correlate;
pub mod dnd;
pub mod history;
pub mod mute;
pub mod publish;
pub mod sub;
pub mod tail;
pub mod topic;

use clap::{Parser, Subcommand};

/// Top-level `mde-bus` CLI parser.
#[derive(Parser, Debug)]
#[command(
    name = "mde-bus",
    version,
    about = "Mackes Bus — mesh-wide notification + clipboard pub/sub bus"
)]
pub struct Cli {
    /// Subcommand. When omitted, behaves as `daemon`.
    #[command(subcommand)]
    pub cmd: Option<Cmd>,
}

/// Top-level subcommand enum.
#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Run the bus daemon. Seeds default topics on first launch,
    /// spawns the ntfy broker + mDNS + subs watcher + webhook
    /// listener, then idles. Exits cleanly on SIGINT / SIGTERM.
    Daemon,
    /// Render a Tera template against live mesh variables and
    /// print the result. Useful for debugging mesh-variable
    /// resolution.
    Render {
        /// The template body. Use single quotes in the shell to
        /// avoid `{{` getting eaten.
        template: String,
    },
    /// Publish a new message to a topic.
    Publish(publish::PublishArgs),
    /// Follow new messages on a topic or wildcard pattern.
    Tail(tail::TailArgs),
    /// Manage per-peer topic subscriptions.
    Sub {
        #[command(subcommand)]
        op: sub::SubOp,
    },
    /// Manage per-peer topic mute patterns.
    Mute {
        #[command(subcommand)]
        op: mute::MuteOp,
    },
    /// Print the last N messages on a topic.
    History(history::HistoryArgs),
    /// List or match topics in the registry.
    Topic {
        #[command(subcommand)]
        op: topic::TopicOp,
    },
    /// Toggle / inspect the mesh-wide Do Not Disturb state
    /// (BUS-2.8). Writes `<bus_root>/dnd.yaml` for `on` / `off`;
    /// `status` reads + prints the current value.
    Dnd {
        #[command(subcommand)]
        op: dnd::DndOp,
    },
    /// Inspect the operator's cross-topic correlation rule config
    /// (BUS-6.5; `$XDG_CONFIG_HOME/mde/bus-correlate.yaml`).
    /// Read-only — synthesizing publishes from rule fires is the
    /// BUS-6.5.evaluator follow-on.
    Correlate {
        #[command(subcommand)]
        op: correlate::CorrelateOp,
    },
}
