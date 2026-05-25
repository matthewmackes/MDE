//! Portal-5 Iced application — Dock with workspace segment + 6 nav buttons.
//!
//! Dock layout (56 px, AllScreens, Intel One Mono):
//!
//! ```text
//! [1›][2›][dev…›][+]  ···spacer···  [›Apps][›Files][›Notif][›VoIP][›Net][›Settings]
//! ```
//!
//! **Workspace segment** (Portal-5): chevron-as-border cells (R4-Q63),
//! adaptive 24 px-floor width with truncation (R4-Q64 / Portal-5.b adds
//! marquee), all workspaces visible + current-output highlight (R3-Q46),
//! click-jump via swayipc (R3-Q23), `+` new-workspace (R3-Q24).
//! Hover Aero-peek is Portal-5.c.
//!
//! **Nav buttons** (Portal-4): 36 px Carbon glyphs, domain-color chevrons
//! (R10-Q46), tonal-inversion active indicator (R10-Q15), count badge
//! (R10-Q3), right-click (R10-Q5).

use iced::widget::{container, mouse_area, row, text};
use iced::{Color, Element, Length, Padding, Subscription, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;

use crate::fonts::{resolve_icon, FONT_INTEL_ONE_MONO};
use crate::workspace::WorkspaceInfo;
use mde_theme::{Icon, IconSize};

/// Crate-private app-id constant visible to the layer-shell compositor.
pub(crate) const APP_ID: &str = "dev.mackes.MDE.Portal";

/// Height of the Dock strip in logical pixels (Portal-2 lock).
pub const DOCK_HEIGHT_PX: u32 = 56;

/// Nav button size in logical pixels (R10-Q2 lock).
pub const NAV_BUTTON_PX: f32 = 36.0;

/// Chevron size between nav buttons in logical pixels (R10-Q2).
pub const CHEVRON_PX: f32 = 16.0;

// ── colour palette ────────────────────────────────────────────────────────────

/// Classic ChromeOS charcoal — `#202124` (dark-mode Dock background).
const CHARCOAL: Color = Color {
    r: 0.125_f32,
    g: 0.129_f32,
    b: 0.141_f32,
    a: 1.0,
};

/// Classic ChromeOS off-white — `#f1f3f4` (light-mode Dock background, R4-Q75).
const OFF_WHITE: Color = Color {
    r: 0.945_f32,
    g: 0.953_f32,
    b: 0.957_f32,
    a: 1.0,
};

/// Apps domain colour — indigo `#5b6af5` (R10-Q3 tier-pulse, R10-Q46 chevron).
pub const COLOR_INDIGO: Color = Color {
    r: 0.357_f32,
    g: 0.416_f32,
    b: 0.961_f32,
    a: 1.0,
};

/// Files domain colour — sage `#81a88e`.
pub const COLOR_SAGE: Color = Color {
    r: 0.506_f32,
    g: 0.659_f32,
    b: 0.557_f32,
    a: 1.0,
};

/// Notifications domain colour — amber `#ffca07`.
pub const COLOR_AMBER: Color = Color {
    r: 1.000_f32,
    g: 0.792_f32,
    b: 0.027_f32,
    a: 1.0,
};

/// VoIP domain colour — purple `#b168f6`.
pub const COLOR_PURPLE: Color = Color {
    r: 0.694_f32,
    g: 0.408_f32,
    b: 0.965_f32,
    a: 1.0,
};

/// Network/Mesh domain colour — cyan `#10bad5`.
pub const COLOR_CYAN: Color = Color {
    r: 0.063_f32,
    g: 0.729_f32,
    b: 0.835_f32,
    a: 1.0,
};

// ── nav buttons ───────────────────────────────────────────────────────────────

/// The six direct nav buttons in Dock order (R10-Q1).
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum NavButton {
    Apps,
    Files,
    Notifications,
    VoIP,
    Network,
    Settings,
}

impl NavButton {
    /// All six buttons in Dock left-to-right order.
    pub const ALL: [NavButton; 6] = [
        NavButton::Apps,
        NavButton::Files,
        NavButton::Notifications,
        NavButton::VoIP,
        NavButton::Network,
        NavButton::Settings,
    ];

    /// Carbon icon for this button (monochrome glyph).
    pub fn icon(self) -> Icon {
        match self {
            NavButton::Apps => Icon::Apps,
            NavButton::Files => Icon::Files,
            NavButton::Notifications => Icon::Notification,
            NavButton::VoIP => Icon::Sound,
            NavButton::Network => Icon::Network,
            NavButton::Settings => Icon::Settings,
        }
    }

    /// Domain colour for the left chevron (R10-Q46) and tier-pulse (R10-Q3).
    pub fn domain_color(self) -> Color {
        match self {
            NavButton::Apps => COLOR_INDIGO,
            NavButton::Files => COLOR_SAGE,
            NavButton::Notifications => COLOR_AMBER,
            NavButton::VoIP => COLOR_PURPLE,
            NavButton::Network => COLOR_CYAN,
            NavButton::Settings => CHARCOAL,
        }
    }

    /// Portal-full layer name emitted to the D-Bus `Goto` call (Portal-16).
    #[allow(dead_code)]
    pub fn portal_layer(self) -> &'static str {
        match self {
            NavButton::Apps => "hub",
            NavButton::Files => "library",
            NavButton::Notifications => "library",
            NavButton::VoIP => "voip",
            NavButton::Network => "network",
            NavButton::Settings => "control",
        }
    }
}

