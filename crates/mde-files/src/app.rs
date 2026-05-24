//! Iced application — top-level State, Message, update, view.

use iced::widget::{column, container, row, scrollable};
use iced::{Background, Border, Color, Element, Length, Padding, Size, Task, Theme};

use crate::backend::{Backend, BackendSnapshot, RealBackend};
use crate::model::{Layout, View};
use crate::panels::{
    ContextMenu, ContextMenuItem, DetailsPanel, DragSession, DragTarget, OpRow, OperationDrawer,
};
use crate::prefs::Accessibility;
use crate::selection::Selection;
use crate::send_to::SendToRequest;
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
    PeerCardBrowse(String),
    PeerCardSend(String),
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
    /// AF-mesh.3 (2026-05-24) — operator clicked into a sub-
    /// directory inside `View::MeshHomeChild`. The name is the
    /// row label (without the trailing `/` the renderer adds for
    /// folders). Pushes onto the path stack so the breadcrumb
    /// + the next backend list call reflect the descent.
    MeshFolderEnter(String),
    /// AF-mesh.3 — pop back up one level inside Mesh Home. Used
    /// by the toolbar back button + the parent-link breadcrumb
    /// click.
    MeshFolderUp,
    /// AF-mesh.3 — pop back to a specific depth (used by
    /// breadcrumb mid-segment clicks). 0 = the slug root.
    MeshFolderPop(usize),
}

/// Breadcrumb segment used by the toolbar.
#[derive(Debug, Clone)]
pub struct Crumb {
    pub label: String,
    /// True if this crumb belongs to a mesh path. Affects colour + the trailing tag chip.
    pub mesh: bool,
}

pub struct MdeFiles {
    /// v4.0.1 AF-* (2026-05-23) — backend that supplies the
    /// rendered roster + file lists. Defaults to `RealBackend`
    /// in production builds (LocalFsBackend + DBusBackend
    /// composed); tests can swap a `DemoBackend` via
    /// `MdeFiles::with_backend`.
    pub backend: Box<dyn Backend>,
    /// v4.0.1 AF-* — last `BackendSnapshot` captured. Refreshed
    /// in `update()` so `view()` returns an `Element` tied to
    /// `&self`'s lifetime (Iced can't borrow from a local).
    pub snapshot: BackendSnapshot,
    /// v4.0.1 AF-* — files loaded for the currently-active peer
    /// view. Refreshed when `View::Peer` is entered so `view()`
    /// can borrow without re-querying the backend per render.
    pub peer_files: Vec<crate::model::FileRow>,
    pub view: View,
    pub local_open: bool,
    pub layout: Layout,
    pub search: String,
    /// AF-mesh.3 — path stack inside `View::MeshHomeChild(slug)`.
    /// Empty = top of the XDG dir. Each entry is a single
    /// subdirectory name. Cleared whenever the active slug
    /// changes so the stack never carries stale state across
    /// dirs.
    pub mesh_home_path: Vec<String>,
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

impl Default for MdeFiles {
    fn default() -> Self {
        let backend: Box<dyn Backend> = Box::new(RealBackend::new());
        let snapshot = BackendSnapshot::capture(&*backend);
        Self {
            backend,
            snapshot,
            peer_files: Vec::new(),
            view: View::default(),
            local_open: false,
            layout: Layout::default(),
            search: String::new(),
            mesh_home_path: Vec::new(),
            selection: Selection::default(),
            details: DetailsPanel::default(),
            context_menu: ContextMenu::default(),
            op_drawer: OperationDrawer::default(),
            drag: DragSession::default(),
            a11y: Accessibility::default(),
            keyboard_pane: KeyboardPane::default(),
            keyboard_active: false,
        }
    }
}

impl MdeFiles {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build with an injected backend (useful for unit tests +
    /// dev modes). Production code lands through `Default`.
    #[must_use]
    pub fn with_backend(backend: Box<dyn Backend>) -> Self {
        Self {
            backend,
            ..Self::default()
        }
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
                let is_local = matches!(v, View::Local);
                // AF-mesh.3 — clear the path stack whenever we
                // leave a MeshHomeChild OR switch to a different
                // slug. Entering MeshHomeChild from the parent
                // implicitly starts at the slug root.
                let drop_path = match (&self.view, &v) {
                    (View::MeshHomeChild(a), View::MeshHomeChild(b)) => a != b,
                    _ => !matches!(v, View::MeshHomeChild(_)),
                };
                if drop_path {
                    self.mesh_home_path.clear();
                }
                self.view = v;
                if !is_local {
                    self.local_open = false;
                }
                // Phase 1.3 — selection is per-view; clear on
                // navigation so stale row keys don't leak across
                // peer folders.
                self.selection.clear();
            }
            Message::MeshFolderEnter(name) => {
                if matches!(self.view, View::MeshHomeChild(_)) {
                    // Strip the trailing `/` the renderer adds
                    // for folders. Reject empty + `..` segments
                    // so the path stack stays canonical.
                    let clean = name.trim_end_matches('/').to_owned();
                    if !clean.is_empty() && clean != ".." && !clean.contains('/') {
                        self.mesh_home_path.push(clean);
                        self.selection.clear();
                    }
                }
            }
            Message::MeshFolderUp => {
                if matches!(self.view, View::MeshHomeChild(_))
                    && !self.mesh_home_path.is_empty()
                {
                    self.mesh_home_path.pop();
                    self.selection.clear();
                }
            }
            Message::MeshFolderPop(depth) => {
                if matches!(self.view, View::MeshHomeChild(_))
                    && depth < self.mesh_home_path.len()
                {
                    self.mesh_home_path.truncate(depth);
                    self.selection.clear();
                }
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
                // Refresh is the explicit reload signal. The
                // other variants are no-op routing hooks that
                // pre-date the live backend; touching them only
                // re-captures so the snapshot stays current.
            }
        }
        self.refresh_snapshot();
        Task::none()
    }

