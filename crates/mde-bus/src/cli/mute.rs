//! `mde-bus mute` — manage per-peer topic mute patterns in
//! `~/.local/share/mde/bus/subs.yaml`.
//!
//! Mute patterns silence a topic even when it matches the
//! subscribe list. Useful for narrowing noisy sources without
//! unsubscribing entirely.
//!
//! Three sub-verbs (mirror the `sub` verb shape so operators
//! don't have to learn two layouts): `add`, `remove`, `list`.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::Subcommand;

use crate::subs::{self, SubsManifest};

/// CLI sub-verbs for `mde-bus mute`.
#[derive(Subcommand, Debug)]
pub enum MuteOp {
    /// Add a mute pattern.
    Add {
        /// Topic or wildcard pattern to mute.
        topic: String,
        /// Override the manifest path.
        #[arg(long)]
        manifest: Option<PathBuf>,
    },
    /// Remove a mute pattern.
    Remove {
        /// Topic or wildcard pattern to unmute.
        topic: String,
        /// Override the manifest path.
        #[arg(long)]
        manifest: Option<PathBuf>,
    },
    /// Print the current mute list.
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
    SubsManifest::parse_yaml(&body).with_context(|| format!("parse {}", path.display()))
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

/// Execute the `mute` verb.
pub async fn run(op: MuteOp) -> Result<()> {
    match op {
        MuteOp::Add { topic, manifest } => {
            let path = resolve_manifest_path(manifest)?;
            let mut m = read_or_default(&path)?;
            if !m.mute.iter().any(|t| t == &topic) {
                m.mute.push(topic.clone());
                m.mute.sort();
                m.mute.dedup();
                write_atomic(&path, &m)?;
                println!("muted: {topic}");
            } else {
                println!("already muted: {topic}");
            }
        }
        MuteOp::Remove { topic, manifest } => {
            let path = resolve_manifest_path(manifest)?;
            let mut m = read_or_default(&path)?;
            let before = m.mute.len();
            m.mute.retain(|t| t != &topic);
            if m.mute.len() != before {
                write_atomic(&path, &m)?;
                println!("unmuted: {topic}");
            } else {
                println!("not muted: {topic}");
            }
        }
        MuteOp::List { manifest } => {
            let path = resolve_manifest_path(manifest)?;
            let m = read_or_default(&path)?;
            for t in &m.mute {
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
    async fn add_inserts_into_mute() {
        let (_tmp, path) = tmp_manifest();
        run(MuteOp::Add {
            topic: "noisy/+".to_string(),
            manifest: Some(path.clone()),
        })
        .await
        .unwrap();
        let m = SubsManifest::parse_yaml(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(m.mute.iter().any(|t| t == "noisy/+"));
    }

    #[tokio::test]
    async fn remove_drops_from_mute() {
        let (_tmp, path) = tmp_manifest();
        run(MuteOp::Add {
            topic: "x".to_string(),
            manifest: Some(path.clone()),
        })
        .await
        .unwrap();
        run(MuteOp::Remove {
            topic: "x".to_string(),
            manifest: Some(path.clone()),
        })
        .await
        .unwrap();
        let m = SubsManifest::parse_yaml(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(!m.mute.iter().any(|t| t == "x"));
    }

    #[tokio::test]
    async fn add_is_idempotent() {
        let (_tmp, path) = tmp_manifest();
        for _ in 0..3 {
            run(MuteOp::Add {
                topic: "x".to_string(),
                manifest: Some(path.clone()),
            })
            .await
            .unwrap();
        }
        let m = SubsManifest::parse_yaml(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(m.mute.iter().filter(|t| *t == "x").count(), 1);
    }
}
