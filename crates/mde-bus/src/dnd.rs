//! BUS-2.8 — Do Not Disturb state machine + per-topic quiet hours.
//!
//! v6.x BUS-2.8 design lock: a single DND toggle gates ALL surfaces
//! (toast, tray, status-zone strip, theater takeover, wallpaper
//! stripe). When DND is active, only messages tagged with
//! `override=dnd` bypass — those messages still surface so genuine
//! emergencies (security incidents, critical alerts) can reach the
//! operator while everyday notifications stay quiet.
//!
//! Per-topic quiet hours layer on top of the DND toggle: each
//! topic config can carry a `quiet_after` / `quiet_until` window
//! of local-time seconds-of-day. Within that window, the topic
//! behaves as if DND was on (message goes to persistent file
//! storage + audit but is NOT routed to display surfaces).
//!
//! ## Files
//!
//! DND state syncs across the mesh via
//! `<XDG_DATA_HOME>/mde/bus/dnd.yaml` on the GFS-replicated
//! `mesh-home`. The schema is intentionally tiny so a flick of
//! the toggle on peer-A propagates to peer-B within the GFS
//! 1-second heal window.
//!
//! ## What ships here (BUS-2.8.data)
//!
//! This module is the v1 — DATA MODEL + DECISION LOGIC.
//! Serialization round-trip + the `is_suppressed` pure helper are
//! both unit-testable in isolation. The GFS sync + inotify watch
//! ship as a separate BUS-2.8.watcher follow-on once the data
//! schema is locked.

use serde::{Deserialize, Serialize};

/// Mesh-wide DND state. Single bool per the design lock —
/// per-topic mute is handled by the `subs.yaml` manifest (per
/// BUS-1.7), not by the DND toggle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DndState {
    /// `true` when DND is active; `false` when off.
    #[serde(default)]
    pub active: bool,
    /// Wall-clock instant the state was last toggled, in
    /// milliseconds since the Unix epoch. Used by the audit
    /// log to capture "DND on since 14:00 local."
    #[serde(default)]
    pub since_unix_ms: i64,
    /// Hostname of the peer that flipped the toggle. Used to
    /// surface "DND on by @<peer>" in the UI; mesh-wide sync
    /// means the source can differ from the local peer.
    #[serde(default)]
    pub set_by_peer: String,
}

impl Default for DndState {
    fn default() -> Self {
        Self {
            active: false,
            since_unix_ms: 0,
            set_by_peer: String::new(),
        }
    }
}

/// Per-topic quiet-hour window. Both fields are seconds-since-
/// midnight in the operator's local timezone (0..86_399).
/// `quiet_after` = window opens at this time; `quiet_until` =
/// window closes. When `quiet_after < quiet_until` the window is
/// same-day (09:00..17:00 = work-quiet); when `quiet_after >
/// quiet_until` the window wraps midnight (22:00..07:00 =
/// overnight-quiet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TopicQuietHours {
    /// Window-open boundary in seconds-of-day (0..86_400). When
    /// both fields are `None`, no quiet window is active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiet_after: Option<u32>,
    /// Window-close boundary in seconds-of-day (0..86_400).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiet_until: Option<u32>,
}

/// Pure-fn — true if the given seconds-of-day falls inside the
/// quiet-hour window. Returns `false` when either bound is
/// `None` (no window configured) or when both bounds are equal
/// (zero-length window). Handles both same-day and overnight
/// (wrap-midnight) windows.
#[must_use]
pub fn is_quiet_hour(now_local_seconds: u32, hours: TopicQuietHours) -> bool {
    let (Some(after), Some(until)) = (hours.quiet_after, hours.quiet_until) else {
        return false;
    };
    if after == until {
        // Zero-length window — never quiet.
        return false;
    }
    if after < until {
        // Same-day window (09:00..17:00 = work-quiet).
        now_local_seconds >= after && now_local_seconds < until
    } else {
        // Overnight window (22:00..07:00). Quiet iff now is
        // after `quiet_after` OR before `quiet_until`.
        now_local_seconds >= after || now_local_seconds < until
    }
}

/// Pure-fn — true if the message should be SUPPRESSED (not
/// routed to display surfaces). The message still gets persisted
/// + audited regardless; suppression is a routing decision, not
/// a storage decision.
///
/// Rules (in priority order):
///   1. `override=dnd` tag → never suppressed (genuine
///      emergency bypass).
///   2. Global DND toggle active → suppressed.
///   3. Topic quiet-hour window active → suppressed.
///   4. Otherwise → not suppressed.
#[must_use]
pub fn is_suppressed(
    state: &DndState,
    topic_hours: TopicQuietHours,
    tags: &[&str],
    now_local_seconds: u32,
) -> bool {
    if tags.contains(&"override=dnd") {
        return false;
    }
    if state.active {
        return true;
    }
    is_quiet_hour(now_local_seconds, topic_hours)
}

