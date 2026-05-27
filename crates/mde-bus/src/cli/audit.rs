//! `mde-bus audit` — inspect the per-peer publish audit log
//! (BUS-7.1). Read-only operator-facing inspection of the
//! `<bus_root>/audit/<YYYY-MM-DD>.jsonl` files.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::Subcommand;

use crate::audit;

/// CLI sub-verbs for `mde-bus audit`.
#[derive(Subcommand, Debug)]
pub enum AuditOp {
    /// Print every audit entry from oldest to newest. Optionally
    /// limit to the last N entries via `--tail N`.
    List {
        /// Override the bus_root path.
        #[arg(long)]
        bus_root: Option<PathBuf>,
        /// Print only the last N entries (most recent). 0 = all.
        #[arg(long, default_value_t = 0)]
        tail: usize,
    },
}

fn resolve_bus_root(arg: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = arg {
        return Ok(p);
    }
    crate::default_data_dir()
        .ok_or_else(|| anyhow!("no $HOME / $XDG_DATA_HOME — pass --bus-root"))
}

/// Execute the `audit` verb. Read-only.
pub fn run(op: AuditOp) -> Result<()> {
    match op {
        AuditOp::List { bus_root, tail } => {
            let root = resolve_bus_root(bus_root)?;
            let entries = audit::read_entries(&root)
                .with_context(|| format!("read audit at {}", root.display()))?;
            let slice: &[_] = if tail == 0 || tail >= entries.len() {
                &entries[..]
            } else {
                &entries[entries.len() - tail..]
            };
            for e in slice {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    e.ts_iso, e.publisher, e.topic, e.priority, e.ulid,
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_with_missing_dir_returns_ok_empty() {
        let tmp = std::env::temp_dir().join(format!("mde-bus-audit-cli-empty-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let r = run(AuditOp::List { bus_root: Some(tmp.clone()), tail: 0 });
        assert!(r.is_ok());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
