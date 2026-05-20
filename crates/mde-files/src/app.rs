//! Iced application — top-level State, Message, update, view.

use iced::widget::{column, container, row, scrollable};
use iced::{Background, Border, Color, Element, Length, Padding, Size, Task, Theme};

use crate::demo_data as data;
use crate::model::{Layout, View};
use crate::panels::{
    ContextMenu, ContextMenuItem, DetailsPanel, DragSession, DragTarget, OperationDrawer, OpRow,
};
use crate::prefs::Accessibility;
use crate::send_to::SendToRequest;
use crate::selection::Selection;
use crate::theme as t;
use crate::views;

#[derive(Debug, Clone)]
pub enum Message {
    SelectView(View),
    ToggleLocal,
    SetLayout(Layout),
    SearchChanged(String),
    Refresh,
    TitlebarMinimize,
    TitlebarMaximize,
    TitlebarClose,
    PeerCardBrowse(&'static str),
    PeerCardSend(&'static str),
    PrimaryAction,
    /// v2.0.0 Phase 1.3 — plain click on a file row.
    RowClick(String),
    /// v2.0.0 Phase 1.3 — ctrl-click on a file row (toggle in
    /// selection).
    RowCtrlClick(String),
    /// v2.0.0 Phase 1.3 — shift-click on a file row. The view
    /// passes the visible row order so the selection model can
    /// build the inclusive range.
    RowShiftClick(String, Vec<String>),
    /// v2.0.0 Phase 1.3 — keyboard down / up arrows. The visible
    /// row order is supplied so wrap-around behaves correctly.
    FocusNext(Vec<String>),
    /// v2.0.0 Phase 1.3 — keyboard up arrow.
    FocusPrev(Vec<String>),
    /// v2.0.0 Phase 1.3 — keyboard space-bar: toggle focused row.
    ToggleFocused,
    /// v2.0.0 Phase 1.3 — keyboard Escape: clear selection.
    ClearSelection,
    /// v2.0.0 Phase 1.4 — toggle the right-side details panel.
    ToggleDetails,
    /// v2.0.0 Phase 1.5 — open the right-click context menu over
    /// the given row at the given window-coord anchor.
    OpenContextMenu(String, f32, f32),
    /// v2.0.0 Phase 1.5 — close the context menu.
    CloseContextMenu,
    /// v2.0.0 Phase 1.5 — a context-menu item was clicked. View
    /// code routes this to the appropriate side-effect (Send-To
    /// dialog, clipboard, etc.); the reducer just closes the
    /// menu so the floating widget disappears.
    ContextMenuItemClicked(ContextMenuItem),
    /// v2.0.0 Phase 1.7 — show / hide the operation drawer.
    ToggleOperationDrawer,
    /// v2.0.0 Phase 1.7 — backend pushed a fresh op row (new or
    /// progress update).
    OpRowUpsert(OpRow),
    /// v2.0.0 Phase 1.7 — dismiss a terminal op from the drawer.
    OpRowDismiss(u64),
    /// v2.0.0 Phase 1.6 — user grabbed a row (or the current
    /// selection) and started dragging.
    DragStart(Vec<String>),
    /// v2.0.0 Phase 1.6 — cursor entered (`Some`) or left (`None`)
    /// a drop target.
    DragHover(Option<DragTarget>),
    /// v2.0.0 Phase 1.6 — user dropped over a target (or empty
    /// space). The reducer translates a target landing into a
    /// `Backend::send_to` call at the view-side; here it just
    /// finishes the drag session.
    DragDrop,
    /// v2.0.0 Phase 1.6 — user pressed Escape mid-drag.
    DragCancel,
    /// v2.0.0 Phase 3.1 — canonical Send-To dispatch. Every
    /// entry point (toolbar / context menu / command palette /
    /// drag-drop / details panel / bulk-select bar) builds a
    /// `SendToRequest` + fires this message.
    SendTo(SendToRequest),
    /// v2.0.0 Phase 5.1 — Tab cycles keyboard focus through panes.
    TabFocus,
    /// v2.0.0 Phase 5.1 — Shift-Tab reverses.
    ShiftTabFocus,
    /// v2.0.0 Phase 5.1 — Ctrl/Cmd-F focuses the toolbar search
    /// field.
    FocusSearch,
    /// v2.0.0 Phase 5.1 — any keyboard input arrived. Flips
    /// `keyboard_active = true` so `FocusVisibility::Auto`
    /// renders rings.
    KeyboardActivity,
    /// v2.0.0 Phase 5.1 — mouse moved / clicked. Flips
    /// `keyboard_active = false`.
    PointerActivity,
    /// No-op message used by buttons that don't have a wired behaviour yet
    /// (e.g. the sidebar's panel-toggle, the peer card's `…` button).
    Noop,
}

/// Breadcrumb segment used by the toolbar.
#[derive(Debug, Clone)]
pub struct Crumb {
    pub label: String,
    /// True if this crumb belongs to a mesh path. Affects colour + the trailing tag chip.
    pub mesh: bool,
}

#[derive(Debug, Default)]
pub struct MdeFiles {
    pub view: View,
    pub local_open: bool,
    pub layout: Layout,
    pub search: String,
    /// v2.0.0 Phase 1.3 — row selection state (focus + anchor +
    /// selected set). Cleared on view change.
    pub selection: Selection,
    /// v2.0.0 Phase 1.4 — right-side details panel state.
    pub details: DetailsPanel,
    /// v2.0.0 Phase 1.5 — right-click context-menu state.
    pub context_menu: ContextMenu,
    /// v2.0.0 Phase 1.7 — slide-up operation drawer state.
    pub op_drawer: OperationDrawer,
    /// v2.0.0 Phase 1.6 — drag-and-drop session state.
    pub drag: DragSession,
    /// v2.0.0 Phase 5.x — accessibility prefs (direction / motion
    /// / focus-ring policy). Loaded once at startup from
    /// `Accessibility::load_from_env`. View code reads these each
    /// frame.
    pub a11y: Accessibility,
    /// v2.0.0 Phase 5.1 — which pane currently owns keyboard focus.
    /// Tab cycles through the locked order: Toolbar → Sidebar →
    /// FileList. Used by the focus-ring renderer + the keyboard
    /// dispatcher.
    pub keyboard_pane: KeyboardPane,
    /// v2.0.0 Phase 5.1 — whether the most recent input was a
    /// keyboard interaction. `FocusVisibility::Auto` consults this
    /// to decide whether to render focus rings.
    pub keyboard_active: bool,
}

/// v2.0.0 Phase 5.1 — pane currently receiving keyboard input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyboardPane {
    /// Toolbar (search input + layout toggle).
    Toolbar,
    /// Left-rail sidebar (peer + pin list).
    Sidebar,
    /// Main file-list pane.
    #[default]
    FileList,
}

impl KeyboardPane {
    /// Tab order: Toolbar → Sidebar → FileList → Toolbar.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Toolbar => Self::Sidebar,
            Self::Sidebar => Self::FileList,
            Self::FileList => Self::Toolbar,
        }
    }

    /// Shift-Tab: reverse direction.
    #[must_use]
    pub fn prev(self) -> Self {
        match self {
            Self::Toolbar => Self::FileList,
            Self::Sidebar => Self::Toolbar,
            Self::FileList => Self::Sidebar,
        }
    }
}