/// Load the mesh-wide DND state from the GFS-replicated YAML
/// file at `<bus_root>/dnd.yaml`. Returns `DndState::default()`
/// (DND off) when the file is missing or unparseable — DND off
/// is the safe default so a corrupted file doesn't silently
/// suppress every notification.
#[must_use]
pub fn load_default(bus_root: &std::path::Path) -> DndState {
    let path = bus_root.join("dnd.yaml");
    let Ok(bytes) = std::fs::read(&path) else {
        return DndState::default();
    };
    serde_yaml::from_slice(&bytes).unwrap_or_default()
}

/// Atomic-write the DND state to `<bus_root>/dnd.yaml` via
/// temp-file + rename. Caller passes the full state (typically
/// from the operator's DND-toggle Workbench surface or a
/// `mde-bus dnd on/off` CLI verb that ships separately).
/// Returns `Ok(())` on success; `Err(io::Error)` on filesystem
/// failure.
pub fn save_default(bus_root: &std::path::Path, state: &DndState) -> std::io::Result<()> {
    std::fs::create_dir_all(bus_root)?;
    let serialized = serde_yaml::to_string(state)
        .map_err(|e| std::io::Error::other(format!("serialize dnd.yaml: {e}")))?;
    let final_path = bus_root.join("dnd.yaml");
    let tmp_path = bus_root.join("dnd.yaml.tmp");
    std::fs::write(&tmp_path, serialized)?;
    std::fs::rename(&tmp_path, &final_path)?;
    Ok(())
}

