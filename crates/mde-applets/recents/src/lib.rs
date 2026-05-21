//! Recents widget — overlay applet showing recently-opened
//! files.
//!
//! Phase E1.2.13: reads `$XDG_DATA_HOME/recently-used.xbel`
//! (the freedesktop spec for the recent-files list every
//! GTK app updates). Returns the N most-recent entries
//! with timestamps + URIs.

#![forbid(unsafe_code)]

use std::path::PathBuf;

use mde_applet_api::{AppletId, AppletSlot, HostMessage};

/// How many recent items to surface in the widget. Matches
/// the v1.x file-manager recents-default cap.
pub const RECENTS_CAP: usize = 12;

#[must_use]
pub fn manifest() -> mde_applet_api::AppletManifest {
    mde_applet_api::AppletManifest {
        id: AppletId::from_static("recents"),
        binary: "mde-applet-recents".into(),
        slot: AppletSlot::Overlay,
        summary: "Recently-opened files widget".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

/// One row of the recents list — extracted from a
/// `<bookmark href="..." modified="...">` element in the
/// XBEL XML.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecentRow {
    /// `file:///` URI (or other scheme).
    pub uri: String,
    /// ISO-8601 modified-time string (verbatim from the
    /// XBEL `modified` attribute).
    pub modified: String,
}

#[must_use]
pub fn recents_xbel_path() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .ok()
        .unwrap_or_else(|| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".local/share"))
                .unwrap_or_else(|_| PathBuf::from("/var/empty"))
        });
    base.join("recently-used.xbel")
}

/// Pure XBEL parser — just pulls `<bookmark href="..."
/// modified="...">` rows. Skips bad nesting + malformed
/// quotes. Returns rows in file order; the caller sorts.
#[must_use]
pub fn parse_xbel(raw: &str) -> Vec<RecentRow> {
    let mut rows = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("<bookmark") {
            continue;
        }
        let Some(uri) = extract_attr(trimmed, "href") else {
            continue;
        };
        let modified = extract_attr(trimmed, "modified").unwrap_or_default();
        rows.push(RecentRow { uri, modified });
    }
    rows
}

fn extract_attr(line: &str, key: &str) -> Option<String> {
    let needle = format!("{key}=\"");
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Sort + truncate to the cap. Sort key is the modified
/// timestamp DESC (newest first) — string compare is OK for
/// ISO-8601.
#[must_use]
pub fn top_n(rows: Vec<RecentRow>, n: usize) -> Vec<RecentRow> {
    let mut sorted = rows;
    sorted.sort_by(|a, b| b.modified.cmp(&a.modified));
    sorted.truncate(n);
    sorted
}

/// Render the widget as a one-line-per-row text block.
#[must_use]
pub fn format_widget(rows: &[RecentRow]) -> String {
    if rows.is_empty() {
        return "(no recent files)".to_string();
    }
    rows.iter()
        .map(|r| {
            // Strip the file:// prefix when present to keep
            // the rendered line readable.
            let display = r.uri.strip_prefix("file://").unwrap_or(&r.uri);
            format!("{}  {}", r.modified, display)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[must_use]
pub fn handle_host(msg: &HostMessage) -> bool {
    !matches!(msg, HostMessage::Shutdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_lands_in_overlay_slot() {
        let m = manifest();
        assert_eq!(m.id.as_str(), "recents");
        assert_eq!(m.slot, AppletSlot::Overlay);
    }

    #[test]
    fn recents_cap_lock() {
        assert_eq!(RECENTS_CAP, 12);
    }

    #[test]
    fn parse_xbel_extracts_href_and_modified() {
        let raw = r#"<xbel version="1.0">
  <bookmark href="file:///home/u/notes.md" added="2024-05-01T10:00:00Z" modified="2024-05-10T11:30:00Z" visited="2024-05-10T11:30:00Z"/>
  <bookmark href="file:///home/u/photo.png" modified="2024-05-09T08:00:00Z"/>
</xbel>"#;
        let rows = parse_xbel(raw);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].uri, "file:///home/u/notes.md");
        assert_eq!(rows[0].modified, "2024-05-10T11:30:00Z");
        assert_eq!(rows[1].uri, "file:///home/u/photo.png");
    }

    #[test]
    fn parse_xbel_empty_on_malformed_input() {
        assert!(parse_xbel("").is_empty());
        assert!(parse_xbel("not xml at all").is_empty());
    }

    #[test]
    fn top_n_sorts_by_modified_desc_and_truncates() {
        let rows = vec![
            RecentRow {
                uri: "file:///a".into(),
                modified: "2024-01-01T00:00:00Z".into(),
            },
            RecentRow {
                uri: "file:///b".into(),
                modified: "2024-05-10T11:30:00Z".into(),
            },
            RecentRow {
                uri: "file:///c".into(),
                modified: "2024-03-15T00:00:00Z".into(),
            },
        ];
        let top = top_n(rows, 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].uri, "file:///b");
        assert_eq!(top[1].uri, "file:///c");
    }

    #[test]
    fn format_widget_strips_file_uri_prefix() {
        let rows = vec![RecentRow {
            uri: "file:///home/u/note.md".into(),
            modified: "2024-05-10T11:30:00Z".into(),
        }];
        let s = format_widget(&rows);
        assert!(s.contains("/home/u/note.md"));
        assert!(!s.contains("file:///"));
    }

    #[test]
    fn format_widget_empty_message() {
        assert_eq!(format_widget(&[]), "(no recent files)");
    }

    #[test]
    fn handle_host_short_circuits_shutdown() {
        assert!(!handle_host(&HostMessage::Shutdown));
    }
}
