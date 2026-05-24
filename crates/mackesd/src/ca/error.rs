//! Error type for the `ca/` module (NF-2.x).
//!
//! Uses a hand-rolled `Display` + `std::error::Error` impl rather
//! than pulling in `thiserror` so the crate's existing
//! dependency surface stays unchanged. The variants closely
//! mirror the failure modes named in the NF-2.x design lock.

use std::fmt;
use std::path::PathBuf;

/// Result alias for the `ca/` module.
pub type CaResult<T> = std::result::Result<T, CaError>;

/// Closed enum of every failure mode the CA subsystem surfaces.
#[derive(Debug)]
pub enum CaError {
    /// Filesystem / I/O failure (read, write, rename, chmod).
    Io(std::io::Error),

    /// A path the caller passed has no parent component (e.g. a
    /// bare filename with no directory), so the seal step can't
    /// create the tempfile in the same directory.
    InvalidPath(PathBuf),

    /// On-disk permissions don't match the expected `0600`
    /// invariant. Raised by `unseal` only — `seal` always writes
    /// at the requested mode.
    InsecurePermissions {
        /// The path that tripped the check.
        path: PathBuf,
        /// The actual on-disk permission bits (`stat().st_mode & 0o7777`).
        actual_mode: u32,
        /// The required permission bits — `0o600` for the CA key.
        expected_mode: u32,
    },

    /// File owner uid doesn't match the running process's uid.
    /// Prevents a less-privileged process from being tricked into
    /// reading a CA key dropped in by another user.
    OwnerMismatch {
        /// The path that tripped the check.
        path: PathBuf,
        /// `stat().st_uid` of the file on disk.
        file_uid: u32,
        /// The process's effective uid.
        proc_uid: u32,
    },

    /// `nebula-cert` binary not found at the expected path. The
    /// daemon depends on the Fedora `nebula` package being
    /// installed; this surfaces the missing-dep state cleanly.
    NebulaCertMissing(PathBuf),

    /// `nebula-cert` exited non-zero. Carries the captured stderr
    /// for operator diagnosis.
    NebulaCertFailed {
        /// `nebula-cert` exit status as a printable integer
        /// (255 if it was killed by a signal).
        exit_status: i32,
        /// Whatever `nebula-cert` wrote to stderr.
        stderr: String,
    },

    /// `nebula-cert` ran but didn't produce one of the expected
    /// output files (`ca.crt` / `ca.key` / `<node>.crt` / etc).
    NebulaCertOutputMissing(PathBuf),

    /// The mesh CIDR is too small to allocate another peer.
    /// Surfaces the Q-MX18 16-peer cap when an operator over-grows
    /// the fleet against a tight CIDR.
    NoOverlayAddressAvailable {
        /// The CIDR that was exhausted — e.g. "10.42.0.0/16".
        cidr: String,
    },

    /// Ed25519 signature on the EnrollmentRequest failed to
    /// verify. The peer's claim of identity is rejected.
    InvalidSignature,

    /// SQL failure surfaced from rusqlite. Wraps the underlying
    /// error message so callers don't need a rusqlite dep.
    Sql(String),

    /// A required SQL row was missing (e.g. asked to bump epoch on
    /// a mesh that has never been minted).
    MeshNotFound {
        /// The mesh id that wasn't in the `nebula_ca` table.
        mesh_id: String,
    },
}

impl fmt::Display for CaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "ca: io error: {e}"),
            Self::InvalidPath(p) => write!(f, "ca: invalid path (no parent): {}", p.display()),
            Self::InsecurePermissions {
                path,
                actual_mode,
                expected_mode,
            } => write!(
                f,
                "ca: insecure permissions on {}: actual=0o{:o}, expected=0o{:o}",
                path.display(),
                actual_mode,
                expected_mode
            ),
            Self::OwnerMismatch {
                path,
                file_uid,
                proc_uid,
            } => write!(
                f,
                "ca: owner mismatch on {}: file_uid={file_uid}, proc_uid={proc_uid}",
                path.display()
            ),
            Self::NebulaCertMissing(p) => {
                write!(f, "ca: nebula-cert binary not found at {}", p.display())
            }
            Self::NebulaCertFailed {
                exit_status,
                stderr,
            } => write!(
                f,
                "ca: nebula-cert failed (exit={exit_status}): {}",
                stderr.trim()
            ),
            Self::NebulaCertOutputMissing(p) => {
                write!(f, "ca: nebula-cert did not produce {}", p.display())
            }
            Self::NoOverlayAddressAvailable { cidr } => {
                write!(f, "ca: no overlay address available in {cidr}")
            }
            Self::InvalidSignature => {
                write!(f, "ca: enrollment request signature failed to verify")
            }
            Self::Sql(msg) => write!(f, "ca: sql: {msg}"),
            Self::MeshNotFound { mesh_id } => {
                write!(f, "ca: mesh {mesh_id} has no CA row (mint first)")
            }
        }
    }
}

impl std::error::Error for CaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CaError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<rusqlite::Error> for CaError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e.to_string())
    }
}
