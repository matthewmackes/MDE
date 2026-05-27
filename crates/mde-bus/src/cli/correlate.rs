//! `mde-bus correlate` — inspect the operator's correlation rule
//! config (`$XDG_CONFIG_HOME/mde/bus-correlate.yaml`).
//!
//! Two sub-verbs:
//!
//! - `list` — print every configured rule as
//!   `<name>\t<sources joined by ,>\t<window_seconds>s\t<emits>\t<priority>`
//!   (TSV-friendly). Useful for verifying a rule was picked up
//!   after editing the YAML.
//! - `path` — print the resolved config-file path. Operators who
//!   don't remember where the config lives can run this to find
//!   the right file before editing.
//!
//! Synthesizing publishes from rule fires is the
//! BUS-6.5.evaluator follow-on; this verb is read-only.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::Subcommand;

use crate::correlate;

/// CLI sub-verbs for `mde-bus correlate`.
#[derive(Subcommand, Debug)]
pub enum CorrelateOp {
    /// Print every configured rule.
    List {
        /// Override the config path.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Print the resolved config path.
    Path,
}

fn resolve_config_path(arg: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = arg {
        return Ok(p);
    }
    correlate::default_config_path()
        .ok_or_else(|| anyhow!("no $HOME / $XDG_CONFIG_HOME — pass --config"))
}

/// Execute the `correlate` verb. Read-only — never writes.
pub fn run(op: CorrelateOp) -> Result<()> {
    match op {
        CorrelateOp::List { config } => {
            let path = resolve_config_path(config)?;
            let cfg = correlate::load_default(&path)
                .with_context(|| format!("load {}", path.display()))?;
            for rule in &cfg.rules {
                println!(
                    "{}\t{}\t{}s\t{}\t{:?}",
                    rule.name,
                    rule.sources.join(","),
                    rule.window_seconds,
                    rule.emits,
                    rule.priority,
                );
            }
        }
        CorrelateOp::Path => {
            let path = correlate::default_config_path()
                .ok_or_else(|| anyhow!("no $HOME / $XDG_CONFIG_HOME"))?;
            println!("{}", path.display());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_with_missing_file_returns_empty_ok() {
        // Missing file is the most common case (operator hasn't
        // configured correlation yet) — must not error.
        let p = std::path::PathBuf::from("/nonexistent/path/bus-correlate.yaml");
        let r = run(CorrelateOp::List { config: Some(p) });
        assert!(r.is_ok());
    }

    #[test]
    fn list_with_existing_config_succeeds() {
        let tmp = std::env::temp_dir().join(format!("mde-bus-correlate-cli-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("bus-correlate.yaml");
        std::fs::write(
            &path,
            "rules:\n  - name: power-outage\n    sources: [a, b]\n    window_seconds: 60\n    emits: incident/outage\n    priority: high\n",
        )
        .unwrap();
        let r = run(CorrelateOp::List { config: Some(path) });
        assert!(r.is_ok());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn path_returns_ok_when_xdg_resolves() {
        // path verb returns Ok as long as dirs::config_dir() does
        // — on most test environments this succeeds.
        if correlate::default_config_path().is_some() {
            assert!(run(CorrelateOp::Path).is_ok());
        }
    }
}
