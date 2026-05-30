//! BUS-5.3 — clipboard blob store + GC lifecycle.
//!
//! Blobs are written by BUS-5.2 for clipboard payloads > 64 KB.
//! `gc_orphaned_blobs()` walks the blob directory, cross-references with
//! live `clipboard/sync` messages in the bus topic tree, and deletes any
//! blob that has no live reference (i.e. its referencing message was
//! retention-evicted by BUS-1.9).
//!
//! GC is called once at startup and then every 50 successful publishes
//! in the daemon's main loop.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::pin::{pin_store_path, PinStore};
use crate::publish::{blob_dir, ClipboardPayload, ClipboardSyncMsg};

/// Run one GC pass over `<data_home>/mde/clipboard/blobs/`.
///
/// Reads every `*.json` file under `<bus_root>/clipboard/sync/`, extracts
/// `BlobRef` paths, and deletes blob files not in that reference set.
/// Errors on individual deletions are logged and skipped — the next pass
/// will retry.
///
/// Returns the number of blob files deleted.
///
/// # Errors
///
/// Returns an error only when the blob directory or bus topic directory
/// cannot be listed (unexpected `readdir` failure).
pub fn gc_orphaned_blobs(bus_root: &Path, data_home: &Path) -> anyhow::Result<usize> {
    let blobs = blob_dir(data_home);
    if !blobs.exists() {
        return Ok(0);
    }

    // 1. Enumerate all blob files on disk.
    let mut on_disk: HashSet<PathBuf> = HashSet::new();
    for entry in std::fs::read_dir(&blobs)
        .map_err(|e| anyhow::anyhow!("readdir {}: {e}", blobs.display()))?
    {
        let e = entry.map_err(|e| anyhow::anyhow!("readdir entry: {e}"))?;
        if e.file_type()
            .map_err(|e| anyhow::anyhow!("file_type: {e}"))?
            .is_file()
        {
            on_disk.insert(e.path());
        }
    }

    if on_disk.is_empty() {
        return Ok(0);
    }

    // 2. Build the set of blob paths referenced by live bus messages.
    let mut referenced: HashSet<PathBuf> = HashSet::new();
    let topic_dir = bus_root.join("clipboard/sync");
    if topic_dir.exists() {
        for entry in std::fs::read_dir(&topic_dir)
            .map_err(|e| anyhow::anyhow!("readdir {}: {e}", topic_dir.display()))?
        {
            let e = entry.map_err(|e| anyhow::anyhow!("readdir entry: {e}"))?;
            let path = e.path();
            if path.extension().map_or(false, |x| x == "json") {
                if let Some(blob_path) = extract_blob_ref(&path) {
                    referenced.insert(blob_path);
                }
            }
        }
    }

    // 3. BUS-5.7 pin protection: blobs referenced by pinned entries survive
    //    even when BUS-1.9 retention-evicts the bus message. We attempt to
    //    read the message path for each pinned ULID and add its BlobRef (if
    //    any) to the referenced set. If the message is already gone there is
    //    nothing to protect — the blob path is irrecoverable from the ULID
    //    alone without the message.
    let pin_path = pin_store_path(data_home);
    let pins = PinStore::load_from(&pin_path);
    let topic_dir_for_pins = bus_root.join("clipboard/sync");
    for ulid in &pins.pinned {
        let msg = topic_dir_for_pins.join(format!("{ulid}.json"));
        if let Some(blob_path) = extract_blob_ref(&msg) {
            referenced.insert(blob_path);
        }
    }

    // 4. Delete unreferenced blobs.
    let mut deleted = 0;
    for blob_path in &on_disk {
        if !referenced.contains(blob_path) {
            match std::fs::remove_file(blob_path) {
                Ok(()) => {
                    tracing::info!(
                        path = %blob_path.display(),
                        "clipboard: GC deleted orphan blob"
                    );
                    deleted += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        path = %blob_path.display(),
                        error = %e,
                        "clipboard: GC failed to delete orphan blob — will retry next pass"
                    );
                }
            }
        }
    }

    Ok(deleted)
}