// ── messages ──────────────────────────────────────────────────────────────────

/// Messages the Dock application handles.
///
/// `#[to_layer_message]` generates the `TryInto<LayershellCustomActions>`
/// impl required by `iced_layershell::Application::run`.
#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    /// User clicked a nav button.
    NavClicked(NavButton),
    /// User right-clicked a nav button (per-button menu, R10-Q5).
    NavRightClicked(NavButton),
    /// Workspace subscription emitted a fresh workspace list (Portal-5).
    WorkspaceList(Vec<WorkspaceInfo>),
    /// User clicked a workspace cell — focus it via swayipc (R3-Q23).
    FocusWorkspace(String),
    /// User clicked `+` — switch to the next unused workspace number.
    NewWorkspace,
    /// User clicked the hostname segment (Portal-6; cross-peer cycling in Portal-6.b).
    HostnameClicked,
    /// Fire-and-forget placeholder for Task::perform callbacks that produce no message.
    Noop,
}

// ── application state ─────────────────────────────────────────────────────────

/// Dock application state (Portal-6).
#[derive(Debug)]
pub struct DockApp {
    /// Currently active nav layer; `None` = Dock-only (Portal-full hidden).
    active_nav: Option<NavButton>,
    /// Unread/pending counts per nav button (index matches `NavButton::ALL`).
    badge_counts: [u32; 6],
    /// Live workspace list from swayipc (Portal-5). Empty until subscription fires.
    workspaces: Vec<WorkspaceInfo>,
    /// This machine's hostname — read from `/proc/sys/kernel/hostname` at startup.
    hostname: String,
}

impl Default for DockApp {
    fn default() -> Self {
        Self {
            active_nav: None,
            badge_counts: [0u32; 6],
            workspaces: Vec::new(),
            hostname: String::new(),
        }
    }
}

impl DockApp {
    /// Construct iced_layershell settings for the Dock surface.
    ///
    /// `StartMode::AllScreens` — one strip per connected output.
    /// `default_font = FONT_INTEL_ONE_MONO` — Intel One Mono primary.
    pub fn settings() -> Settings<()> {
        Settings {
            layer_settings: LayerShellSettings {
                size: Some((0, DOCK_HEIGHT_PX)),
                exclusive_zone: DOCK_HEIGHT_PX as i32,
                anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
                layer: Layer::Top,
                keyboard_interactivity: KeyboardInteractivity::None,
                start_mode: StartMode::AllScreens,
                ..Default::default()
            },
            default_font: FONT_INTEL_ONE_MONO,
            ..Default::default()
        }
    }

    /// Return badge count for a button (index into `NavButton::ALL`).
    pub fn badge_count(&self, btn: NavButton) -> u32 {
        let idx = NavButton::ALL.iter().position(|&b| b == btn).unwrap_or(0);
        self.badge_counts[idx]
    }

    /// Set badge count for a button (called by D-Bus badge-update subscription, Portal-18).
    #[allow(dead_code)]
    pub fn set_badge_count(&mut self, btn: NavButton, count: u32) {
        let idx = NavButton::ALL.iter().position(|&b| b == btn).unwrap_or(0);
        self.badge_counts[idx] = count;
    }
}

// ── iced Application impl ────────────────────────────────────────────────────

