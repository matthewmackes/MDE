//! Notifications center applet — overlay reader for
//! the full notification log.
//!
//! Phase E1.2.6: companion to the notification-bell pill
//! (E1.2.5). The bell shows the unread count; this
//! applet renders the full list grouped by peer with
//! per-row mark-read / dismiss actions.
//!
//! Source-of-truth file is the same QNM-Shared-replicated
//! cache the bell reads: `~/.cache/mackes/notifications.
//! json`. Writes back through `mackes.notifications`
//! (Python library) via a `mackes notifications mark <id>
//! [read|dismissed]` shell call — keeps the write path
//! single-source.

#![forbid(unsafe_code)]

use std::path::PathBuf;

use mde_applet_api::{AppletId, AppletSlot, HostMessage};
use serde::Deserialize;

#[must_use]
pub fn manifest() -> mde_applet_api::AppletManifest {
    mde_applet_api::AppletManifest {
        id: AppletId::from_static("notifications"),
        binary: "mde-applet-notifications".into(),
        slot: AppletSlot::Overlay,
        summary: "Notifications center modal reader".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

/// One notification row — extends the bell-applet's
/// minimal shape with the fields the center renders.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct NotificationRow {
    /// Stable id (used for mark-read / dismiss writes).
    #[serde(default)]
    pub id: String,
    /// Originating peer name; empty for local-only rows.
    #[serde(default)]
    pub peer: String,
    /// Short title.
    #[serde(default)]
    pub title: String,
    /// Long body — may be empty.
    #[serde(default)]
    pub body: String,
    /// Unix-epoch-seconds timestamp.
    #[serde(default)]
    pub created_at: i64,
    /// Read state — `false` (unread) by default.
    #[serde(default)]
    pub read: bool,
    /// Dismissed state — implies read.
    #[serde(default)]
    pub dismissed: bool,
    /// KDC2-5.11 — origin token. When `"phone"`, the renderer
    /// prefixes the row with the phone glyph badge. Stays
    /// blank for local notifications. Wire-compatible with the
    /// Phase 13.4 drawer's marker so old `notifications.json`
    /// snapshots round-trip cleanly.
    #[serde(default)]
    pub origin: String,
    /// BUG-8.c (v4.0.1, 2026-05-23) — originating app id.
    /// Populated from the DBus source's appname when the
    /// notification daemon writes the row. Allows the
    /// notification center to collapse rows by app. Defaults
    /// to empty so old snapshots round-trip cleanly; empty
    /// rows render under an "Other" bucket.
    #[serde(default)]
    pub app_id: String,
}

/// KDC2-5.11 — phone glyph the center prepends to rows whose
/// `origin == "phone"`. Constant so the Iced renderer + the
/// format_center helper agree on the glyph.
pub const PHONE_ORIGIN_GLYPH: &str = "📱";

/// True when the row originated from a paired phone (mirror via
/// the `dev.mackes.MDE.Connect` D-Bus signal flow). Used by the
/// renderer to prepend the glyph + by tests to assert the
/// badging logic.
#[must_use]
pub fn is_phone_origin(row: &NotificationRow) -> bool {
    row.origin == "phone"
}

#[must_use]
pub fn notifications_cache_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".cache/mackes/notifications.json")
}

#[must_use]
pub fn parse_notifications(raw: &str) -> Vec<NotificationRow> {
    serde_json::from_str(raw).unwrap_or_default()
}

/// Filter dismissed rows out — the center hides them by
/// default. A future "Show dismissed" toggle inverts this.
#[must_use]
pub fn visible(rows: Vec<NotificationRow>) -> Vec<NotificationRow> {
    rows.into_iter().filter(|r| !r.dismissed).collect()
}

/// Group + sort: by peer (alphabetical, empty/local first),
/// then within peer by `created_at` DESC.
#[must_use]
pub fn group_and_sort(rows: Vec<NotificationRow>) -> Vec<(String, Vec<NotificationRow>)> {
    use std::collections::BTreeMap;
    let mut grouped: BTreeMap<String, Vec<NotificationRow>> = BTreeMap::new();
    for r in rows {
        grouped.entry(r.peer.clone()).or_default().push(r);
    }
    for (_, list) in grouped.iter_mut() {
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    }
    grouped.into_iter().collect()
}

