//! BUS-7.1 — per-peer JSONL audit log.
//!
//! Every publish through [`crate::persist::Persist::write`] also
//! appends one line to today's audit file at
//! `<bus_root>/audit/<YYYY-MM-DD>.jsonl`. The line carries just
//! the metadata an operator + security audit needs to answer
//! "who / when / what topic / what priority / which ULID" —
//! never the message body. The body lives in the file tree
//! (BUS-1.4) where the audit log can re-fetch when needed.
//!
//! Per `docs/design/v6.x-mackes-bus.md` §7-audit lock:
//!
//! - **Plain JSONL** — no signing, no encryption. The mesh is
//!   flat-trust; the security boundary is GFS perms (0600 on
//!   the audit dir + the file itself) + Nebula transport.
//! - **One file per UTC date** — rotates at midnight UTC. Old
//!   files survive until topic retention (BUS-1.9) reaps them
//!   from the SQLite index; the audit JSONLs themselves stick
//!   around forever (cheap to keep).
//! - **Append-only** — never overwrites + never re-orders
//!   existing entries. If two writers race on the same file
//!   the kernel's O_APPEND guarantees per-write atomicity for
//!   small lines (< PIPE_BUF, 4096 on Linux).
//!
//! Storage layout:
//!
//! ```text
//! ~/.local/share/mde/bus/
//!   audit/
//!     2026-05-26.jsonl   ← today
//!     2026-05-25.jsonl   ← yesterday
//!     ...
//! ```

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One audit-log entry. Metadata only — never the body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEntry {
    /// Publisher identity. Defaults to the peer hostname for
    /// daemon-originated publishes; webhook handlers pass the
    /// adapter name (`github`, `gitea`, etc.); CLI publishes
    /// pass `cli:<hostname>` so audits distinguish operator
    /// commands from daemon traffic.
    pub publisher: String,
    /// ISO-8601 (UTC) timestamp of the publish.
    pub ts_iso: String,
    /// Topic the message landed on.
    pub topic: String,
    /// Priority — `min` / `default` / `high` / `urgent`.
    pub priority: String,
    /// ULID of the message in the file tree + index.
    pub ulid: String,
}

/// Errors writing the audit log.
#[derive(Debug, Error)]
pub enum AuditError {
    /// File-system error (mkdir, open, write).
    #[error("io: {0}")]
    Io(String),
    /// JSON serialization error (should never happen — the
    /// type is plain JSON-compatible).
    #[error("json: {0}")]
    Json(String),
}

/// Resolve the audit directory under `bus_root`. The caller
/// owns the `bus_root` so this is a pure path-join helper.
#[must_use]
pub fn audit_dir(bus_root: &Path) -> PathBuf {
    bus_root.join("audit")
}

/// Compute the per-day filename. `2026-05-26.jsonl` etc.
#[must_use]
pub fn filename_for(date: DateTime<Utc>) -> String {
    format!("{}.jsonl", date.format("%Y-%m-%d"))
}

/// Append one [`AuditEntry`] to today's audit file.
///
/// Opens with `O_APPEND` so small lines (< 4 KB) are atomically
/// appended even under concurrent writers. The audit dir is
/// created on first call if missing; file perms are set to
/// `0600` on creation so only the running peer's user can
/// read.
///
/// # Errors
/// [`AuditError::Io`] on mkdir/open/write; [`AuditError::Json`]
/// on serialization (should not happen).
pub fn append(bus_root: &Path, entry: &AuditEntry) -> Result<(), AuditError> {
    append_at(bus_root, entry, Utc::now())
}

/// Internal helper: append with an explicit "today" so the
/// rotation test can synthesize date crossings without
/// touching the wall clock.
pub fn append_at(
    bus_root: &Path,
    entry: &AuditEntry,
    today: DateTime<Utc>,
) -> Result<(), AuditError> {
    let dir = audit_dir(bus_root);
    std::fs::create_dir_all(&dir)
        .map_err(|e| AuditError::Io(format!("mkdir {}: {e}", dir.display())))?;
    set_dir_perms_0700(&dir)?;
    let path = dir.join(filename_for(today));

    let line = serde_json::to_string(entry)
        .map_err(|e| AuditError::Json(format!("encode: {e}")))?;
    let mut f = open_append_0600(&path)?;
    writeln!(f, "{line}")
        .map_err(|e| AuditError::Io(format!("write {}: {e}", path.display())))?;
    Ok(())
}

#[cfg(unix)]
fn set_dir_perms_0700(dir: &Path) -> Result<(), AuditError> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o700);
    std::fs::set_permissions(dir, perms)
        .map_err(|e| AuditError::Io(format!("chmod 0700 {}: {e}", dir.display())))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_dir_perms_0700(_dir: &Path) -> Result<(), AuditError> {
    Ok(())
}

