//! Phase 1.8 — search-results view (pure-fn filter).
//!
//! When the toolbar's search input is non-empty, the main pane
//! switches from the per-view list (mesh overview / peer folder /
//! local pins) to a flat results list, filtered across the current
//! scope.
//!
//! This module ships the pure data-side: a case-insensitive,
//! whitespace-trimming substring filter over [`FileRow`]. The view
//! layer plugs it into the visible list — that integration lives
//! with the Iced view-functions, not here.
//!
//! Match policy (locked 2026-05-19):
//!
//!   * Trim leading + trailing whitespace from the query before
//!     matching. An all-whitespace query matches nothing (treated
//!     as empty).
//!   * Match against `FileRow::name` and `FileRow::origin()` as a
//!     pair — "type the filename OR the peer name" both work.
//!   * Case-insensitive — `ASCII` only; Unicode case folding lands
//!     when we move to user-data backends (Phase 2.3+).
//!   * Empty / whitespace query returns the full input unchanged so
//!     the caller can use one helper for "search on, search off".

use crate::model::FileRow;

/// Apply the locked search policy to one row.
///
/// Empty / whitespace-only queries match everything (used so the
/// caller doesn't have to branch on "is search active?").
#[must_use]
pub fn matches_query(row: &FileRow, query: &str) -> bool {
    let q = query.trim();
    if q.is_empty() {
        return true;
    }
    let q_lower = q.to_ascii_lowercase();
    let name = row.name.to_ascii_lowercase();
    if name.contains(&q_lower) {
        return true;
    }
    if let Some(origin) = row.origin() {
        if origin.to_ascii_lowercase().contains(&q_lower) {
            return true;
        }
    }
    false
}

/// Filter a slice of rows in place. Returns owned `FileRow`s
/// because `FileRow` is `Copy` and the call site usually wants
/// ownership for the view tree.
#[must_use]
pub fn filter_rows(rows: &[FileRow], query: &str) -> Vec<FileRow> {
    rows.iter()
        .filter(|r| matches_query(r, query))
        .copied()
        .collect()
}

/// `true` when the query carries actual matchable characters
/// after the locked trim. The view code uses this to decide
/// whether to swap the main pane for the search-results view —
/// "search on" if-and-only-if `is_active(&search)`.
#[must_use]
pub fn is_active(query: &str) -> bool {
    !query.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Mime;

    fn row_local(name: &'static str) -> FileRow {
        FileRow::local(name, Mime::Doc, "1 KB", "now")
    }

    fn row_mesh(name: &'static str, peer: &'static str) -> FileRow {
        FileRow::local(name, Mime::Doc, "1 KB", "now").with_mesh(peer)
    }

    #[test]
    fn empty_query_matches_everything() {
        let r = row_local("anything.txt");
        assert!(matches_query(&r, ""));
        assert!(matches_query(&r, "   "));
        assert!(matches_query(&r, "\t"));
    }

    #[test]
    fn substring_in_name_matches() {
        let r = row_local("important-notes.md");
        assert!(matches_query(&r, "notes"));
        assert!(matches_query(&r, "important"));
        assert!(matches_query(&r, ".md"));
    }

    #[test]
    fn substring_match_is_case_insensitive() {
        let r = row_local("NOTES.MD");
        assert!(matches_query(&r, "notes"));
        assert!(matches_query(&r, "NoTeS"));
    }

    #[test]
    fn nonmatching_query_returns_false() {
        let r = row_local("alpha.txt");
        assert!(!matches_query(&r, "beta"));
        assert!(!matches_query(&r, "zzz"));
    }

    #[test]
    fn query_matches_origin_peer_name() {
        let r = row_mesh("data.bin", "pine.mesh");
        assert!(matches_query(&r, "pine"));
        assert!(matches_query(&r, "Pine"));
        assert!(matches_query(&r, "mesh"));
        // The filename alone still works.
        assert!(matches_query(&r, "data"));
    }

    #[test]
    fn whitespace_around_query_is_trimmed() {
        let r = row_local("notes.md");
        assert!(matches_query(&r, "  notes "));
        assert!(matches_query(&r, "\tnotes\n"));
    }

    #[test]
    fn filter_rows_returns_only_matches() {
        let rows = vec![
            row_local("alpha.txt"),
            row_local("beta.txt"),
            row_mesh("data.bin", "pine.mesh"),
        ];
        let out = filter_rows(&rows, "alpha");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "alpha.txt");

        let out = filter_rows(&rows, "pine");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "data.bin");

        // Empty query keeps everything.
        let out = filter_rows(&rows, "");
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn is_active_only_for_non_empty_queries() {
        assert!(!is_active(""));
        assert!(!is_active("   "));
        assert!(!is_active("\t\n"));
        assert!(is_active("x"));
        assert!(is_active("  x  "));
    }

    #[test]
    fn filter_with_no_match_returns_empty() {
        let rows = vec![row_local("x"), row_local("y")];
        let out = filter_rows(&rows, "z");
        assert!(out.is_empty());
    }
}
