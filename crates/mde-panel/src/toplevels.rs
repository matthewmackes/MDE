//! Phase E.3 — foreign-toplevel listener data model.
//!
//! `wlr_foreign_toplevel_management_v1` lets the panel observe
//! every top-level window the compositor knows about. The
//! standalone applets (dock E1.2.7, app-switcher E1.2.11) already
//! consume this via `swayipc-async EventStream(Window)` in their
//! own processes; this module ships the panel-side data model so
//! the panel's Hero widget (E.4.2) + tasklist subscription can
//! pick up focus changes without re-implementing the parser.
//!
//! The eventual SCTK integration writes `ToplevelEvent` values
//! into an `iced::Subscription` channel; consumers (Hero,
//! Tasklist) reduce against that stream.

use std::collections::HashMap;

/// Stable identifier for a top-level window.
pub type ToplevelId = u64;

/// One observed top-level window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toplevel {
    pub id: ToplevelId,
    pub title: String,
    pub app_id: String,
    pub state: ToplevelState,
}

/// Window state flags. Sway/wlroots report all four
/// orthogonally; we mirror them as-is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToplevelState {
    pub focused: bool,
    pub fullscreen: bool,
    pub minimized: bool,
    pub maximized: bool,
}

/// Events the foreign-toplevel listener emits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToplevelEvent {
    /// Window appeared.
    Added(Toplevel),
    /// Window updated (title / app_id / state changed).
    Updated(Toplevel),
    /// Window removed.
    Removed(ToplevelId),
    /// Compositor lost the foreign-toplevel manager — clear the
    /// model and re-enumerate on the next Added event.
    Disconnected,
}

/// In-memory model of every observed top-level. Consumers fold
/// the `ToplevelEvent` stream into this.
#[derive(Debug, Default, Clone)]
pub struct ToplevelModel {
    by_id: HashMap<ToplevelId, Toplevel>,
}

impl ToplevelModel {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a single event, returning whether the model changed.
    pub fn apply(&mut self, event: ToplevelEvent) -> bool {
        match event {
            ToplevelEvent::Added(t) | ToplevelEvent::Updated(t) => {
                let id = t.id;
                let changed = self.by_id.get(&id) != Some(&t);
                self.by_id.insert(id, t);
                changed
            }
            ToplevelEvent::Removed(id) => self.by_id.remove(&id).is_some(),
            ToplevelEvent::Disconnected => {
                let was_non_empty = !self.by_id.is_empty();
                self.by_id.clear();
                was_non_empty
            }
        }
    }

    /// Number of observed top-levels.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// All observed top-levels, sorted by id (stable order).
    #[must_use]
    pub fn ordered(&self) -> Vec<Toplevel> {
        let mut v: Vec<Toplevel> = self.by_id.values().cloned().collect();
        v.sort_by_key(|t| t.id);
        v
    }

    /// The currently-focused top-level, if any.
    #[must_use]
    pub fn focused(&self) -> Option<&Toplevel> {
        self.by_id.values().find(|t| t.state.focused)
    }

    /// All top-levels with `app_id` matching the predicate.
    pub fn filter<F: FnMut(&Toplevel) -> bool>(&self, mut pred: F) -> Vec<Toplevel> {
        self.by_id.values().filter(|t| pred(t)).cloned().collect()
    }
}

