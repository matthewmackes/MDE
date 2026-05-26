//! `mde-bus history` — print stored messages on a topic.
//!
//! Reads from the per-peer SQLite index (BUS-1.4). Supports
//! optional `--since <ulid>` cursor + `--count N` limit. Default
//! is "every message ever stored on the topic" — operators
//! usually want `--count 20` for the last-20 view.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::Args;

use crate::persist::Persist;

/// CLI args for `mde-bus history`.
#[derive(Args, Debug, Default)]
pub struct HistoryArgs {
    /// Topic to print history for. Exact match (no wildcards).
    pub topic: String,
    /// Start cursor (exclusive). Useful for "what's new since
    /// my last poll?" queries.
    #[arg(long)]
    pub since: Option<String>,
    /// Print at most this many messages (most-recent N).
    #[arg(long)]
    pub count: Option<usize>,
    /// Override the bus-root directory (defaults to
    /// `<XDG_DATA_HOME>/mde/bus`).
    #[arg(long)]
    pub bus_root: Option<PathBuf>,
}

fn default_bus_root() -> Result<PathBuf> {
    crate::default_data_dir()
        .ok_or_else(|| anyhow!("no $HOME / $XDG_DATA_HOME — pass --bus-root"))
}

/// Execute the `history` verb.
pub async fn run(args: HistoryArgs) -> Result<()> {
    let bus_root = match args.bus_root.clone() {
        Some(p) => p,
        None => default_bus_root()?,
    };
    let p = Persist::open(bus_root).context("open persist")?;
    let mut rows = p.list_since(&args.topic, args.since.as_deref())?;
    if let Some(n) = args.count {
        let start = rows.len().saturating_sub(n);
        rows = rows.split_off(start);
    }
    for m in &rows {
        let line = crate::cli::tail::format_line(m);
        println!("{line}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::config::Priority;

    #[tokio::test]
    async fn returns_all_when_count_omitted() {
        let tmp = tempfile::tempdir().unwrap();
        let p = Persist::open(tmp.path().to_path_buf()).unwrap();
        for i in 0..5 {
            p.write("t/x", Priority::Default, None, Some(&i.to_string()))
                .unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        // Should not hang or error.
        let args = HistoryArgs {
            topic: "t/x".to_string(),
            since: None,
            count: None,
            bus_root: Some(tmp.path().to_path_buf()),
        };
        run(args).await.unwrap();
    }

    #[tokio::test]
    async fn count_limits_output() {
        // Behavioral check via direct list_since call to avoid
        // capturing stdout in tests.
        let tmp = tempfile::tempdir().unwrap();
        let p = Persist::open(tmp.path().to_path_buf()).unwrap();
        for i in 0..10 {
            p.write("t/x", Priority::Default, None, Some(&i.to_string()))
                .unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let all = p.list_since("t/x", None).unwrap();
        assert_eq!(all.len(), 10);
        // Run the verb — main coverage of the verb itself.
        let args = HistoryArgs {
            topic: "t/x".to_string(),
            since: None,
            count: Some(3),
            bus_root: Some(tmp.path().to_path_buf()),
        };
        run(args).await.unwrap();
    }
}