impl MdeFiles {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the Iced application.
    ///
    /// Builds a fresh `MdeFiles` state, registers the warm-dark theme, opens a
    /// 1480×940 window, and dispatches updates from `Message`.
    pub fn run() -> iced::Result {
        iced::application(Self::title, Self::update, Self::view)
            .theme(Self::theme)
            .window_size(Size::new(t::WIN_W, t::WIN_H))
            .run()
    }

    fn title(&self) -> String {
        "Artifact Manager".into()
    }

    fn theme(&self) -> Theme {
        t::theme()
    }

    /// Update reducer — every interaction in the UI flows through this single
    /// function. No async work happens here yet (the demo backend is in-memory);
    /// once `mded` is wired, several variants will return real `Task`s.
    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::SelectView(v) => {
                self.view = v;
                if !matches!(v, View::Local) {
                    self.local_open = false;
                }
                // Phase 1.3 — selection is per-view; clear on
                // navigation so stale row keys don't leak across
                // peer folders.
                self.selection.clear();
            }
            Message::ToggleLocal => {
                self.local_open = !self.local_open;
                if self.local_open && !matches!(self.view, View::Local) {
                    self.view = View::Local;
                    self.selection.clear();
                } else if !self.local_open && matches!(self.view, View::Local) {
                    self.view = View::default();
                    self.selection.clear();
                }
            }
            Message::SetLayout(l) => self.layout = l,
            Message::SearchChanged(s) => self.search = s,
            Message::PeerCardBrowse(id) => {
                self.view = View::Peer(id);
                self.selection.clear();
            }
            Message::RowClick(key) => {
                self.selection.click(key);
                // Phase 1.4 — details panel tracks focus.
                self.details.set_target(self.selection.focused());
            }
            Message::RowCtrlClick(key) => {
                self.selection.ctrl_click(key);
                self.details.set_target(self.selection.focused());
            }
            Message::RowShiftClick(key, rows) => {
                self.selection.shift_click(key, &rows);
                self.details.set_target(self.selection.focused());
            }
            Message::FocusNext(rows) => {
                self.selection.focus_next(&rows);
                self.details.set_target(self.selection.focused());
            }
            Message::FocusPrev(rows) => {
                self.selection.focus_prev(&rows);
                self.details.set_target(self.selection.focused());
            }
            Message::ToggleFocused => self.selection.toggle_focused(),
            Message::ClearSelection => {
                self.selection.clear();
                self.details.set_target(None);
            }
            Message::ToggleDetails => {
                self.details.toggle(self.selection.focused());
            }
            Message::OpenContextMenu(row, x, y) => {
                self.context_menu.open(row, (x, y));
            }
            Message::CloseContextMenu => self.context_menu.close(),
            Message::ContextMenuItemClicked(_item) => {
                // The actual side-effect routing (Send-To dialog,
                // clipboard, properties window) happens at the
                // view-side. The reducer just dismisses the
                // floating menu so it doesn't linger after the
                // click.
                self.context_menu.close();
            }
            Message::ToggleOperationDrawer => {
                let open = !self.op_drawer.is_open();
                self.op_drawer.set_open(open);
            }
            Message::OpRowUpsert(row) => self.op_drawer.upsert(row),
            Message::OpRowDismiss(id) => {
                self.op_drawer.dismiss(id);
            }
            Message::DragStart(rows) => self.drag.start(rows),
            Message::DragHover(target) => self.drag.set_hover(target),
            Message::DragDrop => {
                // The actual `Backend::send_to` call lives at the
                // view-side because the reducer is sync and the
                // backend is mut. `finish()` returns the drop
                // target so the view can route the call; here we
                // just clean up the session state.
                let _ = self.drag.finish();
            }
            Message::DragCancel => {
                let _ = self.drag.cancel();
            }
            Message::SendTo(_req) => {
                // View-side handlers (the `Backend` trait
                // consumer in `mde-files::main`) pick this up and
                // route to `Backend::send_to`. The reducer is sync
                // + the backend is mut, so we don't perform the
                // call here; instead `Subscription`-driven side-
                // effect Tasks (Phase 2.3 + 2.6) take the request
                // from here. The Phase 3.1 work is the
                // canonical Message shape — the wire-up to
                // mded.Shell.Send is the Phase 2.3 DBus backend's
                // job.
            }
            Message::TabFocus => {
                self.keyboard_pane = self.keyboard_pane.next();
                self.keyboard_active = true;
            }
            Message::ShiftTabFocus => {
                self.keyboard_pane = self.keyboard_pane.prev();
                self.keyboard_active = true;
            }
            Message::FocusSearch => {
                self.keyboard_pane = KeyboardPane::Toolbar;
                self.keyboard_active = true;
            }
            Message::KeyboardActivity => self.keyboard_active = true,
            Message::PointerActivity => self.keyboard_active = false,
            Message::Refresh
            | Message::TitlebarMinimize
            | Message::TitlebarMaximize
            | Message::TitlebarClose
            | Message::PeerCardSend(_)
            | Message::PrimaryAction
            | Message::Noop => {
                // Demo backend has no side effects to run; leave these as routing hooks
                // for the future `Backend::DBus` impl.
            }
        }
        Task::none()
    }

    /// Top-level view tree.
    pub fn view(&self) -> Element<'_, Message> {
        let crumbs = breadcrumbs_for(self.view);

        let main_body: Element<'_, Message> = match self.view {
            View::MeshOverview => views::mesh_overview(&data::SELF_NODE),
            View::Inbox        => views::inbox(&data::SELF_NODE),
            View::Peer(id) => {
                if let Some(peer) = data::PEERS.iter().find(|p| p.id == id) {
                    views::peer_folder(peer, &data::SELF_NODE)
                } else {
                    empty_state("no peer").into()
                }
            }
            View::Downloads => views::downloads(),
            View::Local     => views::local_veil(&data::SELF_NODE),
        };

        let content = container(scrollable(
            container(main_body).padding(Padding { top: 18.0, right: 22.0, bottom: 28.0, left: 22.0 }),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(t::PF_BG_300)),
            border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 0.0.into() },
            ..container::Style::default()
        });

        let main = column![
            views::toolbar(self.view, self.layout, &self.search, crumbs),
            content,
        ]
        .spacing(0);

        let body = row![
            views::sidebar(self.view, self.local_open, &data::SELF_NODE),
            container(main).width(Length::Fill).height(Length::Fill),
        ]
        .height(Length::Fill);

        let online = data::online_count();
        let total = data::PEERS.len();

        container(
            column![views::titlebar(online, total), body].spacing(0),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(t::WINDOW)),
            border: Border { color: Color { a: 0.08, ..Color::WHITE }, width: 1.0, radius: 0.0.into() },
            ..container::Style::default()
        })
        .into()
    }
}