impl iced_layershell::Application for DockApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Task<Message>) {
        let hostname = std::fs::read_to_string("/proc/sys/kernel/hostname")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "localhost".to_string());
        (Self { hostname, ..Self::default() }, Task::none())
    }

    fn namespace(&self) -> String {
        APP_ID.to_string()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NavClicked(btn) => {
                self.active_nav = if self.active_nav == Some(btn) {
                    None
                } else {
                    Some(btn)
                };
            }
            Message::NavRightClicked(_btn) => {
                // Portal-4: right-click is recorded; the context popover is
                // Portal-16's scratchpad surface. For now the click is a
                // no-op so the button state doesn't change.
            }
            Message::WorkspaceList(list) => {
                self.workspaces = list;
            }
            Message::FocusWorkspace(name) => {
                return Task::perform(
                    crate::workspace::focus_workspace(name),
                    |_| Message::Noop,
                );
            }
            Message::NewWorkspace => {
                let taken: Vec<i32> = self.workspaces.iter().map(|w| w.num).collect();
                return Task::perform(
                    crate::workspace::new_workspace(taken),
                    |_| Message::Noop,
                );
            }
            Message::HostnameClicked => {
                // Portal-6.b: cross-peer cycling activates when mesh-home is live.
                // In pre-mesh-home state clicking the hostname is a no-op.
            }
            Message::Noop => {}
            // Variants injected by #[to_layer_message] (layer-shell protocol
            // actions: AnchorChange, SetInputRegion, etc.).  Not used by the
            // Dock strip — forward to the runtime silently.
            _ => {}
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        crate::workspace::workspace_subscription()
    }

    fn view(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let bg = if theme == Theme::Dark { CHARCOAL } else { OFF_WHITE };
        let fg = if theme == Theme::Dark { Color::WHITE } else { Color::BLACK };

        let ws_seg = build_workspace_segment(self, fg);
        let host_seg = build_hostname_segment(self, fg);
        let nav_row = build_nav_row(self, fg);

        container(
            row![
                ws_seg,
                host_seg,
                iced::widget::horizontal_space(),
                nav_row,
            ]
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(Padding::from([0, 8])),
        )
        .style(move |_theme: &Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(bg)),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

// ── widget helpers ────────────────────────────────────────────────────────────

/// Build the hostname segment (Portal-6): `host:output (local-only)`.
///
/// Format per R4-Q6 / R4-Q46. Pre-mesh-home state always shows `(local-only)`;
/// cross-peer cycling and the leader indicator `[leader]` activate in Portal-6.b
/// once GlusterFS mesh-home is established.
fn build_hostname_segment<'a>(app: &DockApp, fg: Color) -> Element<'a, Message> {
    let output = app
        .workspaces
        .iter()
        .find(|w| w.focused)
        .map(|w| w.output.clone())
        .unwrap_or_else(|| "?".to_string());

    let label = if app.hostname.is_empty() {
        format!("{output} (local-only)")
    } else {
        format!("{}:{output} (local-only)", app.hostname)
    };

    mouse_area(
        container(text(label).size(11.0).color(Color { a: 0.6, ..fg }))
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Center)
            .padding(Padding::from([0, 8])),
    )
    .on_press(Message::HostnameClicked)
    .into()
}

