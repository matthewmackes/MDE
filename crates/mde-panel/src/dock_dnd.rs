//! Phase E.9 — drag-to-pin / drag-to-reorder data model.
//!
//! The 1.x GTK version shipped `crates/mackes-panel/src/dock_dnd.rs`
//! with three GtkDnD primitives (`attach_dock_slot`,
//! `attach_tasklist_source`, `attach_pinned_strip_target`). Each
//! used X11 atoms (`mackes-dock-launcher-pos`,
//! `mackes-tasklist-pin`) to ferry indices/desktop-ids between
//! event sources and the drop target.
//!
//! The Iced port wraps the same drop-routing logic in pure-fn
//! helpers; the actual drag-recognition lands inside whichever
//! widget (dock applet, tasklist applet) consumes them. Drop
//! semantics route through `mde_config::with_mut(|cfg|
//! pin_app/reorder_dock)` exactly as the GTK version did, so the
//! 2s refresh tick continues to re-render within ~2s.

/// One pinned dock entry. `desktop_id` is the freedesktop
/// `.desktop` file basename (sans `.desktop`); `label` is the
/// human-readable display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedEntry {
    pub desktop_id: String,
    pub label: String,
}

/// Errors a drop can raise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropError {
    /// Source index out of bounds.
    SourceOutOfRange(usize),
    /// Destination index out of bounds.
    DestOutOfRange(usize),
    /// `desktop_id` already present (caller decides whether
    /// that's an error or a no-op).
    AlreadyPinned(String),
}

/// Reorder one pinned slot from `from_index` to `to_index`.
/// Returns a new pinned list; the original is unchanged.
pub fn reorder_dock(
    pinned: &[PinnedEntry],
    from_index: usize,
    to_index: usize,
) -> Result<Vec<PinnedEntry>, DropError> {
    if from_index >= pinned.len() {
        return Err(DropError::SourceOutOfRange(from_index));
    }
    if to_index > pinned.len() {
        return Err(DropError::DestOutOfRange(to_index));
    }
    let mut out: Vec<PinnedEntry> = pinned.to_vec();
    let entry = out.remove(from_index);
    // Account for the shift caused by remove().
    let adjusted = if to_index > from_index {
        to_index - 1
    } else {
        to_index
    };
    let bound = adjusted.min(out.len());
    out.insert(bound, entry);
    Ok(out)
}

/// Pin a new app onto the dock. If `desktop_id` already exists,
/// returns `Err(AlreadyPinned)`. Insert position is the end of
/// the pinned strip unless `at_index` is provided.
pub fn pin_app(
    pinned: &[PinnedEntry],
    new_entry: PinnedEntry,
    at_index: Option<usize>,
) -> Result<Vec<PinnedEntry>, DropError> {
    if pinned.iter().any(|p| p.desktop_id == new_entry.desktop_id) {
        return Err(DropError::AlreadyPinned(new_entry.desktop_id));
    }
    let mut out = pinned.to_vec();
    match at_index {
        Some(i) if i <= out.len() => out.insert(i, new_entry),
        Some(i) => return Err(DropError::DestOutOfRange(i)),
        None => out.push(new_entry),
    }
    Ok(out)
}

/// Unpin (remove) an entry by `desktop_id`. No-op when not
/// present.
#[must_use]
pub fn unpin(pinned: &[PinnedEntry], desktop_id: &str) -> Vec<PinnedEntry> {
    pinned
        .iter()
        .filter(|e| e.desktop_id != desktop_id)
        .cloned()
        .collect()
}

/// Recognized drag-source atom values. The 1.x GTK version used
/// `mackes-dock-launcher-pos` for slot reorders and
/// `mackes-tasklist-pin` for tasklist → pinned drops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragSource {
    /// Drag started from an existing pinned slot. Payload is
    /// the source slot index.
    DockSlot,
    /// Drag started from a tasklist item. Payload is the
    /// desktop-id of the dragged app.
    Tasklist,
}

