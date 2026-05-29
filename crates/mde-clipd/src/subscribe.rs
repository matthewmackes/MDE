//! BUS-5.4 — inbound `clipboard/sync` subscriber.
//!
//! `spawn_subscriber()` launches a background thread that polls the
//! `clipboard/sync` bus topic tree for new messages at 1 s intervals.
//! Messages from this peer are skipped (dedup by `publisher_peer`).
//! Inbound payloads are decoded and applied to the local Wayland clipboard
//! via `wl-copy --type <mime>`.
//!
//! The two-peer round-trip test (copy on peer A → paste on peer B) is a
//! bench item gated on HW-3. Unit tests cover dedup, ULID collection,
//! and payload decode.

use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use base64::Engine as _;

use crate::publish::{ClipboardPayload, ClipboardSyncMsg, CLIPBOARD_TOPIC};

/// Poll interval for the background subscriber thread.
const POLL_INTERVAL: Duration = Duration::from_secs(1);

// ── Public entry point ─────────────────────────────────────────────────────

/// Spawn the subscriber loop as a daemon thread.
///
/// The thread polls `<bus_root>/clipboard/sync/` every second, applies new
/// inbound clipboard events via `wl-copy`, and skips messages whose
/// `publisher_peer` matches `local_peer_id`.
pub fn spawn_subscriber(bus_root: PathBuf, local_peer_id: String) {
    std::thread::Builder::new()
        .name("clipd-subscriber".into())
        .spawn(move || run_subscriber_loop(&bus_root, &local_peer_id))
        .expect("spawn clipd-subscriber thread");
}

// ── Subscriber loop ────────────────────────────────────────────────────────

fn run_subscriber_loop(bus_root: &Path, local_peer: &str) {
    let topic_dir = bus_root.join(CLIPBOARD_TOPIC);
    let mut seen: BTreeSet<String> = BTreeSet::new();

    // Seed `seen` with existing ULIDs so startup doesn't replay history.
    if let Ok(existing) = collect_ulids(&topic_dir) {
        seen.extend(existing);
    }
    tracing::info!(
        seeded = seen.len(),
        "clipboard/subscriber: started — watching clipboard/sync"
    );

    loop {
        std::thread::sleep(POLL_INTERVAL);

        let ulids = match collect_ulids(&topic_dir) {
            Ok(u) => u,
            Err(e) => {
                tracing::debug!(error = %e, "clipboard/subscriber: readdir failed — waiting");
                continue;
            }
        };

        for ulid in &ulids {
            if !seen.contains(ulid) {
                seen.insert(ulid.clone());
                let msg_path = topic_dir.join(format!("{ulid}.json"));
                process_message(&msg_path, local_peer);
            }
        }
    }
}

// ── Message processing ─────────────────────────────────────────────────────

/// List all non-tmp ULID stems present in `topic_dir`. Returns an empty
/// set if the directory does not exist yet (first-run path).
pub(crate) fn collect_ulids(topic_dir: &Path) -> anyhow::Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    if !topic_dir.exists() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(topic_dir)? {
        let name = entry?.file_name().to_string_lossy().to_string();
        if name.ends_with(".json") && !name.ends_with(".tmp.json") {
            if let Some(ulid) = name.strip_suffix(".json") {
                out.insert(ulid.to_string());
            }
        }
    }
    Ok(out)
}

fn process_message(msg_path: &Path, local_peer: &str) {
    let content = match std::fs::read_to_string(msg_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(path = %msg_path.display(), error = %e, "clipboard/subscriber: read failed");
            return;
        }
    };

    let Some((msg, data)) = decode_message(&content, local_peer) else {
        return;
    };

    apply_clipboard(&data, &msg.selected_mime);
}

/// Parse a bus envelope JSON, return `(ClipboardSyncMsg, payload_bytes)` if
/// the message is from a different peer and the payload decodes cleanly.
/// Returns `None` on parse failure, own-peer dedup, or blob-read error.
pub(crate) fn decode_message(
    envelope_json: &str,
    local_peer: &str,
) -> Option<(ClipboardSyncMsg, Vec<u8>)> {
    let envelope: serde_json::Value = serde_json::from_str(envelope_json).ok()?;
    let body_str = envelope["body"].as_str()?;
    let msg: ClipboardSyncMsg = serde_json::from_str(body_str).ok()?;

    // Dedup: skip own-peer publishes.
    if msg.publisher_peer == local_peer {
        tracing::debug!(
            peer = %msg.publisher_peer,
            "clipboard/subscriber: skipping own publish"
        );
        return None;
    }

    let data = match &msg.payload {
        ClipboardPayload::Inline { data_b64 } => {
            base64::engine::general_purpose::STANDARD
                .decode(data_b64)
                .ok()?
        }
        ClipboardPayload::BlobRef { path, .. } => std::fs::read(path).ok()?,
    };

    Some((msg, data))
}

