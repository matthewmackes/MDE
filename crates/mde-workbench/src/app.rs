//! Top-level Iced application — state, message reducer, view.
//!
//! CB-1.1 + CB-1.2 scaffold: nine-group sidebar + breadcrumb +
//! page title / subtitle. Per-panel views (CB-1.3 ... CB-1.10)
//! land as separate substeps and plug into [`App::view`] via
//! [`crate::View::Panel`] matching.

use std::time::Duration;

use iced::widget::{column, container, row, text};
use iced::{Element, Length, Size, Subscription, Task, Theme};

use crate::dbus::PendingFocus;
use crate::keyboard::{KeyAction, Pane};
use crate::model::{Group, View, view_from_focus_slug};
use crate::patternfly::{breadcrumb, page_subtitle, page_title};
use crate::sidebar::SidebarState;

/// Default window size — matches the v1.x GTK3 sidebar window
/// (`SidebarWindow` defaults).
pub const WIN_W: f32 = 1180.0;
pub const WIN_H: f32 = 760.0;

/// Reducer messages — every interaction lands here.
#[derive(Debug, Clone)]
pub enum Message {
    /// Sidebar click on a top-level group row.
    SelectGroup(Group),
    /// Sidebar click on a leaf panel row.
    SelectPanel { group: Group, panel: &'static str },
    /// Keyboard / chord-bar generated key. Translated by
    /// [`crate::keyboard::interpret_key`] before landing here.
    KeyPressed(KeyAction),
    /// User toggled the user-expansion state of a group
    /// (chevron click). Active group ignores this per CB-1.2.
    ToggleGroupExpansion(Group),
    /// CB-1.13 — a `dev.mackes.MDE.Shell.Workbench.Focus(slug)`
    /// call landed in [`PendingFocus`] and the polling
    /// subscription pulled it out. Empty slug means "raise
    /// only — don't change the view" (the 1.x taskbar
    /// click-through contract).
    FocusRequest(String),
    /// No-op — placeholder for buttons whose behaviour lands in
    /// later CB-1.x substeps.
    Noop,
}

/// Workbench application state.
#[derive(Debug, Clone)]
pub struct App {
    view: View,
    sidebar: SidebarState,
    focused_pane: Pane,
}

impl Default for App {
    fn default() -> Self {
        Self {
            view: View::default(),
            sidebar: SidebarState::new(),
            focused_pane: Pane::Sidebar,
        }
    }
}

impl App {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build an [`App`] pre-focused on a deep-link slug
    /// (e.g. `mde --focus network.mesh_ssh`). Falls back to
    /// the default Dashboard view when the slug is unknown.
    #[must_use]
    pub fn with_focus(focus_slug: &str) -> Self {
        let mut app = Self::default();
        if let Some(view) = view_from_focus_slug(focus_slug) {
            app.view = view;
            app.focused_pane = Pane::Main;
        }
        app
    }

    #[must_use]
    pub fn current_view(&self) -> View {
        self.view
    }

    #[must_use]
    pub fn focused_pane(&self) -> Pane {
        self.focused_pane
    }

    /// Run the Iced application.
    pub fn run() -> iced::Result {
        iced::application(Self::title, Self::update, Self::view)
            .theme(Self::theme)
            .subscription(Self::subscription)
            .window_size(Size::new(WIN_W, WIN_H))
            .run()
    }