impl DragSource {
    #[must_use]
    pub fn atom_name(&self) -> &'static str {
        match self {
            DragSource::DockSlot => "mde-dock-launcher-pos",
            DragSource::Tasklist => "mde-tasklist-pin",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<PinnedEntry> {
        vec![
            PinnedEntry {
                desktop_id: "firefox".into(),
                label: "Firefox".into(),
            },
            PinnedEntry {
                desktop_id: "foot".into(),
                label: "Terminal".into(),
            },
            PinnedEntry {
                desktop_id: "mde-files".into(),
                label: "Files".into(),
            },
        ]
    }

    #[test]
    fn reorder_dock_moves_entry_forward() {
        let out = reorder_dock(&fixture(), 0, 2).unwrap();
        let ids: Vec<&str> = out.iter().map(|e| e.desktop_id.as_str()).collect();
        assert_eq!(ids, vec!["foot", "firefox", "mde-files"]);
    }

    #[test]
    fn reorder_dock_moves_entry_backward() {
        let out = reorder_dock(&fixture(), 2, 0).unwrap();
        let ids: Vec<&str> = out.iter().map(|e| e.desktop_id.as_str()).collect();
        assert_eq!(ids, vec!["mde-files", "firefox", "foot"]);
    }

    #[test]
    fn reorder_dock_to_end_appends() {
        let out = reorder_dock(&fixture(), 0, 3).unwrap();
        let ids: Vec<&str> = out.iter().map(|e| e.desktop_id.as_str()).collect();
        assert_eq!(ids, vec!["foot", "mde-files", "firefox"]);
    }

    #[test]
    fn reorder_dock_same_index_is_noop_shape() {
        let out = reorder_dock(&fixture(), 1, 1).unwrap();
        let ids: Vec<&str> = out.iter().map(|e| e.desktop_id.as_str()).collect();
        assert_eq!(ids, vec!["firefox", "foot", "mde-files"]);
    }

    #[test]
    fn reorder_dock_rejects_source_out_of_range() {
        let err = reorder_dock(&fixture(), 99, 0).unwrap_err();
        assert!(matches!(err, DropError::SourceOutOfRange(99)));
    }

    #[test]
    fn reorder_dock_rejects_dest_out_of_range() {
        let err = reorder_dock(&fixture(), 0, 99).unwrap_err();
        assert!(matches!(err, DropError::DestOutOfRange(99)));
    }

    #[test]
    fn pin_app_appends_when_no_index() {
        let new = PinnedEntry {
            desktop_id: "code".into(),
            label: "VS Code".into(),
        };
        let out = pin_app(&fixture(), new, None).unwrap();
        assert_eq!(out.len(), 4);
        assert_eq!(out.last().unwrap().desktop_id, "code");
    }

    #[test]
    fn pin_app_inserts_at_index() {
        let new = PinnedEntry {
            desktop_id: "code".into(),
            label: "VS Code".into(),
        };
        let out = pin_app(&fixture(), new, Some(0)).unwrap();
        assert_eq!(out[0].desktop_id, "code");
    }

    #[test]
    fn pin_app_rejects_duplicate() {
        let new = PinnedEntry {
            desktop_id: "foot".into(),
            label: "Terminal Dup".into(),
        };
        let err = pin_app(&fixture(), new, None).unwrap_err();
        assert!(matches!(err, DropError::AlreadyPinned(ref id) if id == "foot"));
    }

    #[test]
    fn unpin_removes_matching_entry() {
        let out = unpin(&fixture(), "foot");
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|e| e.desktop_id != "foot"));
    }

    #[test]
    fn unpin_no_op_when_missing() {
        let out = unpin(&fixture(), "nonexistent");
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn drag_source_atom_names_match_v2_namespace() {
        assert_eq!(DragSource::DockSlot.atom_name(), "mde-dock-launcher-pos");
        assert_eq!(DragSource::Tasklist.atom_name(), "mde-tasklist-pin");
    }
}