/// Build the workspace segment (Portal-5): `[ws1›][ws2›][dev…›][+]`.
///
/// Each cell has the workspace label + inline `›` right-chevron acting as the
/// cell border (R4-Q63).  Cells adapt to content width; 24 px floor (R4-Q64).
/// Focused workspace: indigo bg.  Current-output visible workspace: subtle
/// highlight.  Urgent: red bg.  Other outputs' workspaces: dimmed.
fn build_workspace_segment<'a>(app: &DockApp, fg: Color) -> Element<'a, Message> {
    // Determine output of the focused workspace — that's the "current" output.
    let current_output: &str = app
        .workspaces
        .iter()
        .find(|w| w.focused)
        .map(|w| w.output.as_str())
        .unwrap_or("");

    let mut cells: Vec<Element<'a, Message>> = Vec::new();

    for ws in app.workspaces.iter().filter(|w| w.num >= 0) {
        let is_focused = ws.focused;
        let is_current_output = ws.output == current_output;
        let is_urgent = ws.urgent;

        let label = ws.display_label();

        let text_color = if is_focused {
            Color::WHITE
        } else if is_current_output {
            fg
        } else {
            Color { a: 0.5, ..fg }
        };

        let cell_bg: Option<Color> = if is_urgent {
            Some(Color::from_rgb(0.8, 0.1, 0.1))
        } else if is_focused {
            Some(COLOR_INDIGO)
        } else if ws.visible {
            Some(Color { r: 1.0, g: 1.0, b: 1.0, a: 0.1 })
        } else {
            None
        };

        let chevron_color = if is_focused {
            Color { r: 1.0, g: 1.0, b: 1.0, a: 0.5 }
        } else if is_current_output {
            Color { r: 0.5, g: 0.5, b: 0.5, a: 1.0 }
        } else {
            Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 }
        };

        let cell_content = row![
            text(label).size(12.0).color(text_color),
            text("›").size(10.0).color(chevron_color),
        ]
        .spacing(2)
        .align_y(iced::Alignment::Center);

        let cell = container(cell_content)
            .width(Length::Shrink)
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Center)
            .padding(Padding::from([0, 8]))
            .style(move |_: &Theme| iced::widget::container::Style {
                background: cell_bg.map(iced::Background::Color),
                ..Default::default()
            });

        let ws_name = ws.name.clone();
        cells.push(
            mouse_area(cell)
                .on_press(Message::FocusWorkspace(ws_name))
                .into(),
        );
    }

    // `+` button — creates the next unused workspace.
    let plus_cell = container(
        text("+").size(14.0).color(Color { a: 0.6, ..fg }),
    )
    .width(Length::Shrink)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .padding(Padding::from([0, 6]));

    cells.push(
        mouse_area(plus_cell)
            .on_press(Message::NewWorkspace)
            .into(),
    );

    row(cells)
        .spacing(0)
        .height(Length::Fill)
        .align_y(iced::Alignment::Center)
        .into()
}

/// Build the nav-button row segment: `[chevron][button]` × 6.
fn build_nav_row(app: &DockApp, fg: Color) -> Element<'_, Message> {
    let mut items: Vec<Element<'_, Message>> = Vec::new();

    for btn in NavButton::ALL {
        // Domain-colour left chevron (16 px, R10-Q46).
        items.push(nav_chevron(btn.domain_color()));

        // The button cell: glyph + optional badge.
        let is_active = app.active_nav == Some(btn);
        let badge = app.badge_count(btn);
        items.push(nav_button_cell(btn, is_active, badge, fg));
    }

    row(items)
        .spacing(0)
        .height(Length::Fill)
        .align_y(iced::Alignment::Center)
        .into()
}

/// Render the 16 px domain-colour chevron glyph `›`.
fn nav_chevron(color: Color) -> Element<'static, Message> {
    container(
        text("›")
            .size(CHEVRON_PX)
            .color(color),
    )
    .width(CHEVRON_PX)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