    /// Re-capture the `BackendSnapshot` + (when on a peer view)
    /// the active peer's file list. Called at the end of every
    /// `update()` so the next `view()` render sees fresh data.
    /// O(few backend calls); per-tick cost is acceptable since
    /// Iced only re-runs `update()` on Message arrival.
    fn refresh_snapshot(&mut self) {
        self.snapshot = BackendSnapshot::capture(&*self.backend);
        self.peer_files = match &self.view {
            View::Peer(id) => self.backend.list(&format!("peer:{id}")),
            View::MeshHomeChild(slug) => {
                let mut p = format!("local:{slug}");
                for seg in &self.mesh_home_path {
                    p.push('/');
                    p.push_str(seg);
                }
                self.backend.list(&p)
            }
            _ => Vec::new(),
        };
    }

    /// Top-level view tree.
    pub fn view(&self) -> Element<'_, Message> {
        let crumbs = breadcrumbs_for_with_path(&self.view, &self.mesh_home_path);
        let snap = &self.snapshot;

        let main_body: Element<'_, Message> = match &self.view {
            View::MeshOverview => views::mesh_overview(snap),
            View::Inbox => views::inbox(snap),
            View::Peer(id) => {
                if let Some(p) = snap.peers.iter().find(|p| &p.id == id) {
                    views::peer_folder(
                        p,
                        &snap.self_node,
                        self.peer_files.clone(),
                        &self.search,
                        self.layout,
                    )
                } else {
                    empty_state("no peer").into()
                }
            }
            View::Downloads => views::downloads(snap),
            View::Local => views::local_veil(snap),
            View::MeshHome => views::mesh_home(snap),
            View::MeshHomeChild(slug) => views::mesh_home_child(
                slug,
                self.peer_files.clone(),
                &self.search,
                self.layout,
                &self.mesh_home_path,
            ),
        };

        let content = container(scrollable(container(main_body).padding(Padding {
            top: 18.0,
            right: 22.0,
            bottom: 28.0,
            left: 22.0,
        })))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(t::PF_BG_300)),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        });

        let main = column![
            views::toolbar(&self.view, self.layout, &self.search, crumbs),
            content,
        ]
        .spacing(0);

        let body = row![
            views::sidebar(&self.view, self.local_open, snap),
            container(main).width(Length::Fill).height(Length::Fill),
        ]
        .height(Length::Fill);

        let online = snap
            .peers
            .iter()
            .filter(|p| matches!(p.status, crate::model::PeerStatus::Online))
            .count();
        let total = snap.peers.len();

        container(
            column![
                views::titlebar_with_status(online, total, snap.mesh_volume.as_ref()),
                body
            ]
            .spacing(0),
        )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(t::WINDOW)),
                border: Border {
                    color: Color {
                        a: 0.08,
                        ..Color::WHITE
                    },
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..container::Style::default()
            })
            .into()
    }
}

/// AF-mesh.3 — path-aware breadcrumb builder. Identical to
/// `breadcrumbs_for` except for `MeshHomeChild`, where each
/// element of `path` becomes its own crumb between the dir-name
/// crumb and the leaf.
pub fn breadcrumbs_for_with_path(view: &View, path: &[String]) -> Vec<Crumb> {
    if let View::MeshHomeChild(slug) = view {
        let mut out = vec![
            Crumb {
                label: "Mesh".into(),
                mesh: true,
            },
            Crumb {
                label: "Home".into(),
                mesh: true,
            },
            Crumb {
                label: mesh_home_label(slug).into(),
                mesh: true,
            },
        ];
        for seg in path {
            out.push(Crumb {
                label: seg.clone(),
                mesh: true,
            });
        }
        // Mark the final crumb as leaf (mesh: false) so the
        // styling matches the other leaf crumbs.
        if let Some(last) = out.last_mut() {
            last.mesh = false;
        }
        return out;
    }
    breadcrumbs_for(view)
}

