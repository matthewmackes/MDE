//! On-disk sealing for the Nebula CA key + cert (NF-2.4).
//!
//! Per the v2.5 Nebula lock (Q3), the CA private key lives at
//! `/var/lib/mackesd/nebula-ca/ca.key` with mode 0600, owned by the
//! daemon's uid. `seal` writes a file atomically (tempfile in the
//! same directory, fsync, rename), then chmods to the requested
//! mode. `unseal` reads the file back and refuses to surface its
//! bytes unless the on-disk permission bits + owner uid match the
//! expected invariants.
//!
//! No path-traversal logic lives here — callers pass an absolute
//! path (typically derived from the `MACKESD_NEBULA_CA_DIR` env
//! injection used by the tempdir test path).

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use super::error::{CaError, CaResult};

/// Atomically write `bytes` to `path` with the requested unix
/// permission `mode` (e.g. `0o600` for the CA key, `0o644` for the
/// public cert).
///
/// Implementation: write to `<path>.tmp.<pid>` in the same directory,
/// flush + fsync, set permissions on the temp file, then rename over
/// the target. The rename is the atomic step — readers either see
/// the previous contents (if any) or the full new payload, never a
/// half-written file.
///
/// # Errors
///
/// Returns [`CaError::Io`] on any underlying filesystem error
/// (missing parent dir, EACCES, ENOSPC, etc).
pub fn seal(path: &Path, bytes: &[u8], mode: u32) -> CaResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| CaError::InvalidPath(path.to_path_buf()))?;
    fs::create_dir_all(parent).map_err(CaError::Io)?;

    let tmp = parent.join(format!(
        ".{}.tmp.{}",
        path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("seal"),
        std::process::id()
    ));

    // OpenOptions::mode sets the permission bits at create time so
    // the file never exists with the default umask.
    {
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(mode)
            .open(&tmp)
            .map_err(CaError::Io)?;
        f.write_all(bytes).map_err(CaError::Io)?;
        f.flush().map_err(CaError::Io)?;
        f.sync_all().map_err(CaError::Io)?;
    }

    // Belt-and-braces: explicitly chmod in case `OpenOptions::mode`
    // got ANDed with a restrictive umask (some libc impls).
    let perms = std::fs::Permissions::from_mode(mode);
    fs::set_permissions(&tmp, perms).map_err(CaError::Io)?;

    fs::rename(&tmp, path).map_err(CaError::Io)?;

    // fsync the parent directory so the rename hits stable storage
    // before we return success. Errors from this step are
    // best-effort — some filesystems (tmpfs) don't support it.
    if let Ok(dir) = File::open(parent) {
        let _ = dir.sync_all();
    }
    Ok(())
}

/// Read `path` back. Errors if the file isn't mode `0600` or isn't
/// owned by the current uid — the key invariants the seal step
/// established.
///
/// # Errors
///
/// - [`CaError::InsecurePermissions`] if the mode bits aren't `0600`.
/// - [`CaError::OwnerMismatch`] if the file owner uid differs from
///   the process's effective uid.
/// - [`CaError::Io`] for any other filesystem error.
pub fn unseal(path: &Path) -> CaResult<Vec<u8>> {
    let meta = fs::metadata(path).map_err(CaError::Io)?;

    // Strip the file-type bits; only the permission triplet matters.
    let mode = meta.permissions().mode() & 0o7777;
    if mode != 0o600 {
        return Err(CaError::InsecurePermissions {
            path: path.to_path_buf(),
            actual_mode: mode,
            expected_mode: 0o600,
        });
    }

    let file_uid = meta.uid();
    let proc_uid = unix_uid();
    if file_uid != proc_uid {
        return Err(CaError::OwnerMismatch {
            path: path.to_path_buf(),
            file_uid,
            proc_uid,
        });
    }

    fs::read(path).map_err(CaError::Io)
}

/// Resolve the active Nebula CA directory.
///
/// Reads `MACKESD_NEBULA_CA_DIR` first (test injection). Falls back
/// to the production path `/var/lib/mackesd/nebula-ca/`.
#[must_use]
pub fn ca_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("MACKESD_NEBULA_CA_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    PathBuf::from("/var/lib/mackesd/nebula-ca")
}

/// Process-effective uid — wrapped so tests can read it without
/// reaching for libc directly. Reads `/proc/self/status` because
/// the workspace forbids `unsafe_code` so libc::geteuid is off
/// the table.
///
/// The `Uid:` line in `/proc/self/status` is
/// `Uid:\t<real>\t<effective>\t<saved-set>\t<filesystem>`; we
/// want the effective uid (second field) since that's what file
/// permission checks resolve against.
#[must_use]
fn unix_uid() -> u32 {
    if let Ok(s) = fs::read_to_string("/proc/self/status") {
        for line in s.lines() {
            if let Some(rest) = line.strip_prefix("Uid:") {
                let mut toks = rest.split_whitespace();
                // Skip the real uid; the second token is effective.
                let _ = toks.next();
                if let Some(tok) = toks.next() {
                    if let Ok(n) = tok.parse::<u32>() {
                        return n;
                    }
                }
            }
        }
    }
    // Last-resort sentinel — the unseal check still works because
    // an attacker-owned file with mismatched uid still trips
    // `OwnerMismatch`, just against this sentinel.
    u32::MAX
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    #[test]
    fn seal_writes_with_requested_mode() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("ca.key");
        seal(&path, b"top-secret", 0o600).expect("seal");
        let mode = fs::metadata(&path).expect("stat").permissions().mode() & 0o7777;
        assert_eq!(mode, 0o600, "seal must apply the requested mode bits");
    }

    #[test]
    fn unseal_rejects_world_readable_file() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("ca.key");
        seal(&path, b"top-secret", 0o644).expect("seal");
        match unseal(&path) {
            Err(CaError::InsecurePermissions { actual_mode, .. }) => {
                assert_eq!(actual_mode, 0o644);
            }
            other => panic!("expected InsecurePermissions, got {other:?}"),
        }
    }

    #[test]
    fn unseal_round_trips_when_mode_is_0600() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("ca.key");
        seal(&path, b"top-secret-bytes", 0o600).expect("seal");
        let bytes = unseal(&path).expect("unseal");
        assert_eq!(bytes, b"top-secret-bytes");
    }

    #[test]
    fn unseal_errors_on_missing_path() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("nonexistent.key");
        match unseal(&path) {
            Err(CaError::Io(_)) => {}
            other => panic!("expected Io error, got {other:?}"),
        }
    }
}