#[cfg(unix)]
fn open_append_0600(path: &Path) -> Result<File, AuditError> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| AuditError::Io(format!("open {}: {e}", path.display())))
}

#[cfg(not(unix))]
fn open_append_0600(path: &Path) -> Result<File, AuditError> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| AuditError::Io(format!("open {}: {e}", path.display())))
}

/// List every JSONL filename in the audit dir, sorted oldest-first.
/// Used by the BUS-7.3 history view.
///
/// # Errors
/// [`AuditError::Io`] if the dir read fails. Missing dir
/// returns `Ok(vec![])` (not an error — pre-first-publish state).
pub fn list_files(bus_root: &Path) -> Result<Vec<PathBuf>, AuditError> {
    let dir = audit_dir(bus_root);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&dir)
        .map_err(|e| AuditError::Io(format!("readdir {}: {e}", dir.display())))?;
    for entry in entries {
        let entry = entry
            .map_err(|e| AuditError::Io(format!("readdir entry: {e}")))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// Read every audit entry under `<bus_root>/audit/*.jsonl` in
/// oldest-first order (sorted by filename which is `YYYY-MM-DD`
/// so date-ordered). Returns `Ok(vec![])` when the audit dir
/// doesn't exist yet. Per-line parse failures are skipped + logged
/// as warnings (one malformed line shouldn't blow away the whole
/// read; the rest of the file should still be readable).
///
/// # Errors
/// [`AuditError::Io`] on directory-read or file-open failure;
/// per-line JSON parse errors are NOT bubbled — they log + skip.
pub fn read_entries(bus_root: &Path) -> Result<Vec<AuditEntry>, AuditError> {
    let files = list_files(bus_root)?;
    let mut out = Vec::new();
    for path in files {
        let body = match std::fs::read_to_string(&path) {
            Ok(b) => b,
            Err(e) => {
                return Err(AuditError::Io(format!(
                    "read {}: {e}",
                    path.display()
                )));
            }
        };
        for (line_no, line) in body.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<AuditEntry>(line) {
                Ok(entry) => out.push(entry),
                Err(e) => {
                    tracing::warn!(
                        target: "mde_bus::audit",
                        path = %path.display(),
                        line = line_no + 1,
                        error = %e,
                        "skipping malformed audit line"
                    );
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn entry(publisher: &str, topic: &str, ulid: &str) -> AuditEntry {
        AuditEntry {
            publisher: publisher.to_string(),
            ts_iso: Utc::now().to_rfc3339(),
            topic: topic.to_string(),
            priority: "default".to_string(),
            ulid: ulid.to_string(),
        }
    }

    #[test]
    fn filename_format_is_yyyy_mm_dd_jsonl() {
        let d = Utc.with_ymd_and_hms(2026, 5, 26, 12, 0, 0).unwrap();
        assert_eq!(filename_for(d), "2026-05-26.jsonl");
    }

    #[test]
    fn audit_dir_resolves_under_bus_root() {
        let p = Path::new("/var/lib/mde/bus");
        assert_eq!(audit_dir(p), Path::new("/var/lib/mde/bus/audit"));
    }

    #[test]
    fn append_creates_file_on_first_write() {
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path();
        let today = Utc.with_ymd_and_hms(2026, 5, 26, 10, 0, 0).unwrap();
        append_at(bus_root, &entry("test", "t/x", "01A"), today).unwrap();
        let path = audit_dir(bus_root).join("2026-05-26.jsonl");
        assert!(path.exists());
    }

    #[test]
    fn append_is_append_only_and_jsonl_per_line() {
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path();
        let today = Utc.with_ymd_and_hms(2026, 5, 26, 10, 0, 0).unwrap();
        append_at(bus_root, &entry("a", "t/1", "u1"), today).unwrap();
        append_at(bus_root, &entry("b", "t/2", "u2"), today).unwrap();
        append_at(bus_root, &entry("c", "t/3", "u3"), today).unwrap();
        let path = audit_dir(bus_root).join("2026-05-26.jsonl");
        let body = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = body.lines().collect();
        assert_eq!(lines.len(), 3);
        // Each line round-trips through serde.
        for (i, l) in lines.iter().enumerate() {
            let parsed: AuditEntry = serde_json::from_str(l).unwrap();
            assert_eq!(parsed.ulid, format!("u{}", i + 1));
        }
    }

    #[test]
    fn append_rotates_on_date_change() {
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path();
        let day1 = Utc.with_ymd_and_hms(2026, 5, 26, 23, 59, 59).unwrap();
        let day2 = Utc.with_ymd_and_hms(2026, 5, 27, 0, 0, 1).unwrap();
        append_at(bus_root, &entry("a", "t/1", "u1"), day1).unwrap();
        append_at(bus_root, &entry("b", "t/2", "u2"), day2).unwrap();
        let p1 = audit_dir(bus_root).join("2026-05-26.jsonl");
        let p2 = audit_dir(bus_root).join("2026-05-27.jsonl");
        assert!(p1.exists());
        assert!(p2.exists());
        assert_eq!(std::fs::read_to_string(&p1).unwrap().lines().count(), 1);
        assert_eq!(std::fs::read_to_string(&p2).unwrap().lines().count(), 1);
    }

    #[test]
    fn list_files_returns_sorted_jsonls_only() {
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path();
        // Plant out-of-order writes
        let d1 = Utc.with_ymd_and_hms(2026, 5, 25, 10, 0, 0).unwrap();
        let d2 = Utc.with_ymd_and_hms(2026, 5, 26, 10, 0, 0).unwrap();
        let d3 = Utc.with_ymd_and_hms(2026, 5, 27, 10, 0, 0).unwrap();
        append_at(bus_root, &entry("a", "t", "u"), d3).unwrap();
        append_at(bus_root, &entry("a", "t", "u"), d1).unwrap();
        append_at(bus_root, &entry("a", "t", "u"), d2).unwrap();
        // Plant a non-JSONL file in the dir — must be ignored.
        std::fs::write(audit_dir(bus_root).join("note.txt"), "skip me").unwrap();
        let files = list_files(bus_root).unwrap();
        assert_eq!(files.len(), 3);
        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(
            names,
            vec![
                "2026-05-25.jsonl".to_string(),
                "2026-05-26.jsonl".to_string(),
                "2026-05-27.jsonl".to_string()
            ]
        );
    }

    #[test]
    fn list_files_returns_empty_when_dir_missing() {
        let tmp = tempfile::tempdir().unwrap();
        // Don't write anything; just call list_files.
        let files = list_files(tmp.path()).unwrap();
        assert!(files.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn file_lands_with_0600_perms() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path();
        let today = Utc.with_ymd_and_hms(2026, 5, 26, 10, 0, 0).unwrap();
        append_at(bus_root, &entry("a", "t", "u"), today).unwrap();
        let path = audit_dir(bus_root).join("2026-05-26.jsonl");
        let meta = std::fs::metadata(&path).unwrap();
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "expected 0600, got {mode:o}");
    }

    #[cfg(unix)]
    #[test]
    fn dir_lands_with_0700_perms() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path();
        let today = Utc.with_ymd_and_hms(2026, 5, 26, 10, 0, 0).unwrap();
        append_at(bus_root, &entry("a", "t", "u"), today).unwrap();
        let dir = audit_dir(bus_root);
        let meta = std::fs::metadata(&dir).unwrap();
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(mode, 0o700, "expected 0700, got {mode:o}");
    }

    #[test]
    fn read_entries_returns_empty_when_audit_dir_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path();
        let entries = read_entries(bus_root).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn read_entries_returns_logged_entries_in_order() {
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path();
        let day1 = Utc.with_ymd_and_hms(2026, 5, 25, 10, 0, 0).unwrap();
        let day2 = Utc.with_ymd_and_hms(2026, 5, 26, 10, 0, 0).unwrap();
        // Out-of-order writes — filename-sort should put day1 first.
        append_at(bus_root, &entry("a", "t1", "u1"), day2).unwrap();
        append_at(bus_root, &entry("a", "t1", "u2"), day2).unwrap();
        append_at(bus_root, &entry("a", "t0", "u0"), day1).unwrap();
        let entries = read_entries(bus_root).unwrap();
        assert_eq!(entries.len(), 3);
        // day1 entry first (sorted by filename), then 2x day2 entries
        // in append order.
        assert_eq!(entries[0].ulid, "u0");
        assert_eq!(entries[1].ulid, "u1");
        assert_eq!(entries[2].ulid, "u2");
    }

    #[test]
    fn read_entries_skips_malformed_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let bus_root = tmp.path();
        let today = Utc.with_ymd_and_hms(2026, 5, 26, 10, 0, 0).unwrap();
        // Write a valid entry, then a corrupt line, then another
        // valid entry. read_entries should skip the corrupt line +
        // return 2 valid entries.
        append_at(bus_root, &entry("a", "t", "u1"), today).unwrap();
        let dir = audit_dir(bus_root);
        let file_path = dir.join(filename_for(today));
        let mut existing = std::fs::read_to_string(&file_path).unwrap();
        existing.push_str("not-valid-json\n");
        std::fs::write(&file_path, existing).unwrap();
        append_at(bus_root, &entry("a", "t", "u2"), today).unwrap();
        let entries = read_entries(bus_root).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].ulid, "u1");
        assert_eq!(entries[1].ulid, "u2");
    }
}