/// BUG-8.c — group + sort by `app_id`. Rows with empty
/// `app_id` cluster under `"Other"` so the renderer can always
/// emit a header. Within each app bucket, sort `created_at`
/// DESC (newest first) to match the peer-bucket convention.
/// The first-element tuple is the bucket key (the display
/// label for the section header); the second element is the
/// rows in display order.
#[must_use]
pub fn group_by_app(rows: Vec<NotificationRow>) -> Vec<(String, Vec<NotificationRow>)> {
    use std::collections::BTreeMap;
    let mut grouped: BTreeMap<String, Vec<NotificationRow>> = BTreeMap::new();
    for r in rows {
        let key = if r.app_id.is_empty() {
            "Other".to_string()
        } else {
            r.app_id.clone()
        };
        grouped.entry(key).or_default().push(r);
    }
    for (_, list) in grouped.iter_mut() {
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    }
    grouped.into_iter().collect()
}

/// Render the center as one section per peer, one line per
/// row: `<read-mark> <title> · <body-truncated>`.
#[must_use]
pub fn format_center(groups: &[(String, Vec<NotificationRow>)]) -> String {
    if groups.is_empty() {
        return "(no notifications)".to_string();
    }
    let mut out = String::new();
    for (peer, rows) in groups {
        let header = if peer.is_empty() {
            "Local".to_string()
        } else {
            peer.clone()
        };
        out.push_str(&format!("== {header} ==\n"));
        for r in rows {
            let mark = if r.read { " " } else { "•" };
            let body_preview: String = r.body.chars().take(60).collect();
            // KDC2-5.11 — phone-origin rows wear the badge.
            let badge = if is_phone_origin(r) {
                format!("{PHONE_ORIGIN_GLYPH} ")
            } else {
                String::new()
            };
            out.push_str(&format!("{mark} {badge}{} · {body_preview}\n", r.title));
        }
    }
    out.trim_end_matches('\n').to_string()
}