fn breadcrumbs_for(view: &View) -> Vec<Crumb> {
    match view {
        View::MeshOverview => vec![
            Crumb {
                label: "Mesh".into(),
                mesh: true,
            },
            Crumb {
                label: "Overview".into(),
                mesh: false,
            },
        ],
        View::Inbox => vec![
            Crumb {
                label: "Mesh".into(),
                mesh: true,
            },
            Crumb {
                label: "Inbox".into(),
                mesh: false,
            },
        ],
        View::Peer(id) => {
            // The host string is built from the peer id by
            // convention (id "pine" → host "pine.mesh"). We
            // don't have the live peer list here; the toolbar
            // shows the host literal which the runtime can
            // patch on next render.
            let host = format!("{id}.mesh");
            vec![
                Crumb {
                    label: "Mesh".into(),
                    mesh: true,
                },
                Crumb {
                    label: host,
                    mesh: false,
                },
            ]
        }
        View::Downloads => vec![
            Crumb {
                label: "Mesh".into(),
                mesh: true,
            },
            Crumb {
                label: "Downloads".into(),
                mesh: false,
            },
        ],
        View::Local => vec![
            Crumb {
                label: "Local".into(),
                mesh: false,
            },
            Crumb {
                label: "/".into(),
                mesh: false,
            },
        ],
        View::MeshHome => vec![
            Crumb {
                label: "Mesh".into(),
                mesh: true,
            },
            Crumb {
                label: "Home".into(),
                mesh: false,
            },
        ],
        View::MeshHomeChild(slug) => vec![
            Crumb {
                label: "Mesh".into(),
                mesh: true,
            },
            Crumb {
                label: "Home".into(),
                mesh: true,
            },
            Crumb {
                label: mesh_home_label(slug).into(),
                mesh: false,
            },
        ],
    }
}

/// Human-readable label for a mesh-home XDG-dir slug.
pub fn mesh_home_label(slug: &str) -> &'static str {
    match slug {
        "docs" => "Documents",
        "pics" => "Pictures",
        "music" => "Music",
        "videos" => "Videos",
        "downloads" => "Downloads",
        _ => "Files",
    }
}