/// Pure helper — given a focus change (new focus id), return
/// the events that should fire to reflect it. The previous
/// focused window gets a state-update event with `focused=false`
/// and the new one gets `focused=true`.
#[must_use]
pub fn focus_change_events(model: &ToplevelModel, new_focus: ToplevelId) -> Vec<ToplevelEvent> {
    let mut events = Vec::new();
    let new_target = model.by_id.get(&new_focus).cloned();

    for (id, t) in &model.by_id {
        if t.state.focused && *id != new_focus {
            let mut updated = t.clone();
            updated.state.focused = false;
            events.push(ToplevelEvent::Updated(updated));
        }
    }

    if let Some(mut t) = new_target {
        if !t.state.focused {
            t.state.focused = true;
            events.push(ToplevelEvent::Updated(t));
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock(id: u64, title: &str, focused: bool) -> Toplevel {
        Toplevel {
            id,
            title: title.into(),
            app_id: format!("app-{id}"),
            state: ToplevelState {
                focused,
                ..Default::default()
            },
        }
    }

    #[test]
    fn empty_model_starts_empty() {
        let m = ToplevelModel::new();
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
        assert!(m.focused().is_none());
    }

    #[test]
    fn added_event_increments_count() {
        let mut m = ToplevelModel::new();
        let changed = m.apply(ToplevelEvent::Added(mock(1, "A", false)));
        assert!(changed);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn removed_event_drops_window() {
        let mut m = ToplevelModel::new();
        m.apply(ToplevelEvent::Added(mock(1, "A", false)));
        m.apply(ToplevelEvent::Added(mock(2, "B", false)));
        m.apply(ToplevelEvent::Removed(1));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn updated_event_replaces_in_place() {
        let mut m = ToplevelModel::new();
        m.apply(ToplevelEvent::Added(mock(1, "Old", false)));
        let changed = m.apply(ToplevelEvent::Updated(mock(1, "New", false)));
        assert!(changed);
        assert_eq!(m.len(), 1);
        assert_eq!(m.ordered()[0].title, "New");
    }

    #[test]
    fn updated_event_with_same_value_reports_no_change() {
        let mut m = ToplevelModel::new();
        m.apply(ToplevelEvent::Added(mock(1, "A", false)));
        let changed = m.apply(ToplevelEvent::Updated(mock(1, "A", false)));
        assert!(!changed);
    }

    #[test]
    fn disconnected_clears_model_only_when_non_empty() {
        let mut m = ToplevelModel::new();
        let changed = m.apply(ToplevelEvent::Disconnected);
        assert!(!changed);

        m.apply(ToplevelEvent::Added(mock(1, "A", false)));
        let changed = m.apply(ToplevelEvent::Disconnected);
        assert!(changed);
        assert!(m.is_empty());
    }

    #[test]
    fn focused_returns_the_focused_window() {
        let mut m = ToplevelModel::new();
        m.apply(ToplevelEvent::Added(mock(1, "A", false)));
        m.apply(ToplevelEvent::Added(mock(2, "B", true)));
        m.apply(ToplevelEvent::Added(mock(3, "C", false)));
        assert_eq!(m.focused().unwrap().id, 2);
    }

    #[test]
    fn focus_change_events_clears_old_and_sets_new() {
        let mut m = ToplevelModel::new();
        m.apply(ToplevelEvent::Added(mock(1, "A", true)));
        m.apply(ToplevelEvent::Added(mock(2, "B", false)));
        let events = focus_change_events(&m, 2);
        assert_eq!(events.len(), 2);
        // First event clears 1's focus.
        if let ToplevelEvent::Updated(t) = &events[0] {
            assert_eq!(t.id, 1);
            assert!(!t.state.focused);
        } else {
            panic!("expected Updated(1)");
        }
        // Second sets 2's focus.
        if let ToplevelEvent::Updated(t) = &events[1] {
            assert_eq!(t.id, 2);
            assert!(t.state.focused);
        } else {
            panic!("expected Updated(2)");
        }
    }

    #[test]
    fn focus_change_when_target_already_focused_is_noop() {
        let mut m = ToplevelModel::new();
        m.apply(ToplevelEvent::Added(mock(1, "A", true)));
        let events = focus_change_events(&m, 1);
        assert!(events.is_empty());
    }

    #[test]
    fn ordered_is_sorted_by_id() {
        let mut m = ToplevelModel::new();
        m.apply(ToplevelEvent::Added(mock(3, "C", false)));
        m.apply(ToplevelEvent::Added(mock(1, "A", false)));
        m.apply(ToplevelEvent::Added(mock(2, "B", false)));
        let ids: Vec<u64> = m.ordered().iter().map(|t| t.id).collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn filter_returns_matching_subset() {
        let mut m = ToplevelModel::new();
        m.apply(ToplevelEvent::Added(mock(1, "A", false)));
        m.apply(ToplevelEvent::Added(mock(2, "B", true)));
        m.apply(ToplevelEvent::Added(mock(3, "C", false)));
        let focused: Vec<_> = m.filter(|t| t.state.focused);
        assert_eq!(focused.len(), 1);
        assert_eq!(focused[0].id, 2);
    }
}