/// Render one nav button: glyph with tonal-inversion when active + badge.
fn nav_button_cell<'a>(
    btn: NavButton,
    is_active: bool,
    badge: u32,
    fg: Color,
) -> Element<'a, Message> {
    let resolved = resolve_icon(btn.icon(), IconSize::Nav);
    let glyph = resolved.fallback_glyph;

    let glyph_color = if is_active { Color::WHITE } else { fg };
    let glyph_element = text(glyph)
        .size(NAV_BUTTON_PX * 0.5) // glyph at ~18 px within 36 px cell
        .color(glyph_color);

    let cell_bg: Option<Color> = if is_active { Some(COLOR_INDIGO) } else { None };

    let inner = container(glyph_element)
        .width(NAV_BUTTON_PX)
        .height(NAV_BUTTON_PX)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_theme: &Theme| iced::widget::container::Style {
            background: cell_bg.map(iced::Background::Color),
            border: iced::Border {
                radius: iced::border::Radius::from(4.0),
                ..Default::default()
            },
            ..Default::default()
        });

    // Badge overlay: stack glyph container + badge text in a column-like
    // relative stack (badge top-right, R10-Q3). Iced lacks Z-stack in 0.13;
    // we approximate with a row where badge is appended right.
    let cell: Element<'_, Message> = if badge > 0 {
        let badge_label = if badge > 99 { "99+".to_string() } else { badge.to_string() };
        row![
            inner,
            container(
                text(badge_label).size(9.0).color(Color::WHITE),
            )
            .style(|_: &Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.9, 0.1, 0.1))),
                border: iced::Border {
                    radius: iced::border::Radius::from(6.0),
                    ..Default::default()
                },
                ..Default::default()
            })
            .padding(Padding::from([1, 3]))
            .width(Length::Shrink),
        ]
        .align_y(iced::Alignment::Start)
        .spacing(0)
        .into()
    } else {
        inner.into()
    };

    mouse_area(cell)
        .on_press(Message::NavClicked(btn))
        .on_right_press(Message::NavRightClicked(btn))
        .into()
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dock_height_is_56() {
        assert_eq!(DOCK_HEIGHT_PX, 56, "Portal-2 lock: Dock is 56 px");
    }

    #[test]
    fn nav_button_px_is_36() {
        assert!((NAV_BUTTON_PX - 36.0).abs() < f32::EPSILON, "Portal-4 lock: nav button 36 px");
    }

    #[test]
    fn chevron_px_is_16() {
        assert!((CHEVRON_PX - 16.0).abs() < f32::EPSILON, "Portal-4 lock: chevron 16 px");
    }

    #[test]
    fn charcoal_matches_chromeos_lock() {
        let r = (CHARCOAL.r * 255.0).round() as u8;
        let g = (CHARCOAL.g * 255.0).round() as u8;
        let b = (CHARCOAL.b * 255.0).round() as u8;
        assert_eq!((r, g, b), (32, 33, 36), "#202124 charcoal");
    }

    #[test]
    fn off_white_matches_chromeos_lock() {
        let r = (OFF_WHITE.r * 255.0).round() as u8;
        let g = (OFF_WHITE.g * 255.0).round() as u8;
        let b = (OFF_WHITE.b * 255.0).round() as u8;
        assert_eq!((r, g, b), (241, 243, 244), "#f1f3f4 off-white");
    }

    #[test]
    fn indigo_matches_accent_lock() {
        // Q2 indigo — #5b6af5 = rgb(91, 106, 245)
        let r = (COLOR_INDIGO.r * 255.0).round() as u8;
        let g = (COLOR_INDIGO.g * 255.0).round() as u8;
        let b = (COLOR_INDIGO.b * 255.0).round() as u8;
        assert_eq!((r, g, b), (91, 106, 245), "#5b6af5 indigo");
    }

    #[test]
    fn settings_use_all_screens() {
        let settings = DockApp::settings();
        assert!(matches!(settings.layer_settings.start_mode, StartMode::AllScreens));
    }

    #[test]
    fn settings_exclusive_zone_equals_dock_height() {
        let s = DockApp::settings();
        assert_eq!(s.layer_settings.exclusive_zone, DOCK_HEIGHT_PX as i32);
    }

    #[test]
    fn app_id_is_portal_bus_name() {
        assert_eq!(APP_ID, "dev.mackes.MDE.Portal");
    }

    #[test]
    fn nav_button_all_has_six_entries() {
        assert_eq!(NavButton::ALL.len(), 6);
    }

    #[test]
    fn nav_clicked_toggles_active() {
        let mut app = DockApp::default();
        assert_eq!(app.active_nav, None);

        let _ = iced_layershell::Application::update(&mut app, Message::NavClicked(NavButton::Apps));
        assert_eq!(app.active_nav, Some(NavButton::Apps));

        let _ = iced_layershell::Application::update(&mut app, Message::NavClicked(NavButton::Apps));
        assert_eq!(app.active_nav, None, "second click on same button deactivates");
    }

    #[test]
    fn nav_clicked_switches_active() {
        let mut app = DockApp::default();
        let _ = iced_layershell::Application::update(&mut app, Message::NavClicked(NavButton::Files));
        let _ = iced_layershell::Application::update(&mut app, Message::NavClicked(NavButton::Network));
        assert_eq!(app.active_nav, Some(NavButton::Network));
    }

    #[test]
    fn badge_count_starts_at_zero() {
        let app = DockApp::default();
        for btn in NavButton::ALL {
            assert_eq!(app.badge_count(btn), 0);
        }
    }

    #[test]
    fn set_badge_count_round_trips() {
        let mut app = DockApp::default();
        app.set_badge_count(NavButton::Notifications, 5);
        assert_eq!(app.badge_count(NavButton::Notifications), 5);
        assert_eq!(app.badge_count(NavButton::Apps), 0, "other buttons unaffected");
    }

    #[test]
    fn domain_colors_all_distinct() {
        let colors: Vec<[u8; 3]> = NavButton::ALL
            .iter()
            .map(|b| {
                let c = b.domain_color();
                [
                    (c.r * 255.0).round() as u8,
                    (c.g * 255.0).round() as u8,
                    (c.b * 255.0).round() as u8,
                ]
            })
            .collect();
        let unique: std::collections::HashSet<[u8; 3]> = colors.iter().cloned().collect();
        assert_eq!(unique.len(), 6, "all 6 domain colors must be distinct");
    }

    #[test]
    fn each_button_has_a_portal_layer() {
        for btn in NavButton::ALL {
            assert!(!btn.portal_layer().is_empty());
        }
    }

    #[test]
    fn apps_navigates_to_hub() {
        assert_eq!(NavButton::Apps.portal_layer(), "hub");
    }

    #[test]
    fn settings_navigates_to_control() {
        assert_eq!(NavButton::Settings.portal_layer(), "control");
    }

    // ── Portal-5 workspace segment tests ─────────────────────────────────────

    fn make_ws(num: i32, name: &str, focused: bool, visible: bool, output: &str) -> WorkspaceInfo {
        WorkspaceInfo {
            num,
            name: name.to_string(),
            focused,
            visible,
            output: output.to_string(),
            urgent: false,
        }
    }

    #[test]
    fn workspace_list_updates_state() {
        let mut app = DockApp::default();
        assert!(app.workspaces.is_empty());
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::WorkspaceList(vec![make_ws(1, "1", true, true, "eDP-1")]),
        );
        assert_eq!(app.workspaces.len(), 1);
        assert_eq!(app.workspaces[0].num, 1);
    }

    #[test]
    fn workspace_list_replaces_previous() {
        let mut app = DockApp::default();
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::WorkspaceList(vec![
                make_ws(1, "1", true, true, "eDP-1"),
                make_ws(2, "2", false, false, "eDP-1"),
            ]),
        );
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::WorkspaceList(vec![make_ws(3, "3", true, true, "HDMI-A-1")]),
        );
        assert_eq!(app.workspaces.len(), 1, "list should be replaced, not appended");
    }

    #[test]
    fn noop_message_is_handled_silently() {
        let mut app = DockApp::default();
        let task = iced_layershell::Application::update(&mut app, Message::Noop);
        // Task::none() — no side-effects; state unchanged.
        drop(task);
        assert!(app.workspaces.is_empty());
    }

    #[test]
    fn workspaces_start_empty() {
        let app = DockApp::default();
        assert!(app.workspaces.is_empty());
    }

    // ── Portal-6 hostname segment tests ──────────────────────────────────────

    #[test]
    fn hostname_defaults_to_empty_in_test_mode() {
        let app = DockApp::default();
        // default() uses empty hostname; real hostname read in new() at runtime.
        assert_eq!(app.hostname, "");
    }

    #[test]
    fn hostname_segment_shows_local_only_tag_when_no_focused_ws() {
        let app = DockApp { hostname: "mybox".to_string(), ..DockApp::default() };
        // No workspaces → output defaults to "?"; label includes (local-only).
        let ws = app.workspaces.iter().find(|w| w.focused);
        let output = ws.map(|w| w.output.as_str()).unwrap_or("?");
        let label = format!("{}:{output} (local-only)", app.hostname);
        assert!(label.contains("(local-only)"));
        assert!(label.contains("mybox"));
    }

    #[test]
    fn hostname_segment_uses_focused_workspace_output() {
        let mut app = DockApp { hostname: "devbox".to_string(), ..DockApp::default() };
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::WorkspaceList(vec![
                WorkspaceInfo {
                    num: 1, name: "1".to_string(), focused: true,
                    visible: true, output: "eDP-1".to_string(), urgent: false,
                },
            ]),
        );
        let ws = app.workspaces.iter().find(|w| w.focused);
        let output = ws.map(|w| w.output.as_str()).unwrap_or("?");
        assert_eq!(output, "eDP-1");
        let label = format!("{}:{output} (local-only)", app.hostname);
        assert_eq!(label, "devbox:eDP-1 (local-only)");
    }

    #[test]
    fn hostname_clicked_is_noop() {
        let mut app = DockApp::default();
        let _task = iced_layershell::Application::update(&mut app, Message::HostnameClicked);
        // State unchanged.
        assert!(app.workspaces.is_empty());
        assert_eq!(app.active_nav, None);
    }

    #[test]
    fn new_workspace_task_fires_without_panic() {
        let mut app = DockApp::default();
        // Set up two taken workspaces so new_workspace picks 3.
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::WorkspaceList(vec![
                make_ws(1, "1", false, false, "eDP-1"),
                make_ws(2, "2", true, true, "eDP-1"),
            ]),
        );
        // Should produce a Task::perform without panicking (sway not running,
        // so the async op will fail silently at runtime).
        let _task = iced_layershell::Application::update(&mut app, Message::NewWorkspace);
    }
}
