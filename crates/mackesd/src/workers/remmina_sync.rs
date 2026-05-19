//! v2.0.0 Phase B.5 — remmina-sync worker.
//!
//! Drives the existing `mackes/remmina_sync.py` business logic (keeps
//! Remmina's connection list in sync with the mesh peer registry)
//! every 60 s under the unified supervisor. Replaces
//! `mackes-remmina-sync.service` + `mackes-remmina-sync.timer`.
//! Python module stays the source-of-truth through the v1.x line;
//! v2.0.0 cut reimplements its xml-writer surface in Rust under this
//! module.

#![cfg(feature = "async-services")]

use std::ffi::OsString;
use std::time::Duration;

use super::subprocess_tick::SubprocessTickWorker;

/// Cadence locked at 60 s per the legacy `mackes-remmina-sync.timer`.
pub const TICK_INTERVAL_S: u64 = 60;

/// Construct the supervisor-ready worker.
#[must_use]
pub fn build() -> SubprocessTickWorker {
    SubprocessTickWorker::new(
        "remmina-sync",
        "python3",
        vec![
            OsString::from("-m"),
            OsString::from("mackes.remmina_sync"),
        ],
        Duration::from_secs(TICK_INTERVAL_S),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workers::Worker;

    #[test]
    fn remmina_sync_worker_name_matches_phase_b_lock() {
        let w = build();
        assert_eq!(w.name(), "remmina-sync");
    }

    #[test]
    fn tick_interval_matches_legacy_timer() {
        assert_eq!(TICK_INTERVAL_S, 60);
    }
}