/// Parse a bus envelope file and return the `BlobRef` path if the payload
/// is a large-payload reference; returns `None` for inline payloads or
/// unreadable/malformed files.
fn extract_blob_ref(msg_path: &Path) -> Option<PathBuf> {
    let content = std::fs::read_to_string(msg_path).ok()?;
    let envelope: serde_json::Value = serde_json::from_str(&content).ok()?;
    let body_str = envelope["body"].as_str()?;
    let msg: ClipboardSyncMsg = serde_json::from_str(body_str).ok()?;
    match msg.payload {
        ClipboardPayload::BlobRef { path, .. } => Some(PathBuf::from(path)),
        ClipboardPayload::Inline { .. } => None,
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pin::PinStore;
    use crate::publish::{publish_clipboard, INLINE_THRESHOLD};

    #[test]
    fn gc_no_blob_dir_returns_zero() {
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path().join("bus");
        let data_home = tmp.path().to_path_buf();
        // No blob directory — GC is a no-op.
        assert_eq!(gc_orphaned_blobs(&bus_root, &data_home).unwrap(), 0);
    }

    #[test]
    fn gc_empty_blob_dir_returns_zero() {
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path().join("bus");
        let data_home = tmp.path().to_path_buf();
        std::fs::create_dir_all(blob_dir(&data_home)).unwrap();
        assert_eq!(gc_orphaned_blobs(&bus_root, &data_home).unwrap(), 0);
    }

    #[test]
    fn orphan_blob_is_deleted() {
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path().join("bus");
        let data_home = tmp.path().to_path_buf();

        let blobs = blob_dir(&data_home);
        std::fs::create_dir_all(&blobs).unwrap();
        let orphan = blobs.join("01ORPHANBLOB00000000000000.txt");
        std::fs::write(&orphan, b"orphan content").unwrap();

        assert_eq!(gc_orphaned_blobs(&bus_root, &data_home).unwrap(), 1);
        assert!(!orphan.exists(), "orphan must be deleted");
    }

    #[test]
    fn live_blob_survives_gc() {
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path().join("bus");
        let data_home = tmp.path().to_path_buf();
        let big = vec![0u8; INLINE_THRESHOLD + 1];

        publish_clipboard(
            &bus_root,
            &data_home,
            "p",
            &["image/png".to_string()],
            "image/png",
            &big,
        )
        .unwrap();

        let blobs = blob_dir(&data_home);
        assert_eq!(std::fs::read_dir(&blobs).unwrap().count(), 1);

        assert_eq!(gc_orphaned_blobs(&bus_root, &data_home).unwrap(), 0);

        assert_eq!(
            std::fs::read_dir(&blobs).unwrap().count(),
            1,
            "live blob must survive"
        );
    }

    #[test]
    fn orphan_among_live_blobs_only_orphan_deleted() {
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path().join("bus");
        let data_home = tmp.path().to_path_buf();
        let big = vec![0u8; INLINE_THRESHOLD + 1];

        publish_clipboard(
            &bus_root,
            &data_home,
            "p",
            &["image/png".to_string()],
            "image/png",
            &big,
        )
        .unwrap();

        let blobs = blob_dir(&data_home);
        let orphan = blobs.join("01ORPHANBLOB00000000000000.png");
        std::fs::write(&orphan, b"orphan").unwrap();

        assert_eq!(gc_orphaned_blobs(&bus_root, &data_home).unwrap(), 1);
        assert!(!orphan.exists(), "orphan deleted");
        assert_eq!(
            std::fs::read_dir(&blobs).unwrap().count(),
            1,
            "live blob remains"
        );
    }

    #[test]
    fn blob_becomes_orphan_after_message_eviction() {
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path().join("bus");
        let data_home = tmp.path().to_path_buf();
        let big = vec![0u8; INLINE_THRESHOLD + 1];

        publish_clipboard(
            &bus_root,
            &data_home,
            "p",
            &["image/png".to_string()],
            "image/png",
            &big,
        )
        .unwrap();

        // Simulate retention-eviction: delete the bus message.
        let topic_dir = bus_root.join("clipboard/sync");
        for e in std::fs::read_dir(&topic_dir).unwrap() {
            std::fs::remove_file(e.unwrap().path()).unwrap();
        }

        // Now orphaned.
        assert_eq!(gc_orphaned_blobs(&bus_root, &data_home).unwrap(), 1);

        let blobs = blob_dir(&data_home);
        assert_eq!(
            std::fs::read_dir(&blobs).unwrap().count(),
            0,
            "no blobs after GC"
        );
    }

    #[test]
    fn pinned_blob_survives_when_pinned_message_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path().join("bus");
        let data_home = tmp.path().to_path_buf();
        let big = vec![0u8; INLINE_THRESHOLD + 1];

        // Publish a large blob so a BlobRef message is created.
        publish_clipboard(
            &bus_root,
            &data_home,
            "p",
            &["image/png".to_string()],
            "image/png",
            &big,
        )
        .unwrap();

        // Pin the message ULID by reading the topic dir for the file name.
        let topic_dir = bus_root.join("clipboard/sync");
        let msg_file = std::fs::read_dir(&topic_dir)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let ulid = msg_file
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let pin_path = pin_store_path(&data_home);
        let mut pins = PinStore::default();
        pins.pin(&ulid);
        pins.save_to(&pin_path).unwrap();

        // GC must not delete the blob because the message is pinned.
        assert_eq!(gc_orphaned_blobs(&bus_root, &data_home).unwrap(), 0);
        let blobs = blob_dir(&data_home);
        assert_eq!(
            std::fs::read_dir(&blobs).unwrap().count(),
            1,
            "pinned blob must survive"
        );
    }

    #[test]
    fn inline_message_does_not_create_false_reference() {
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path().join("bus");
        let data_home = tmp.path().to_path_buf();

        // Inline publish — no blob written.
        publish_clipboard(
            &bus_root,
            &data_home,
            "p",
            &["text/plain".to_string()],
            "text/plain",
            b"hello",
        )
        .unwrap();

        // Plant an orphan blob.
        let blobs = blob_dir(&data_home);
        std::fs::create_dir_all(&blobs).unwrap();
        let orphan = blobs.join("01ORPHANBLOB00000000000000.txt");
        std::fs::write(&orphan, b"orphan").unwrap();

        // Inline message must not shield the orphan blob from GC.
        assert_eq!(gc_orphaned_blobs(&bus_root, &data_home).unwrap(), 1);
        assert!(!orphan.exists());
    }
}
