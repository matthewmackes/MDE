//! `mde-bus sub` — manage per-peer topic subscriptions in
//! `~/.local/share/mde/bus/subs.yaml`.
//!
//! Three sub-verbs:
//!
//! - `add <topic>` — append the topic pattern to `subs.topics`.
//!   Idempotent (no-op if already present). MQTT wildcards
//!   (`+` / `#`) are allowed.
//! - `remove <topic>` — drop the topic pattern from
//!   `subs.topics`. No-op when not present.
//! - `list` — print the current topic list, one per line.
//!
//! The verb reads + parses the per-peer manifest, mutates the
//! topics list, and atomically rewrites the file via the
//! BUS-1.7 `subs::to_yaml` round-trip. The daemon's
//! [`subs::SubsWatcher`] picks up the change within ~100 ms.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::Subcommand;

use crate::subs::{self, SubsManifest};

/// CLI sub-verbs for `mde-bus sub`.
#[derive(Subcommand, Debug)]
pub enum SubOp {
    /// Add a topic pattern to the subscription list.
    Add {
        /// Topic or wildcard pattern (e.g. `fleet/+` or `gh/#`).
        topic: String,
        /// Override the manifest path (defaults to
        /// `<XDG_DATA_HOME>/mde/bus/subs.yaml`).
        #[arg(long)]
        manifest: Option<PathBuf>,
    },
    /// Remove a topic pattern.
    Remove {
        /// Topic or wildcard pattern to remove.
        topic: String,
        /// Override the manifest path.
        #[arg(long)]
        manifest: Option<PathBuf>,
    },
    /// Print the current subscription list.
    List {
        /// Override the manifest path.
        #[arg(long)]
        manifest: Option<PathBuf>,
    },
}

fn resolve_manifest_path(arg: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = arg {
        return Ok(p);
    }
    subs::default_per_peer_path()
        .ok_or_else(|| anyhow!("no $HOME / $XDG_DATA_HOME — pass --manifest"))
}

fn read_or_default(path: &std::path::Path) -> Result<SubsManifest> {
    if !path.exists() {
        return Ok(SubsManifest::default());
    }
    let body = std::fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    let m = SubsManifest::parse_yaml(&body)
        .with_context(|| format!("parse {}", path.display()))?;
    Ok(m)
}

fn write_atomic(path: &std::path::Path, m: &SubsManifest) -> Result<()> {
    let body = m.to_yaml().context("encode subs.yaml")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let tmp = path.with_extension("yaml.tmp");
    std::fs::write(&tmp, body.as_bytes())
        .with_context(|| format!("write {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| {
        format!("rename {} → {}", tmp.display(), path.display())
    })?;
    Ok(())
}

/// Execute the `sub` verb.
pub async fn run(op: SubOp) -> Result<()> {
    match op {
        SubOp::Add { topic, manifest } => {
            let path = resolve_manifest_path(manifest)?;
            let mut m = read_or_default(&path)?;
            if !m.topics.iter().any(|t| t == &topic) {
                m.topics.push(topic.clone());
                m.topics.sort();
                m.topics.dedup();
                write_atomic(&path, &m)?;
                println!("subscribed: {topic}");
            } else {
                println!("already subscribed: {topic}");
            }
        }
        SubOp::Remove { topic, manifest } => {
            let path = resolve_manifest_path(manifest)?;
            let mut m = read_or_default(&path)?;
            let before = m.topics.len();
            m.topics.retain(|t| t != &topic);
            if m.topics.len() != before {
                write_atomic(&path, &m)?;
                println!("unsubscribed: {topic}");
            } else {
                println!("not subscribed: {topic}");
            }
        }
        SubOp::List { manifest } => {
            let path = resolve_manifest_path(manifest)?;
            let m = read_or_default(&path)?;
            for t in &m.topics {
                println!("{t}");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_manifest() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("subs.yaml");
        (tmp, path)
    }

    #[tokio::test]
    async fn add_inserts_into_topics() {
        let (_tmp, path) = tmp_manifest();
        run(SubOp::Add {
            topic: "fleet/+".to_string(),
            manifest: Some(path.clone()),
        })
        .await
        .unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        let m = SubsManifest::parse_yaml(&body).unwrap();
        assert!(m.topics.iter().any(|t| t == "fleet/+"));
    }

    #[tokio::test]
    async fn add_is_idempotent() {
        let (_tmp, path) = tmp_manifest();
        for _ in 0..3 {
            run(SubOp::Add {
                topic: "fleet/+".to_string(),
                manifest: Some(path.clone()),
            })
            .await
            .unwrap();
        }
        let body = std::fs::read_to_string(&path).unwrap();
        let m = SubsManifest::parse_yaml(&body).unwrap();
        assert_eq!(
            m.topics.iter().filter(|t| *t == "fleet/+").count(),
            1,
            "add should be idempotent"
        );
    }

    #[tokio::test]
    async fn remove_drops_from_topics() {
        let (_tmp, path) = tmp_manifest();
        run(SubOp::Add {
            topic: "a/b".to_string(),
            manifest: Some(path.clone()),
        })
        .await
        .unwrap();
        run(SubOp::Remove {
            topic: "a/b".to_string(),
            manifest: Some(path.clone()),
        })
        .await
        .unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        let m = SubsManifest::parse_yaml(&body).unwrap();
        assert!(!m.topics.iter().any(|t| t == "a/b"));
    }

    #[tokio::test]
    async fn remove_missing_is_noop() {
        let (_tmp, path) = tmp_manifest();
        // Manifest doesn't even exist yet.
        run(SubOp::Remove {
            topic: "never/seen".to_string(),
            manifest: Some(path.clone()),
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn list_prints_topics_without_error() {
        let (_tmp, path) = tmp_manifest();
        run(SubOp::Add {
            topic: "x".to_string(),
            manifest: Some(path.clone()),
        })
        .await
        .unwrap();
        // Just verify it runs without panicking — stdout isn't
        // captured here.
        run(SubOp::List {
            manifest: Some(path),
        })
        .await
        .unwrap();
    }
}