// ── Clipboard apply ────────────────────────────────────────────────────────

/// Write `data` to the local Wayland clipboard via `wl-copy --type <mime>`.
fn apply_clipboard(data: &[u8], mime: &str) {
    let mut child = match Command::new("wl-copy")
        .arg("--type")
        .arg(mime)
        .stdin(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "clipboard/subscriber: wl-copy spawn failed");
            return;
        }
    };

    if let Some(stdin) = child.stdin.as_mut() {
        if let Err(e) = stdin.write_all(data) {
            tracing::warn!(error = %e, "clipboard/subscriber: write to wl-copy stdin failed");
        }
    }

    match child.wait() {
        Ok(status) if status.success() => {
            tracing::info!(
                mime = %mime,
                bytes = data.len(),
                "clipboard/subscriber: applied inbound clipboard"
            );
        }
        Ok(status) => {
            tracing::warn!(
                ?status,
                mime = %mime,
                "clipboard/subscriber: wl-copy exited with error"
            );
        }
        Err(e) => {
            tracing::warn!(error = %e, "clipboard/subscriber: wait for wl-copy failed");
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::publish::{publish_clipboard, INLINE_THRESHOLD};

    fn make_inline_envelope(peer: &str, data: &[u8], mime: &str) -> String {
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path().join("bus");
        let data_home = tmp.path().to_path_buf();
        publish_clipboard(
            &bus_root,
            &data_home,
            peer,
            &[mime.to_string()],
            mime,
            data,
        )
        .unwrap();
        let topic_dir = bus_root.join("clipboard/sync");
        let e = std::fs::read_dir(&topic_dir).unwrap().next().unwrap().unwrap();
        std::fs::read_to_string(e.path()).unwrap()
    }

    #[test]
    fn collect_ulids_empty_dir_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("clipboard/sync");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(collect_ulids(&dir).unwrap().is_empty());
    }

    #[test]
    fn collect_ulids_nonexistent_dir_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("no/such/dir");
        assert!(collect_ulids(&dir).unwrap().is_empty());
    }

    #[test]
    fn collect_ulids_finds_json_files() {
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path().join("bus");
        let data_home = tmp.path().to_path_buf();

        // Three publishes → three .json files.
        for _ in 0..3 {
            publish_clipboard(
                &bus_root,
                &data_home,
                "p",
                &["text/plain".to_string()],
                "text/plain",
                b"x",
            )
            .unwrap();
        }

        let topic_dir = bus_root.join("clipboard/sync");
        let ulids = collect_ulids(&topic_dir).unwrap();
        assert_eq!(ulids.len(), 3, "expected 3 ULIDs");
        // BTreeSet is sorted — ULIDs are timestamp-ordered.
        let v: Vec<_> = ulids.into_iter().collect();
        assert!(v[0] < v[1] && v[1] < v[2], "ULIDs should be ordered");
    }

    #[test]
    fn decode_message_own_peer_returns_none() {
        let envelope = make_inline_envelope("mypeer", b"hello", "text/plain");
        // Same peer — should be skipped.
        assert!(decode_message(&envelope, "mypeer").is_none());
    }

    #[test]
    fn decode_message_foreign_peer_returns_data() {
        let envelope = make_inline_envelope("peer-b", b"hello from B", "text/plain");
        let result = decode_message(&envelope, "peer-a");
        let (msg, data) = result.expect("should decode for a different peer");
        assert_eq!(msg.publisher_peer, "peer-b");
        assert_eq!(data, b"hello from B");
    }

    #[test]
    fn decode_message_blob_ref_reads_file() {
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path().join("bus");
        let data_home = tmp.path().to_path_buf();
        let big = vec![42u8; INLINE_THRESHOLD + 1];

        publish_clipboard(
            &bus_root,
            &data_home,
            "peer-b",
            &["image/png".to_string()],
            "image/png",
            &big,
        )
        .unwrap();

        let topic_dir = bus_root.join("clipboard/sync");
        let e = std::fs::read_dir(&topic_dir).unwrap().next().unwrap().unwrap();
        let envelope = std::fs::read_to_string(e.path()).unwrap();

        let (msg, data) = decode_message(&envelope, "peer-a").unwrap();
        assert_eq!(msg.publisher_peer, "peer-b");
        assert_eq!(data.len(), big.len());
        assert!(matches!(
            msg.payload,
            ClipboardPayload::BlobRef { .. }
        ));
    }

    #[test]
    fn decode_message_malformed_returns_none() {
        assert!(decode_message("not json", "p").is_none());
        assert!(decode_message("{}", "p").is_none());
        assert!(decode_message(r#"{"ulid":"x","body":"bad"}"#, "p").is_none());
    }
}