fn empty_state(label: &str) -> Element<'static, Message> {
    container(
        iced::widget::text(label.to_string())
            .size(12)
            .color(t::FG_FAINT),
    )
    .padding(Padding::new(56.0))
    .width(Length::Fill)
    .style(|_| container::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border {
            color: Color {
                a: 0.10,
                ..Color::WHITE
            },
            width: 1.0,
            radius: 0.0.into(),
        },
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
        assert!(
            !s.local_open,
            "local disclosure must close when leaving Local view"
        );
    }

    #[test]
    fn peer_card_browse_routes_to_peer_view() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::PeerCardBrowse("pine".into()));
        assert_eq!(s.view, View::Peer("pine".into()));
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
        let _ = s.update(Message::PeerCardBrowse("pine".into()));
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
        assert_eq!(
            s.drag.hover_target(),
            Some(&DragTarget::Peer("pine".into()))
        );
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
        let c = breadcrumbs_for(&View::MeshOverview);
        assert_eq!(c.len(), 2);
        assert!(c[0].mesh);
        assert_eq!(c[0].label, "Mesh");
        assert_eq!(c[1].label, "Overview");

        let c = breadcrumbs_for(&View::Peer("birch".into()));
        assert_eq!(c[1].label, "birch.mesh");

        let c = breadcrumbs_for(&View::Local);
        assert_eq!(c.len(), 2);
        assert!(!c.iter().any(|x| x.mesh));
    }

    #[test]
    fn selecting_mesh_home_clears_selection() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::RowClick("x".into()));
        let _ = s.update(Message::SelectView(View::MeshHome));
        assert_eq!(s.view, View::MeshHome);
        assert!(s.selection.is_empty(), "mesh-home should clear selection");
    }

    #[test]
    fn mesh_home_child_refreshes_local_listing() {
        let mut s = MdeFiles::default();
        // Route into a child; refresh_snapshot should query the
        // backend for `local:<slug>`. With the default RealBackend
        // pointing at $HOME, the call returns whatever's on disk
        // (or an empty Vec). The contract under test is just that
        // refresh_snapshot doesn't panic + the view variant is
        // accepted.
        let _ = s.update(Message::SelectView(View::MeshHomeChild("docs".into())));
        assert_eq!(s.view, View::MeshHomeChild("docs".into()));
    }

    #[test]
    fn mesh_home_label_covers_xdg_slugs() {
        assert_eq!(mesh_home_label("docs"), "Documents");
        assert_eq!(mesh_home_label("pics"), "Pictures");
        assert_eq!(mesh_home_label("music"), "Music");
        assert_eq!(mesh_home_label("videos"), "Videos");
        assert_eq!(mesh_home_label("downloads"), "Downloads");
        assert_eq!(mesh_home_label("unknown"), "Files");
    }

    #[test]
    fn breadcrumbs_for_mesh_home_marks_mesh_segments() {
        let c = breadcrumbs_for(&View::MeshHome);
        assert_eq!(c.len(), 2);
        assert!(c[0].mesh);
        assert_eq!(c[0].label, "Mesh");
        assert_eq!(c[1].label, "Home");

        let c = breadcrumbs_for(&View::MeshHomeChild("docs".into()));
        assert_eq!(c.len(), 3);
        assert_eq!(c[2].label, "Documents");
    }

    #[test]
    fn mesh_folder_enter_pushes_onto_path_stack() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::SelectView(View::MeshHomeChild("docs".into())));
        let _ = s.update(Message::MeshFolderEnter("Projects/".into()));
        assert_eq!(s.mesh_home_path, vec!["Projects".to_string()]);
        let _ = s.update(Message::MeshFolderEnter("MDE".into()));
        assert_eq!(
            s.mesh_home_path,
            vec!["Projects".to_string(), "MDE".to_string()]
        );
    }

    #[test]
    fn mesh_folder_enter_outside_mesh_home_child_is_noop() {
        let mut s = MdeFiles::default();
        // Default view is MeshOverview, not MeshHomeChild.
        let _ = s.update(Message::MeshFolderEnter("anywhere".into()));
        assert!(s.mesh_home_path.is_empty());
    }

    #[test]
    fn mesh_folder_enter_rejects_path_separators() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::SelectView(View::MeshHomeChild("docs".into())));
        // Reject anything that smells like an escape attempt.
        let _ = s.update(Message::MeshFolderEnter("..".into()));
        let _ = s.update(Message::MeshFolderEnter("a/b".into()));
        let _ = s.update(Message::MeshFolderEnter("".into()));
        assert!(s.mesh_home_path.is_empty());
    }

    #[test]
    fn mesh_folder_up_pops_one_level() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::SelectView(View::MeshHomeChild("docs".into())));
        let _ = s.update(Message::MeshFolderEnter("a".into()));
        let _ = s.update(Message::MeshFolderEnter("b".into()));
        let _ = s.update(Message::MeshFolderUp);
        assert_eq!(s.mesh_home_path, vec!["a".to_string()]);
    }

    #[test]
    fn mesh_folder_up_at_root_is_noop() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::SelectView(View::MeshHomeChild("docs".into())));
        let _ = s.update(Message::MeshFolderUp);
        assert!(s.mesh_home_path.is_empty());
    }

    #[test]
    fn mesh_folder_pop_truncates_to_depth() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::SelectView(View::MeshHomeChild("docs".into())));
        let _ = s.update(Message::MeshFolderEnter("a".into()));
        let _ = s.update(Message::MeshFolderEnter("b".into()));
        let _ = s.update(Message::MeshFolderEnter("c".into()));
        let _ = s.update(Message::MeshFolderPop(1));
        assert_eq!(s.mesh_home_path, vec!["a".to_string()]);
    }

    #[test]
    fn changing_slug_clears_path_stack() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::SelectView(View::MeshHomeChild("docs".into())));
        let _ = s.update(Message::MeshFolderEnter("a".into()));
        let _ = s.update(Message::SelectView(View::MeshHomeChild("pics".into())));
        assert!(
            s.mesh_home_path.is_empty(),
            "path must reset when slug changes"
        );
    }

    #[test]
    fn leaving_mesh_home_child_clears_path_stack() {
        let mut s = MdeFiles::default();
        let _ = s.update(Message::SelectView(View::MeshHomeChild("docs".into())));
        let _ = s.update(Message::MeshFolderEnter("a".into()));
        let _ = s.update(Message::SelectView(View::MeshOverview));
        assert!(s.mesh_home_path.is_empty());
    }

    #[test]
    fn breadcrumbs_with_path_lists_each_segment() {
        let path = vec!["Projects".to_string(), "MDE".to_string()];
        let c = breadcrumbs_for_with_path(&View::MeshHomeChild("docs".into()), &path);
        assert_eq!(c.len(), 5);
        assert_eq!(c[0].label, "Mesh");
        assert_eq!(c[1].label, "Home");
        assert_eq!(c[2].label, "Documents");
        assert_eq!(c[3].label, "Projects");
        assert_eq!(c[4].label, "MDE");
        // The leaf crumb is rendered without the mesh tint.
        assert!(!c[4].mesh);
    }
}
