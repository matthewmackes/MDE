//! v2.0.0 Phase B.4 — media-sync worker.
//!
//! Drives the existing `mackes/media_sync_daemon.py` business logic
//! (discovers mesh media servers + writes Sublime Music / Delfin /
//! Thunar configs) every 60 s under the unified supervisor. Replaces
//! `mackes-media-sync.service` + `mackes-media-sync.timer`. The
//! Python module stays the source-of-truth implementation through
//! the v1.x line; the v2.0.0 cut reimplements its discovery + JSON
//! writer surface in Rust under this module.

#![cfg(feature = "async-services")]

use std::ffi::OsString;
use std::time::Duration;

use super::subprocess_tick::SubprocessTickWorker;

/// Cadence locked at 60 s per the legacy `mackes-media-sync.timer`
/// `OnUnitActiveSec=60s` setting.
pub const TICK_INTERVAL_S: u64 = 60;

/// Construct the supervisor-ready worker. The cadence matches the
/// retired systemd timer so behavior is byte-for-byte identical
/// during the v1.x → v2.0.0 transition.
#[must_use]
pub fn build() -> SubprocessTickWorker {
    SubprocessTickWorker::new(
        "media-sync",
        "python3",
        vec![
            OsString::from("-m"),
            OsString::from("mackes.media_sync_daemon"),
        ],
        Duration::from_secs(TICK_INTERVAL_S),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workers::Worker;

    #[test]
    fn media_sync_worker_name_matches_phase_b_lock() {
        let w = build();
        assert_eq!(w.name(), "media-sync");
    }

    #[test]
    fn tick_interval_matches_legacy_timer() {
        assert_eq!(TICK_INTERVAL_S, 60);
    }
}