    /// Iced subscription bundle — polls [`PendingFocus`] on a
    /// 200 ms tick so any `dev.mackes.MDE.Shell.Workbench.Focus`
    /// D-Bus call from a sibling `mde-workbench --focus <slug>`
    /// invocation propagates into the reducer as
    /// [`Message::FocusRequest`].
    #[allow(clippy::unused_self)]
    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(200)).map(|_| {
            PendingFocus::drain()
                .map_or(Message::Noop, Message::FocusRequest)
        })
    }

    fn title(&self) -> String {
        format!("MDE Workbench — {}", page_title(self.view))
    }

    #[allow(clippy::unused_self)]
    fn theme(&self) -> Theme {
        // CB-1.2 surface ships with the Iced default Dark theme;
        // the cosmic-theme adapter (E3.1) hooks in once Phase
        // E.1.3 wires libcosmic into the panel + workbench.
        Theme::Dark
    }

    /// Apply a [`Message`] to the state. Returns [`Task::none`]
    /// for now — once `Backend::DBus` (CB-1.x panel ports)
    /// lands, several variants will return real async tasks.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectGroup(group) => {
                self.view = View::Group(group);
                self.focused_pane = Pane::Main;
            }
            Message::SelectPanel { group, panel } => {
                self.view = View::Panel { group, panel };
                self.focused_pane = Pane::Main;
            }
            Message::ToggleGroupExpansion(group) => {
                self.sidebar.toggle(group, self.view.group());
            }
            Message::KeyPressed(action) => {
                self.apply_key_action(action);
            }
            Message::FocusRequest(slug) => {
                self.apply_focus_request(&slug);
            }
            Message::Noop => {}
        }
        Task::none()
    }

    fn apply_focus_request(&mut self, slug: &str) {
        if slug.is_empty() {
            // Empty slug = "raise only, no view change" — the
            // 1.x taskbar click-through behaviour.
            return;
        }
        if let Some(view) = view_from_focus_slug(slug) {
            self.view = view;
            self.focused_pane = Pane::Main;
        }
        // Unknown slug silently ignored — matches the 1.x
        // `mackes --focus` Dashboard fallback for unmapped
        // surfaces (here we keep the current view since
        // jumping back to Dashboard on a typo would
        // surprise the user mid-task).
    }

    fn apply_key_action(&mut self, action: KeyAction) {
        match action {
            KeyAction::FocusPane(pane) => {
                self.focused_pane = pane;
            }
            KeyAction::JumpToGroup(group) => {
                self.view = View::Group(group);
                self.focused_pane = Pane::Sidebar;
            }
            KeyAction::CloseDetail => {
                if let View::Panel { group, .. } = self.view {
                    self.view = View::Group(group);
                    self.focused_pane = Pane::Sidebar;
                }
            }
            KeyAction::Ignored => {}
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let sidebar = crate::sidebar::view(
            &self.sidebar,
            self.view,
            Message::SelectGroup,
            |group, panel| Message::SelectPanel { group, panel },
        );

        let crumbs = breadcrumb(self.view)
            .into_iter()
            .map(|c| c.label)
            .collect::<Vec<_>>()
            .join(" / ");

        let main = column![
            text(crumbs).size(12),
            text(page_title(self.view)).size(26),
            text(page_subtitle(self.view)).size(13),
        ]
        .spacing(6)
        .padding(24);

        let layout = row![
            sidebar,
            container(main).width(Length::Fill).height(Length::Fill)
        ]
        .height(Length::Fill);

        container(layout).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_app_lands_on_dashboard_view() {
        let app = App::new();
        assert_eq!(app.current_view(), View::Group(Group::Dashboard));
        assert_eq!(app.focused_pane(), Pane::Sidebar);
    }

    #[test]
    fn select_group_updates_view_and_focuses_main_pane() {
        let mut app = App::new();
        let _ = app.update(Message::SelectGroup(Group::Network));
        assert_eq!(app.current_view(), View::Group(Group::Network));
        assert_eq!(app.focused_pane(), Pane::Main);
    }

    #[test]
    fn select_panel_carries_group_and_panel_slug() {
        let mut app = App::new();
        let _ = app.update(Message::SelectPanel {
            group: Group::Network,
            panel: "mesh_ssh",
        });
        assert_eq!(
            app.current_view(),
            View::Panel {
                group: Group::Network,
                panel: "mesh_ssh"
            }
        );
    }

    #[test]
    fn ctrl_digit_key_action_jumps_to_group_and_refocuses_sidebar() {
        let mut app = App::new();
        let _ = app.update(Message::KeyPressed(KeyAction::JumpToGroup(Group::Help)));
        assert_eq!(app.current_view(), View::Group(Group::Help));
        assert_eq!(app.focused_pane(), Pane::Sidebar);
    }

    #[test]
    fn escape_from_panel_view_returns_to_parent_group_landing() {
        let mut app = App::new();
        let _ = app.update(Message::SelectPanel {
            group: Group::Maintain,
            panel: "snapshots",
        });
        let _ = app.update(Message::KeyPressed(KeyAction::CloseDetail));
        assert_eq!(app.current_view(), View::Group(Group::Maintain));
        assert_eq!(app.focused_pane(), Pane::Sidebar);
    }

    #[test]
    fn escape_from_group_view_is_noop() {
        let mut app = App::new();
        let _ = app.update(Message::SelectGroup(Group::System));
        let _ = app.update(Message::KeyPressed(KeyAction::CloseDetail));
        // Still on the same group landing — no leaf to close.
        assert_eq!(app.current_view(), View::Group(Group::System));
    }

    #[test]
    fn tab_focus_pane_action_updates_focused_pane() {
        let mut app = App::new();
        let _ = app.update(Message::KeyPressed(KeyAction::FocusPane(Pane::Main)));
        assert_eq!(app.focused_pane(), Pane::Main);
    }

    #[test]
    fn toggle_group_expansion_flips_state() {
        let mut app = App::new();
        // Inactive group starts collapsed.
        assert!(!app.sidebar.is_expanded(Group::Network, Group::Dashboard));
        let _ = app.update(Message::ToggleGroupExpansion(Group::Network));
        assert!(app.sidebar.is_expanded(Group::Network, Group::Dashboard));
    }

    #[test]
    fn with_focus_lands_on_named_panel_and_focuses_main() {
        let app = App::with_focus("network.mesh_ssh");
        assert_eq!(
            app.current_view(),
            View::Panel {
                group: Group::Network,
                panel: "mesh_ssh"
            }
        );
        assert_eq!(app.focused_pane(), Pane::Main);
    }

    #[test]
    fn with_focus_falls_back_to_dashboard_on_unknown_slug() {
        let app = App::with_focus("not-a-real-slug");
        assert_eq!(app.current_view(), View::Group(Group::Dashboard));
    }

    #[test]
    fn noop_message_does_not_change_state() {
        let mut app = App::new();
        let before_view = app.current_view();
        let before_pane = app.focused_pane();
        let _ = app.update(Message::Noop);
        assert_eq!(app.current_view(), before_view);
        assert_eq!(app.focused_pane(), before_pane);
    }

    #[test]
    fn focus_request_with_panel_slug_jumps_to_panel_and_focuses_main() {
        let mut app = App::new();
        let _ = app.update(Message::FocusRequest("network.mesh_ssh".into()));
        assert_eq!(
            app.current_view(),
            View::Panel {
                group: Group::Network,
                panel: "mesh_ssh"
            }
        );
        assert_eq!(app.focused_pane(), Pane::Main);
    }

    #[test]
    fn focus_request_with_group_slug_lands_on_group_view() {
        let mut app = App::new();
        let _ = app.update(Message::FocusRequest("help".into()));
        assert_eq!(app.current_view(), View::Group(Group::Help));
    }

    #[test]
    fn focus_request_empty_slug_preserves_view() {
        let mut app = App::new();
        let _ = app.update(Message::SelectPanel {
            group: Group::Apps,
            panel: "sources",
        });
        let before = app.current_view();
        let _ = app.update(Message::FocusRequest(String::new()));
        assert_eq!(
            app.current_view(),
            before,
            "empty slug = raise-only contract — view must not change"
        );
    }

    #[test]
    fn focus_request_unknown_slug_preserves_view() {
        let mut app = App::new();
        let _ = app.update(Message::SelectGroup(Group::Maintain));
        let before = app.current_view();
        let _ = app.update(Message::FocusRequest("not-a-real-slug".into()));
        assert_eq!(
            app.current_view(),
            before,
            "unknown slug must not jolt the user out of their current view"
        );
    }

    #[test]
    fn title_includes_active_page() {
        let mut app = App::new();
        let _ = app.update(Message::SelectGroup(Group::Apps));
        assert!(app.title().contains("Apps"));
        assert!(app.title().starts_with("MDE Workbench"));
    }
}