#[must_use]
pub fn handle_host(msg: &HostMessage) -> bool {
    !matches!(msg, HostMessage::Shutdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_row(id: &str, peer: &str, title: &str, ts: i64, read: bool) -> NotificationRow {
        NotificationRow {
            id: id.into(),
            peer: peer.into(),
            title: title.into(),
            body: format!("body of {title}"),
            created_at: ts,
            read,
            dismissed: false,
            origin: String::new(),
            app_id: String::new(),
        }
    }

    #[test]
    fn manifest_lands_in_overlay_slot() {
        let m = manifest();
        assert_eq!(m.id.as_str(), "notifications");
        assert_eq!(m.slot, AppletSlot::Overlay);
    }

    #[test]
    fn parse_notifications_empty_on_garbage() {
        assert!(parse_notifications("").is_empty());
        assert!(parse_notifications("not json").is_empty());
    }

    #[test]
    fn parse_notifications_extracts_full_shape() {
        let raw = r#"[
            {"id": "n1", "peer": "alpha", "title": "T", "body": "B",
             "created_at": 1234, "read": false, "dismissed": false}
        ]"#;
        let rows = parse_notifications(raw);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "n1");
        assert_eq!(rows[0].peer, "alpha");
        assert_eq!(rows[0].created_at, 1234);
        assert!(!rows[0].read);
    }

    #[test]
    fn visible_filters_dismissed() {
        let rows = vec![
            NotificationRow {
                id: "a".into(),
                dismissed: false,
                ..Default::default()
            },
            NotificationRow {
                id: "b".into(),
                dismissed: true,
                ..Default::default()
            },
        ];
        let v = visible(rows);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].id, "a");
    }

    #[test]
    fn group_and_sort_buckets_by_peer_and_sorts_desc() {
        let rows = vec![
            make_row("a1", "alpha", "first", 100, false),
            make_row("b1", "beta", "second", 200, false),
            make_row("a2", "alpha", "third", 300, false),
            make_row("l1", "", "local", 150, false),
        ];
        let grouped = group_and_sort(rows);
        assert_eq!(grouped.len(), 3);
        // BTreeMap keys are alphabetical; "" sorts before
        // "alpha" before "beta".
        assert_eq!(grouped[0].0, "");
        assert_eq!(grouped[1].0, "alpha");
        assert_eq!(grouped[2].0, "beta");
        // alpha has 2 rows; within peer, newer (created_at
        // 300) comes first.
        assert_eq!(grouped[1].1.len(), 2);
        assert_eq!(grouped[1].1[0].id, "a2");
    }

    #[test]
    fn format_center_empty_message() {
        assert_eq!(format_center(&[]), "(no notifications)");
    }

    #[test]
    fn format_center_marks_unread_with_bullet() {
        let groups = vec![(
            "alpha".to_string(),
            vec![make_row("a1", "alpha", "Hello", 100, false)],
        )];
        let s = format_center(&groups);
        assert!(s.contains("== alpha =="));
        assert!(s.contains("•"));
        assert!(s.contains("Hello"));
    }

    #[test]
    fn format_center_empty_peer_renders_as_local() {
        let groups = vec![(
            "".to_string(),
            vec![make_row("l1", "", "Local note", 100, true)],
        )];
        let s = format_center(&groups);
        assert!(s.contains("== Local =="));
        // Read row uses " " marker, not bullet.
        assert!(!s.lines().any(|l| l.starts_with("•")));
    }

    #[test]
    fn handle_host_short_circuits_shutdown() {
        assert!(!handle_host(&HostMessage::Shutdown));
    }

    // ─────────────────────────────────────────────────────────
    // KDC2-5.11 — phone-origin badge
    // ─────────────────────────────────────────────────────────

    #[test]
    fn is_phone_origin_matches_only_on_phone_token() {
        let mut r = NotificationRow::default();
        assert!(!is_phone_origin(&r), "default origin is local");
        r.origin = "phone".into();
        assert!(is_phone_origin(&r));
        r.origin = "tablet".into();
        assert!(!is_phone_origin(&r), "tablet is not phone");
    }

    #[test]
    fn format_center_prepends_phone_glyph_for_phone_origin_rows() {
        let mut row = make_row("p1", "alpha", "Ring", 100, false);
        row.origin = "phone".into();
        let groups = vec![("alpha".to_string(), vec![row])];
        let s = format_center(&groups);
        assert!(s.contains(PHONE_ORIGIN_GLYPH), "phone badge missing: {s}");
        assert!(s.contains("Ring"));
    }

    #[test]
    fn format_center_omits_glyph_for_local_rows() {
        let groups = vec![(
            "".to_string(),
            vec![make_row("l1", "", "Local", 100, false)],
        )];
        let s = format_center(&groups);
        assert!(
            !s.contains(PHONE_ORIGIN_GLYPH),
            "local row must not wear phone glyph",
        );
    }

    // ─────────────────────────────────────────────────────────
    // BUG-8.c — per-app grouping
    // ─────────────────────────────────────────────────────────

    #[test]
    fn group_by_app_buckets_by_app_id_and_sorts_desc() {
        let mut rows = vec![
            make_row("a1", "alpha", "first", 100, false),
            make_row("a2", "alpha", "second", 200, false),
            make_row("b1", "beta", "third", 150, false),
        ];
        rows[0].app_id = "firefox".into();
        rows[1].app_id = "firefox".into();
        rows[2].app_id = "slack".into();
        let grouped = group_by_app(rows);
        assert_eq!(grouped.len(), 2);
        // BTreeMap keys are alphabetical.
        assert_eq!(grouped[0].0, "firefox");
        assert_eq!(grouped[1].0, "slack");
        // Within firefox: newer (200) first.
        assert_eq!(grouped[0].1.len(), 2);
        assert_eq!(grouped[0].1[0].id, "a2");
    }

    #[test]
    fn group_by_app_clusters_empty_app_ids_into_other() {
        let rows = vec![
            make_row("a1", "alpha", "first", 100, false),
            make_row("b1", "beta", "second", 200, false),
        ];
        let grouped = group_by_app(rows);
        // Both rows have empty app_id → both land in "Other".
        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[0].0, "Other");
        assert_eq!(grouped[0].1.len(), 2);
    }

    #[test]
    fn group_by_app_emits_other_only_when_present() {
        let mut rows = vec![make_row("a1", "alpha", "first", 100, false)];
        rows[0].app_id = "firefox".into();
        let grouped = group_by_app(rows);
        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[0].0, "firefox");
        // No "Other" bucket because there are no rows lacking
        // an app_id.
        assert!(grouped.iter().all(|(k, _)| k != "Other"));
    }

    #[test]
    fn app_id_round_trips_through_json() {
        // BUG-8.c — the parser must pick up app_id when the
        // notification daemon emits it, but old snapshots
        // without the field still parse (Option<>-style
        // default).
        let raw = r#"[
            {"id": "n1", "peer": "", "title": "T", "body": "B",
             "created_at": 1, "read": false, "dismissed": false,
             "app_id": "firefox"},
            {"id": "n2", "peer": "", "title": "T", "body": "B",
             "created_at": 2, "read": false, "dismissed": false}
        ]"#;
        let rows = parse_notifications(raw);
        assert_eq!(rows[0].app_id, "firefox");
        assert!(rows[1].app_id.is_empty());
    }

    #[test]
    fn phone_origin_round_trips_through_json() {
        // Wire-compat lock with the v1.x drawer's `origin:
        // "phone"` marker — snapshots from the old format
        // deserialize cleanly.
        let raw = r#"[
            {"id": "p1", "peer": "alpha", "title": "T", "body": "B",
             "created_at": 100, "read": false, "dismissed": false,
             "origin": "phone"}
        ]"#;
        let rows = parse_notifications(raw);
        assert_eq!(rows.len(), 1);
        assert!(is_phone_origin(&rows[0]));
    }
}