fn breadcrumbs_for(view: View) -> Vec<Crumb> {
    match view {
        View::MeshOverview => vec![
            Crumb { label: "Mesh".into(),     mesh: true },
            Crumb { label: "Overview".into(), mesh: false },
        ],
        View::Inbox => vec![
            Crumb { label: "Mesh".into(),  mesh: true },
            Crumb { label: "Inbox".into(), mesh: false },
        ],
        View::Peer(id) => {
            let host = data::PEERS.iter().find(|p| p.id == id).map_or("?", |p| p.host);
            vec![
                Crumb { label: "Mesh".into(),     mesh: true },
                Crumb { label: host.to_string(), mesh: false },
            ]
        }
        View::Downloads => vec![
            Crumb { label: "~".into(),         mesh: false },
            Crumb { label: "Downloads".into(), mesh: false },
        ],
        View::Local => vec![
            Crumb { label: "Local".into(),                     mesh: false },
            Crumb { label: data::SELF_NODE.host.to_string(),   mesh: false },
            Crumb { label: "/".into(),                          mesh: false },
        ],
    }
}

fn empty_state(label: &str) -> Element<'static, Message> {
    container(iced::widget::text(label.to_string()).size(12).color(t::FG_FAINT))
        .padding(Padding::new(56.0))
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            border: Border { color: Color { a: 0.10, ..Color::WHITE }, width: 1.0, radius: 0.0.into() },
            ..container::Style::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_view_is_mesh_overview() {
        let s = MdeFiles::default();
        assert_eq!(s.view, View::MeshOverview);
        assert!(!s.local_open);
        assert_eq!(s.layout, Layout::List);
        assert!(s.search.is_empty());
    }

    #[test]
    fn toggle_local_opens_local_view() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::ToggleLocal);
        assert!(s.local_open);
        assert_eq!(s.view, View::Local);
        let _ = s.update(Message::ToggleLocal);
        assert!(!s.local_open);
        assert_eq!(s.view, View::MeshOverview);
    }

    #[test]
    fn selecting_non_local_view_closes_local_disclosure() {
        let mut s = MdeFiles::default();
        s.local_open = true;
        s.view = View::Local;
        let _ = s.update(Message::SelectView(View::Inbox));
        assert_eq!(s.view, View::Inbox);
        assert!(!s.local_open, "local disclosure must close when leaving Local view");
    }

    #[test]
    fn peer_card_browse_routes_to_peer_view() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::PeerCardBrowse("pine"));
        assert_eq!(s.view, View::Peer("pine"));
    }

    #[test]
    fn row_click_message_updates_selection() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::RowClick("doc.txt".into()));
        assert_eq!(s.selection.len(), 1);
        assert!(s.selection.is_selected("doc.txt"));
    }

    #[test]
    fn row_ctrl_click_toggles() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::RowCtrlClick("a".into()));
        let _ = s.update(Message::RowCtrlClick("b".into()));
        assert_eq!(s.selection.len(), 2);
        let _ = s.update(Message::RowCtrlClick("a".into()));
        assert_eq!(s.selection.len(), 1);
        assert!(s.selection.is_selected("b"));
    }

    #[test]
    fn row_shift_click_extends_range() {
        let mut s = MdeFiles::default();
        let rows: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        let _ = s.update(Message::RowClick("a".into()));
        let _ = s.update(Message::RowShiftClick("c".into(), rows));
        assert_eq!(s.selection.len(), 3);
    }

    #[test]
    fn focus_next_and_prev_messages() {
        let mut s = MdeFiles::default();
        let rows: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        let _ = s.update(Message::FocusNext(rows.clone()));
        assert_eq!(s.selection.focused(), Some("a"));
        let _ = s.update(Message::FocusPrev(rows));
        assert_eq!(s.selection.focused(), Some("c"));
    }

    #[test]
    fn toggle_focused_message() {
        let mut s = MdeFiles::default();
        let rows: Vec<String> = vec!["x".into()];
        let _ = s.update(Message::FocusNext(rows));
        let _ = s.update(Message::ToggleFocused);
        assert!(s.selection.is_selected("x"));
    }

    #[test]
    fn clear_selection_message_resets() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::RowClick("x".into()));
        let _ = s.update(Message::ClearSelection);
        assert!(s.selection.is_empty());
    }

    #[test]
    fn view_change_clears_selection() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::RowClick("x".into()));
        assert!(!s.selection.is_empty());
        let _ = s.update(Message::SelectView(View::Inbox));
        assert!(s.selection.is_empty(), "view change must clear selection");
    }

    #[test]
    fn peer_card_browse_clears_selection() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::RowClick("x".into()));
        let _ = s.update(Message::PeerCardBrowse("pine"));
        assert!(s.selection.is_empty());
    }

    #[test]
    fn toggle_details_panel_message() {
        let mut s = MdeFiles::default();
        // No focus → toggle is a no-op (Phase 1.4 lock).
        let _ = s.update(Message::ToggleDetails);
        assert!(!s.details.is_open());
        // After focusing a row, toggle opens it.
        let _ = s.update(Message::RowClick("a.txt".into()));
        let _ = s.update(Message::ToggleDetails);
        assert!(s.details.is_open());
        assert_eq!(s.details.target(), Some("a.txt"));
    }

    #[test]
    fn row_click_updates_details_target_when_open() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::RowClick("a".into()));
        let _ = s.update(Message::ToggleDetails);
        let _ = s.update(Message::RowClick("b".into()));
        assert_eq!(s.details.target(), Some("b"));
        assert!(s.details.is_open());
    }

    #[test]
    fn clear_selection_closes_details() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::RowClick("a".into()));
        let _ = s.update(Message::ToggleDetails);
        assert!(s.details.is_open());
        let _ = s.update(Message::ClearSelection);
        assert!(!s.details.is_open(), "details hides when nothing selected");
    }

    #[test]
    fn open_context_menu_message() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::OpenContextMenu("a.txt".into(), 100.0, 200.0));
        assert!(s.context_menu.is_open());
        assert_eq!(s.context_menu.row(), Some("a.txt"));
        assert_eq!(s.context_menu.anchor(), Some((100.0, 200.0)));
    }

    #[test]
    fn context_menu_item_clicked_closes_menu() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::OpenContextMenu("a.txt".into(), 0.0, 0.0));
        let _ = s.update(Message::ContextMenuItemClicked(ContextMenuItem::Open));
        assert!(!s.context_menu.is_open());
    }

    #[test]
    fn toggle_operation_drawer_message() {
        let mut s = MdeFiles::default();
        assert!(!s.op_drawer.is_open());
        let _ = s.update(Message::ToggleOperationDrawer);
        assert!(s.op_drawer.is_open());
        let _ = s.update(Message::ToggleOperationDrawer);
        assert!(!s.op_drawer.is_open());
    }

    #[test]
    fn op_row_upsert_and_dismiss_messages() {
        use crate::panels::{OpRow, OpState};
        let mut s = MdeFiles::default();
        let row = OpRow {
            op_id: 7,
            source: "a.txt".into(),
            destination: "pine".into(),
            progress_permille: 500,
            state: OpState::Running,
        };
        let _ = s.update(Message::OpRowUpsert(row));
        assert_eq!(s.op_drawer.len(), 1);
        let _ = s.update(Message::OpRowDismiss(7));
        assert_eq!(s.op_drawer.len(), 0);
    }

    #[test]
    fn drag_start_and_drop_messages() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::DragStart(vec!["a.txt".into(), "b.txt".into()]));
        assert!(s.drag.is_active());
        assert_eq!(s.drag.sources().len(), 2);
        let _ = s.update(Message::DragHover(Some(DragTarget::Peer("pine".into()))));
        assert_eq!(s.drag.hover_target(), Some(&DragTarget::Peer("pine".into())));
        let _ = s.update(Message::DragDrop);
        assert!(!s.drag.is_active(), "drag finishes after drop");
    }

    #[test]
    fn drag_cancel_message() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::DragStart(vec!["a".into()]));
        let _ = s.update(Message::DragCancel);
        assert!(!s.drag.is_active());
    }

    #[test]
    fn tab_focus_cycles_through_panes() {
        let mut s = MdeFiles::default();
        assert_eq!(s.keyboard_pane, KeyboardPane::FileList);
        let _ = s.update(Message::TabFocus);
        assert_eq!(s.keyboard_pane, KeyboardPane::Toolbar);
        let _ = s.update(Message::TabFocus);
        assert_eq!(s.keyboard_pane, KeyboardPane::Sidebar);
        let _ = s.update(Message::TabFocus);
        assert_eq!(s.keyboard_pane, KeyboardPane::FileList);
    }

    #[test]
    fn shift_tab_focus_reverses() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::ShiftTabFocus);
        assert_eq!(s.keyboard_pane, KeyboardPane::Sidebar);
        let _ = s.update(Message::ShiftTabFocus);
        assert_eq!(s.keyboard_pane, KeyboardPane::Toolbar);
    }

    #[test]
    fn focus_search_jumps_to_toolbar() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::FocusSearch);
        assert_eq!(s.keyboard_pane, KeyboardPane::Toolbar);
        assert!(s.keyboard_active);
    }

    #[test]
    fn keyboard_activity_toggles_keyboard_active_flag() {
        let mut s = MdeFiles::default();
        assert!(!s.keyboard_active);
        let _ = s.update(Message::KeyboardActivity);
        assert!(s.keyboard_active);
        let _ = s.update(Message::PointerActivity);
        assert!(!s.keyboard_active);
    }

    #[test]
    fn keyboard_pane_tab_order_is_three_step_cycle() {
        let start = KeyboardPane::Toolbar;
        let one = start.next();
        let two = one.next();
        let three = two.next();
        assert_eq!(three, start, "Tab returns to start after 3 hops");
    }

    #[test]
    fn send_to_message_is_a_silent_routing_hook() {
        use crate::backend::{ConflictPolicy, Destination, SendMode};
        use crate::send_to::{SendToEntry, SendToRequest};
        let mut s = MdeFiles::default();
        // The reducer just routes — no observable state change.
        // The DemoBackend doesn't get called from here (the
        // view-side `Backend` consumer does that), so we only
        // assert the message round-trips without panicking.
        let req = SendToRequest {
            sources: vec![std::path::PathBuf::from("/tmp/a.txt")],
            destination: Destination::Peer("pine".into()),
            mode: SendMode::Copy,
            conflict: ConflictPolicy::Ask,
            entry: SendToEntry::Toolbar,
        };
        let _ = s.update(Message::SendTo(req));
        // No assertion on state — that's the contract.
    }

    #[test]
    fn breadcrumbs_match_view() {
        let c = breadcrumbs_for(View::MeshOverview);
        assert_eq!(c.len(), 2);
        assert!(c[0].mesh);
        assert_eq!(c[0].label, "Mesh");
        assert_eq!(c[1].label, "Overview");

        let c = breadcrumbs_for(View::Peer("birch"));
        assert_eq!(c[1].label, "birch.mesh");

        let c = breadcrumbs_for(View::Local);
        assert_eq!(c.len(), 3);
        assert!(!c.iter().any(|x| x.mesh));
    }
}