/// Convenience: parse an `HH:MM` (24-hour) string into a
/// seconds-of-day value. Returns `None` on malformed input
/// (missing colon, non-numeric, out-of-range hour or minute).
/// Used by the `dnd.yaml` migration path that accepts both raw
/// seconds + human-readable HH:MM strings.
#[must_use]
pub fn parse_hhmm(s: &str) -> Option<u32> {
    let (h_str, m_str) = s.split_once(':')?;
    let h: u32 = h_str.parse().ok()?;
    let m: u32 = m_str.parse().ok()?;
    if h >= 24 || m >= 60 {
        return None;
    }
    Some(h * 3600 + m * 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_dnd_off() {
        let s = DndState::default();
        assert!(!s.active);
        assert_eq!(s.since_unix_ms, 0);
        assert!(s.set_by_peer.is_empty());
    }

    #[test]
    fn dnd_state_roundtrips_yaml() {
        let s = DndState {
            active: true,
            since_unix_ms: 1_700_000_000_000,
            set_by_peer: "fedora".to_string(),
        };
        let yaml = serde_yaml::to_string(&s).unwrap();
        let back: DndState = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn topic_quiet_hours_default_no_window() {
        let h = TopicQuietHours::default();
        assert!(h.quiet_after.is_none());
        assert!(h.quiet_until.is_none());
        assert!(!is_quiet_hour(12 * 3600, h));
    }

    #[test]
    fn quiet_hour_same_day_window() {
        // 09:00..17:00 work-quiet.
        let h = TopicQuietHours {
            quiet_after: Some(9 * 3600),
            quiet_until: Some(17 * 3600),
        };
        assert!(!is_quiet_hour(8 * 3600, h));    // 08:00 — before window
        assert!(is_quiet_hour(9 * 3600, h));     // 09:00 — boundary in
        assert!(is_quiet_hour(12 * 3600, h));    // 12:00 — middle
        assert!(!is_quiet_hour(17 * 3600, h));   // 17:00 — boundary out
        assert!(!is_quiet_hour(20 * 3600, h));   // 20:00 — after window
    }

    #[test]
    fn quiet_hour_overnight_window() {
        // 22:00..07:00 overnight-quiet.
        let h = TopicQuietHours {
            quiet_after: Some(22 * 3600),
            quiet_until: Some(7 * 3600),
        };
        assert!(is_quiet_hour(23 * 3600, h));    // 23:00 — after `after`
        assert!(is_quiet_hour(0, h));            // 00:00 — wrap midnight
        assert!(is_quiet_hour(6 * 3600, h));     // 06:00 — before `until`
        assert!(!is_quiet_hour(7 * 3600, h));    // 07:00 — boundary out
        assert!(!is_quiet_hour(12 * 3600, h));   // 12:00 — daytime
        assert!(!is_quiet_hour(21 * 3600 + 59 * 60, h)); // 21:59 — just before `after`
    }

    #[test]
    fn quiet_hour_zero_length_window_never_fires() {
        let h = TopicQuietHours {
            quiet_after: Some(12 * 3600),
            quiet_until: Some(12 * 3600),
        };
        for hour in 0..24 {
            assert!(!is_quiet_hour(hour * 3600, h));
        }
    }

    #[test]
    fn quiet_hour_one_sided_window_never_fires() {
        // Either bound None → no window.
        let only_after = TopicQuietHours {
            quiet_after: Some(9 * 3600),
            quiet_until: None,
        };
        assert!(!is_quiet_hour(12 * 3600, only_after));
        let only_until = TopicQuietHours {
            quiet_after: None,
            quiet_until: Some(17 * 3600),
        };
        assert!(!is_quiet_hour(12 * 3600, only_until));
    }

    #[test]
    fn override_dnd_tag_bypasses_global_toggle() {
        let state = DndState {
            active: true,
            since_unix_ms: 1_000,
            set_by_peer: "fedora".to_string(),
        };
        let hours = TopicQuietHours::default();
        let tags_with_override = ["priority=urgent", "override=dnd"];
        let tags_without = ["priority=urgent"];
        assert!(!is_suppressed(&state, hours, &tags_with_override, 12 * 3600));
        assert!(is_suppressed(&state, hours, &tags_without, 12 * 3600));
    }

    #[test]
    fn override_dnd_tag_bypasses_quiet_hours() {
        let state = DndState::default();
        let hours = TopicQuietHours {
            quiet_after: Some(9 * 3600),
            quiet_until: Some(17 * 3600),
        };
        let tags_with_override = ["override=dnd"];
        let tags_without: [&str; 0] = [];
        // Inside quiet hour, override bypasses; without override
        // the quiet window suppresses.
        assert!(!is_suppressed(&state, hours, &tags_with_override, 12 * 3600));
        assert!(is_suppressed(&state, hours, &tags_without, 12 * 3600));
    }

    #[test]
    fn dnd_off_outside_quiet_hours_is_not_suppressed() {
        let state = DndState::default();
        let hours = TopicQuietHours {
            quiet_after: Some(9 * 3600),
            quiet_until: Some(17 * 3600),
        };
        // 20:00 — DND off AND outside the quiet window → delivered.
        assert!(!is_suppressed(&state, hours, &[], 20 * 3600));
    }

    #[test]
    fn parse_hhmm_round_trip() {
        assert_eq!(parse_hhmm("09:00"), Some(9 * 3600));
        assert_eq!(parse_hhmm("17:00"), Some(17 * 3600));
        assert_eq!(parse_hhmm("00:00"), Some(0));
        assert_eq!(parse_hhmm("23:59"), Some(23 * 3600 + 59 * 60));
        assert_eq!(parse_hhmm("12:30"), Some(12 * 3600 + 30 * 60));
    }

    #[test]
    fn load_default_missing_file_returns_default() {
        let tmp = std::env::temp_dir().join(format!("mde-bus-dnd-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        // No dnd.yaml in tmp — should return default (DND off).
        let s = load_default(&tmp);
        assert_eq!(s, DndState::default());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn save_default_round_trip() {
        let tmp = std::env::temp_dir().join(format!("mde-bus-dnd-roundtrip-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let original = DndState {
            active: true,
            since_unix_ms: 1_700_000_000_000,
            set_by_peer: "fedora".to_string(),
        };
        save_default(&tmp, &original).unwrap();
        let loaded = load_default(&tmp);
        assert_eq!(original, loaded);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_default_corrupted_yaml_returns_default() {
        let tmp = std::env::temp_dir().join(format!("mde-bus-dnd-corrupt-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("dnd.yaml"), "this is not yaml: {[}{").unwrap();
        let s = load_default(&tmp);
        // DND off is the safe default — a corrupted file must NOT
        // silently suppress every notification.
        assert!(!s.active);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn parse_hhmm_rejects_malformed() {
        assert!(parse_hhmm("").is_none());
        assert!(parse_hhmm("9").is_none());
        assert!(parse_hhmm("09").is_none());
        assert!(parse_hhmm("09:").is_none());
        assert!(parse_hhmm(":00").is_none());
        assert!(parse_hhmm("24:00").is_none()); // hour out of range
        assert!(parse_hhmm("09:60").is_none()); // minute out of range
        assert!(parse_hhmm("ab:cd").is_none()); // non-numeric
    }
}
