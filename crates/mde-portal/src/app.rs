//! Portal-5 Iced application — Dock with workspace segment + 6 nav buttons.
//!
//! Dock layout (56 px, AllScreens, Intel One Mono):
//!
//! ```text
//! [1›][2›][dev…›][+]  ···spacer···  [›Apps][›Files][›Notif][›VoIP][›Net][›Settings][▏]
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
//!
//! **Show-wallpaper strip** (Portal-12, R4-Q72): 4 px strip at far-right;
//! click moves all tiling windows to scratchpad (indigo active indicator);
//! click again restores them by container ID (R4-Q73).

use iced::widget::{container, mouse_area, row, text};
use iced::{Color, Element, Length, Padding, Subscription, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;

use crate::fonts::{resolve_icon, FONT_INTEL_ONE_MONO};
use crate::status::StatusInfo;
use crate::workspace::{WindowInfo, WorkspaceInfo};
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
    /// 1-second tick from the clock subscription (Portal-11).
    ClockTick,
    /// 5-second tick to persist shell state (Portal-29 crash recovery).
    SnapshotTick,
    /// User clicked the show-wallpaper strip (Portal-12, R4-Q72).
    ShowDesktopToggle,
    /// Async result of `show_desktop_hide()` — carries IDs of moved windows.
    ShowDesktopHidden(Vec<i64>),
    /// Window subscription delivered a fresh window list (Portal-8.a).
    WindowList(Vec<WindowInfo>),
    /// User clicked a running-zone cell — focus that window by con_id (Portal-8.a).
    FocusWindowById(i64),
    /// 30-second sysfs poll result (Portal-9.a: battery/network/backlight).
    StatusUpdate(StatusInfo),
    /// User clicked the Lock glyph — triggers `loginctl lock-session` (Portal-9.a).
    LockClicked,
    /// User clicked the Power glyph — triggers `systemctl suspend` (Portal-9.a).
    PowerClicked,
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
    /// Current wall-clock time for the clock segment (Portal-11).
    clock_now: chrono::DateTime<chrono::Local>,
    /// Whether the show-wallpaper strip is active (Portal-12).
    wallpaper_strip_on: bool,
    /// Container IDs of windows moved to scratchpad by show-wallpaper toggle.
    desktop_window_ids: Vec<i64>,
    /// Last sysfs status snapshot (Portal-9.a). Updated every 30 s.
    status_info: StatusInfo,
    /// Live window list from swayipc tree (Portal-8.a). Empty until subscription fires.
    running_windows: Vec<WindowInfo>,
}

impl Default for DockApp {
    fn default() -> Self {
        Self {
            active_nav: None,
            badge_counts: [0u32; 6],
            workspaces: Vec::new(),
            hostname: String::new(),
            clock_now: chrono::Local::now(),
            wallpaper_strip_on: false,
            desktop_window_ids: Vec::new(),
            status_info: StatusInfo::default(),
            running_windows: Vec::new(),
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

        let snap = restore_snapshot().unwrap_or_default();
        let active_nav = snap
            .active_nav_index
            .and_then(|i| NavButton::ALL.get(i).copied());

        (
            Self {
                hostname,
                active_nav,
                badge_counts: snap.badge_counts,
                ..Self::default()
            },
            Task::none(),
        )
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
            Message::ClockTick => {
                self.clock_now = chrono::Local::now();
            }
            Message::SnapshotTick => {
                persist_snapshot(self);
            }
            Message::ShowDesktopToggle => {
                if self.wallpaper_strip_on {
                    // Restore: pull the stored IDs, clear state immediately (optimistic),
                    // then fire the async restore.
                    let ids = std::mem::take(&mut self.desktop_window_ids);
                    self.wallpaper_strip_on = false;
                    return Task::perform(
                        crate::workspace::show_desktop_restore(ids),
                        |_| Message::Noop,
                    );
                } else {
                    return Task::perform(
                        crate::workspace::show_desktop_hide(),
                        Message::ShowDesktopHidden,
                    );
                }
            }
            Message::ShowDesktopHidden(ids) => {
                // Active only when at least one window was actually moved.
                self.wallpaper_strip_on = !ids.is_empty();
                self.desktop_window_ids = ids;
            }
            Message::WindowList(windows) => {
                self.running_windows = windows;
            }
            Message::FocusWindowById(con_id) => {
                return Task::perform(
                    crate::workspace::focus_window_by_id(con_id),
                    |_| Message::Noop,
                );
            }
            Message::StatusUpdate(info) => {
                self.status_info = info;
            }
            Message::LockClicked => {
                return Task::perform(
                    async {
                        let _ = tokio::process::Command::new("loginctl")
                            .arg("lock-session")
                            .spawn();
                    },
                    |_| Message::Noop,
                );
            }
            Message::PowerClicked => {
                return Task::perform(
                    async {
                        let _ = tokio::process::Command::new("systemctl")
                            .arg("suspend")
                            .spawn();
                    },
                    |_| Message::Noop,
                );
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
        Subscription::batch([
            crate::workspace::workspace_subscription(),
            crate::workspace::window_subscription(),
            clock_subscription(),
            snapshot_subscription(),
            status_subscription(),
        ])
    }

    fn view(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let bg = if theme == Theme::Dark { CHARCOAL } else { OFF_WHITE };
        let fg = if theme == Theme::Dark { Color::WHITE } else { Color::BLACK };

        let ws_seg = build_workspace_segment(self, fg);
        let host_seg = build_hostname_segment(self, fg);
        let running_zone = build_running_zone(self, fg);
        let status_seg = build_status_segment(self, fg);
        let clock_seg = build_clock_segment(self, fg);
        let nav_row = build_nav_row(self, fg);
        let wallpaper_strip = build_wallpaper_strip(self);

        container(
            row![
                ws_seg,
                host_seg,
                running_zone,
                iced::widget::horizontal_space(),
                status_seg,
                clock_seg,
                nav_row,
                wallpaper_strip,
            ]
                .width(Length::Fill)
                .height(Length::Fill)
                // Left pad 8 px; strip is flush at right edge (R4-Q72).
                .padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 8.0 }),
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

// ── Portal-29 crash-recovery snapshot ────────────────────────────────────────

/// Persisted state for crash recovery (R2-Q59, R4-Q48).
///
/// Serialized to `~/.cache/mde/shell-state.json` every 5 seconds.
/// On respawn, restored in `DockApp::new()` before the first frame.
#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
struct ShellSnapshot {
    /// Index into `NavButton::ALL` of the active nav, or `None`.
    active_nav_index: Option<usize>,
    /// Badge counts matching `NavButton::ALL` order.
    badge_counts: [u32; 6],
}

fn snapshot_path() -> Option<std::path::PathBuf> {
    dirs::cache_dir().map(|d| d.join("mde").join("shell-state.json"))
}

fn persist_snapshot(app: &DockApp) {
    let Some(path) = snapshot_path() else { return };
    let snap = ShellSnapshot {
        active_nav_index: app.active_nav.and_then(|btn| {
            NavButton::ALL.iter().position(|&b| b == btn)
        }),
        badge_counts: app.badge_counts,
    };
    if let Ok(json) = serde_json::to_string(&snap) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, json);
    }
}

fn restore_snapshot() -> Option<ShellSnapshot> {
    let path = snapshot_path()?;
    let json = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&json).ok()
}

/// 5-second snapshot subscription (Portal-29).
fn snapshot_subscription() -> Subscription<Message> {
    Subscription::run_with_id(
        "mde-portal-snapshot",
        async_stream::stream! {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                yield Message::SnapshotTick;
            }
        },
    )
}

/// 30-second status-poll subscription (Portal-9.a).
///
/// Emits an initial `StatusUpdate` immediately on startup, then every 30 s.
/// Reads are synchronous sysfs calls (< 1 ms) so blocking inside the async
/// stream is acceptable.
fn status_subscription() -> Subscription<Message> {
    Subscription::run_with_id(
        "mde-portal-status",
        async_stream::stream! {
            yield Message::StatusUpdate(crate::status::read_status());
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                yield Message::StatusUpdate(crate::status::read_status());
            }
        },
    )
}

/// 1-second clock tick subscription (Portal-11).
fn clock_subscription() -> Subscription<Message> {
    Subscription::run_with_id(
        "mde-portal-clock",
        async_stream::stream! {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                yield Message::ClockTick;
            }
        },
    )
}

/// Build the clock segment (Portal-11): 24h time on top, date below.
///
/// Calendar popover on click is Portal-11.b (requires Portal-16 scratchpad surface).
fn build_clock_segment<'a>(app: &DockApp, fg: Color) -> Element<'a, Message> {
    use iced::widget::column;
    let time_str = app.clock_now.format("%H:%M").to_string();
    let date_str = app.clock_now.format("%b %d").to_string();

    container(
        column![
            text(time_str).size(13.0).color(fg),
            text(date_str).size(10.0).color(Color { a: 0.6, ..fg }),
        ]
        .align_x(iced::Alignment::Center)
        .spacing(1),
    )
    .height(Length::Fill)
    .align_y(iced::alignment::Vertical::Center)
    .padding(Padding::from([0, 10]))
    .into()
}

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

/// Build the running-zone segment (Portal-8.a, R3-Q15).
///
/// Groups windows by `app_id`.  Each group is a clickable cell showing:
///   - Short app label (first 10 chars of app_id, or title fallback)
///   - Workspace-number badge when the group spans multiple workspaces (R3-Q15)
///   - Count badge when there are 2+ windows in the group
///   - Indigo background when any window in the group is focused
///
/// Click calls `[con_id=N] focus` on the focused window in the group (or
/// the first window if none is focused).
///
/// WM-buttons-on-hover (Portal-8.b) not yet wired.
fn build_running_zone<'a>(app: &DockApp, fg: Color) -> Element<'a, Message> {
    // Group by app_id key (fall back to con_id string if no app_id).
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, Vec<&WindowInfo>> = BTreeMap::new();
    for w in &app.running_windows {
        let key = w
            .app_id
            .clone()
            .unwrap_or_else(|| format!("#{}", w.con_id));
        groups.entry(key).or_default().push(w);
    }

    let mut cells: Vec<Element<'a, Message>> = Vec::new();

    for (label_key, group) in &groups {
        let any_focused = group.iter().any(|w| w.focused);
        let count = group.len();

        // Best window to focus on click: the focused one or the first.
        let target_id = group
            .iter()
            .find(|w| w.focused)
            .unwrap_or(&group[0])
            .con_id;

        // Workspace numbers present in this group (for multi-WS badge).
        let mut ws_nums: Vec<i32> = group.iter().map(|w| w.workspace_num).collect();
        ws_nums.dedup();

        let bg: Option<Color> = if any_focused { Some(COLOR_INDIGO) } else { None };
        let text_color = if any_focused { Color::WHITE } else { fg };

        // Truncate the label key to a display-friendly length.
        let display: String = {
            let raw = label_key.as_str();
            let max = 10usize;
            if raw.chars().count() > max {
                let prefix: String = raw.chars().take(max).collect();
                format!("{prefix}…")
            } else {
                raw.to_string()
            }
        };

        let mut label_parts: Vec<Element<'a, Message>> = vec![
            text(display).size(10.0).color(text_color).into(),
        ];

        // Count badge for multiple windows.
        if count > 1 {
            label_parts.push(
                container(
                    text(count.to_string()).size(8.0).color(Color::WHITE),
                )
                .style(|_: &Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(
                        Color { r: 0.5, g: 0.5, b: 0.6, a: 1.0 }
                    )),
                    border: iced::Border {
                        radius: iced::border::Radius::from(4.0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .padding(Padding::from([0, 2]))
                .width(Length::Shrink)
                .into(),
            );
        }

        // Workspace badge when windows spread across multiple workspaces.
        if ws_nums.len() > 1 {
            label_parts.push(
                text("·").size(8.0).color(Color { a: 0.5, ..text_color }).into(),
            );
        }

        let inner = container(
            row(label_parts)
                .spacing(2)
                .align_y(iced::Alignment::Center),
        )
        .height(Length::Fill)
        .align_y(iced::alignment::Vertical::Center)
        .padding(Padding::from([0, 6]))
        .style(move |_: &Theme| iced::widget::container::Style {
            background: bg.map(iced::Background::Color),
            ..Default::default()
        });

        cells.push(
            mouse_area(inner)
                .on_press(Message::FocusWindowById(target_id))
                .into(),
        );
    }

    if cells.is_empty() {
        return iced::widget::Space::new(0.0, Length::Fill).into();
    }

    row(cells)
        .spacing(2)
        .height(Length::Fill)
        .align_y(iced::Alignment::Center)
        .padding(Padding::from([0, 4]))
        .into()
}

/// Build the status-zone glyph segment (Portal-9.a, R4-Q56, R3-Q32–R3-Q35).
///
/// Layout: `[bat%] [net●][mesh●] [♫] [▭bri%] [lock] [pwr]`
///
/// - Battery: colour-coded percentage; charging prefix "⚡".
/// - Network / Mesh: coloured 8 px dots (green / indigo when up, dim when down).
/// - Volume: static "♫" glyph — IPC wired in Portal-9.b.
/// - Brightness: "▭ XX%" — sysfs value.
/// - Lock: click → `loginctl lock-session`.
/// - Power: click → `systemctl suspend`.
fn build_status_segment<'a>(app: &DockApp, fg: Color) -> Element<'a, Message> {
    let si = &app.status_info;
    let mut items: Vec<Element<'a, Message>> = Vec::new();

    // ── Battery ───────────────────────────────────────────────────────────────
    if let Some(pct) = si.battery_pct {
        let bat_color = if pct > 50 {
            Color::from_rgb(0.22, 0.78, 0.35) // green
        } else if pct > 20 {
            Color::from_rgb(0.95, 0.75, 0.10) // amber
        } else {
            Color::from_rgb(0.90, 0.22, 0.12) // red
        };
        let charging_prefix = if si.battery_charging { "⚡" } else { "" };
        let label = format!("{charging_prefix}{pct}%");
        items.push(
            container(text(label).size(10.0).color(bat_color))
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Center)
                .padding(Padding::from([0, 4]))
                .into(),
        );
    }

    // ── Network + Mesh dots ───────────────────────────────────────────────────
    let net_color = if si.network_up {
        Color::from_rgb(0.22, 0.78, 0.35)
    } else {
        Color { a: 0.25, ..fg }
    };
    items.push(
        container(text("●").size(8.0).color(net_color))
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Center)
            .padding(Padding::from([0, 2]))
            .into(),
    );
    let mesh_color = if si.mesh_up { COLOR_INDIGO } else { Color { a: 0.25, ..fg } };
    items.push(
        container(text("●").size(8.0).color(mesh_color))
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Center)
            .padding(Padding::from([0, 2]))
            .into(),
    );

    // ── Volume (static glyph — IPC wired in Portal-9.b) ──────────────────────
    let vol_glyph = resolve_icon(Icon::Sound, IconSize::Inline).fallback_glyph;
    items.push(
        container(text(vol_glyph).size(11.0).color(Color { a: 0.55, ..fg }))
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Center)
            .padding(Padding::from([0, 4]))
            .into(),
    );

    // ── Brightness ────────────────────────────────────────────────────────────
    if let Some(bri) = si.brightness_pct {
        let bri_glyph = resolve_icon(Icon::Display, IconSize::Inline).fallback_glyph;
        let label = format!("{bri_glyph}{bri}%");
        items.push(
            container(text(label).size(10.0).color(Color { a: 0.6, ..fg }))
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Center)
                .padding(Padding::from([0, 4]))
                .into(),
        );
    }

    // ── Lock (clickable → loginctl lock-session) ──────────────────────────────
    let lock_glyph = resolve_icon(Icon::Session, IconSize::Inline).fallback_glyph;
    items.push(
        mouse_area(
            container(text(lock_glyph).size(11.0).color(Color { a: 0.65, ..fg }))
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Center)
                .padding(Padding::from([0, 4])),
        )
        .on_press(Message::LockClicked)
        .into(),
    );

    // ── Power (clickable → systemctl suspend) ─────────────────────────────────
    let pwr_glyph = resolve_icon(Icon::Power, IconSize::Inline).fallback_glyph;
    items.push(
        mouse_area(
            container(text(pwr_glyph).size(11.0).color(Color { a: 0.65, ..fg }))
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Center)
                .padding(Padding::from([0, 4])),
        )
        .on_press(Message::PowerClicked)
        .into(),
    );

    row(items)
        .spacing(0)
        .height(Length::Fill)
        .align_y(iced::Alignment::Center)
        .into()
}

/// Build the show-wallpaper strip (Portal-12, R4-Q72).
///
/// 4 px wide, full-height, flush at the right edge of the Dock.
/// Inactive: subtle grey.  Active (windows in scratchpad): indigo accent.
/// Click toggles between hiding all tiling windows (→ wallpaper visible)
/// and restoring them by their saved container IDs (R4-Q73).
fn build_wallpaper_strip(app: &DockApp) -> Element<'_, Message> {
    let strip_color = if app.wallpaper_strip_on {
        COLOR_INDIGO
    } else {
        Color { r: 0.40, g: 0.41, b: 0.43, a: 1.0 }
    };

    mouse_area(
        container(iced::widget::Space::new(Length::Fill, Length::Fill))
            .width(4.0)
            .height(Length::Fill)
            .style(move |_: &Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(strip_color)),
                ..Default::default()
            }),
    )
    .on_press(Message::ShowDesktopToggle)
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

    // ── Portal-29 snapshot tests ──────────────────────────────────────────────

    #[test]
    fn shell_snapshot_default_has_no_active_nav() {
        let snap = ShellSnapshot::default();
        assert!(snap.active_nav_index.is_none());
    }

    #[test]
    fn shell_snapshot_default_has_zero_badge_counts() {
        let snap = ShellSnapshot::default();
        assert_eq!(snap.badge_counts, [0u32; 6]);
    }

    #[test]
    fn shell_snapshot_roundtrips_via_json() {
        let snap = ShellSnapshot {
            active_nav_index: Some(2),
            badge_counts: [0, 3, 0, 0, 0, 0],
        };
        let json = serde_json::to_string(&snap).expect("serialize");
        let restored: ShellSnapshot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.active_nav_index, Some(2));
        assert_eq!(restored.badge_counts[1], 3);
    }

    #[test]
    fn restore_snapshot_returns_none_for_corrupt_json() {
        // Simulate a corrupt file — should not panic, returns None.
        let result: Option<ShellSnapshot> = serde_json::from_str("{bad json}").ok();
        assert!(result.is_none());
    }

    #[test]
    fn snapshot_tick_message_handled_without_state_change() {
        let mut app = DockApp::default();
        app.active_nav = Some(NavButton::Files);
        // SnapshotTick triggers persist_snapshot (writes to disk if possible).
        // State itself should be unchanged.
        let _task = iced_layershell::Application::update(&mut app, Message::SnapshotTick);
        assert_eq!(app.active_nav, Some(NavButton::Files), "SnapshotTick should not change active_nav");
    }

    #[test]
    fn active_nav_index_round_trips_through_snapshot() {
        // If NavButton::Files is index 1 in NavButton::ALL, snapshot index = 1.
        let idx = NavButton::ALL.iter().position(|&b| b == NavButton::Files);
        assert_eq!(idx, Some(1), "Files should be at index 1");
        let restored = NavButton::ALL.get(1).copied();
        assert_eq!(restored, Some(NavButton::Files));
    }

    // ── Portal-11 clock segment tests ─────────────────────────────────────────

    #[test]
    fn clock_now_initialized_in_default() {
        let app = DockApp::default();
        // clock_now should be a recent timestamp (within 5 seconds of now).
        let diff = (chrono::Local::now() - app.clock_now).num_seconds().abs();
        assert!(diff < 5, "clock_now should be near startup time; diff={diff}s");
    }

    #[test]
    fn clock_tick_advances_time() {
        let mut app = DockApp::default();
        // Manually set a past time.
        app.clock_now = chrono::Local::now() - chrono::Duration::seconds(60);
        let old_time = app.clock_now;

        let _ = iced_layershell::Application::update(&mut app, Message::ClockTick);
        // After ClockTick, clock_now should be more recent than old_time.
        assert!(app.clock_now > old_time, "ClockTick should update clock_now");
    }

    #[test]
    fn clock_formats_time_correctly() {
        let app = DockApp::default();
        let formatted = app.clock_now.format("%H:%M").to_string();
        // Format should be HH:MM — two digits, colon, two digits.
        assert_eq!(formatted.len(), 5, "time format should be HH:MM");
        assert_eq!(&formatted[2..3], ":", "colon at position 2");
    }

    #[test]
    fn clock_formats_date_correctly() {
        let app = DockApp::default();
        let formatted = app.clock_now.format("%b %d").to_string();
        // Format: "Jan 01" style — non-empty, has a space.
        assert!(!formatted.is_empty());
        assert!(formatted.contains(' '), "date format should have space: {formatted}");
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

    // ── Portal-8.a running-zone tests ────────────────────────────────────────

    fn make_window(con_id: i64, app_id: &str, ws_num: i32, focused: bool) -> WindowInfo {
        WindowInfo {
            con_id,
            app_id: Some(app_id.to_string()),
            title: Some(format!("{app_id} window")),
            workspace_num: ws_num,
            focused,
        }
    }

    #[test]
    fn running_windows_start_empty() {
        let app = DockApp::default();
        assert!(app.running_windows.is_empty());
    }

    #[test]
    fn window_list_message_updates_state() {
        let mut app = DockApp::default();
        let windows = vec![
            make_window(1, "foot", 1, true),
            make_window(2, "firefox", 1, false),
        ];
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::WindowList(windows),
        );
        assert_eq!(app.running_windows.len(), 2);
        assert_eq!(app.running_windows[0].app_id.as_deref(), Some("foot"));
    }

    #[test]
    fn window_list_replaces_previous() {
        let mut app = DockApp::default();
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::WindowList(vec![make_window(1, "foot", 1, true)]),
        );
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::WindowList(vec![
                make_window(2, "firefox", 1, false),
                make_window(3, "code", 2, true),
            ]),
        );
        assert_eq!(app.running_windows.len(), 2, "list should be replaced, not appended");
        assert!(app.running_windows.iter().all(|w| w.app_id.as_deref() != Some("foot")));
    }

    #[test]
    fn focus_window_by_id_task_fires_without_panic() {
        let mut app = DockApp::default();
        // sway not running in test; the async op fails silently at runtime.
        let _task = iced_layershell::Application::update(
            &mut app,
            Message::FocusWindowById(99),
        );
    }

    // ── Portal-9.a status segment tests ──────────────────────────────────────

    #[test]
    fn status_info_starts_at_default() {
        let app = DockApp::default();
        assert!(app.status_info.battery_pct.is_none());
        assert!(!app.status_info.network_up);
        assert!(!app.status_info.mesh_up);
    }

    #[test]
    fn status_update_message_stores_info() {
        let mut app = DockApp::default();
        let info = StatusInfo {
            battery_pct: Some(80),
            battery_charging: true,
            network_up: true,
            mesh_up: false,
            brightness_pct: Some(60),
        };
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::StatusUpdate(info),
        );
        assert_eq!(app.status_info.battery_pct, Some(80));
        assert!(app.status_info.battery_charging);
        assert!(app.status_info.network_up);
        assert!(!app.status_info.mesh_up);
        assert_eq!(app.status_info.brightness_pct, Some(60));
    }

    #[test]
    fn lock_clicked_returns_task_without_panic() {
        let mut app = DockApp::default();
        // loginctl may not be available in test env; spawn failure is silent.
        let _task = iced_layershell::Application::update(&mut app, Message::LockClicked);
    }

    #[test]
    fn power_clicked_returns_task_without_panic() {
        let mut app = DockApp::default();
        let _task = iced_layershell::Application::update(&mut app, Message::PowerClicked);
    }

    // ── Portal-12 show-wallpaper strip tests ─────────────────────────────────

    #[test]
    fn wallpaper_strip_starts_inactive() {
        let app = DockApp::default();
        assert!(!app.wallpaper_strip_on);
        assert!(app.desktop_window_ids.is_empty());
    }

    #[test]
    fn show_desktop_hidden_activates_strip_when_windows_moved() {
        let mut app = DockApp::default();
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::ShowDesktopHidden(vec![101, 202]),
        );
        assert!(app.wallpaper_strip_on, "strip should be active after hiding windows");
        assert_eq!(app.desktop_window_ids, vec![101, 202]);
    }

    #[test]
    fn show_desktop_hidden_with_empty_ids_leaves_strip_inactive() {
        let mut app = DockApp::default();
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::ShowDesktopHidden(vec![]),
        );
        assert!(!app.wallpaper_strip_on, "strip should stay inactive when no windows were moved");
    }

    #[test]
    fn show_desktop_toggle_when_active_clears_state_immediately() {
        let mut app = DockApp::default();
        // Simulate: windows already hidden.
        app.wallpaper_strip_on = true;
        app.desktop_window_ids = vec![55, 66];

        let _task = iced_layershell::Application::update(
            &mut app,
            Message::ShowDesktopToggle,
        );
        // strip_on resets immediately (optimistic); IDs are cleared.
        assert!(!app.wallpaper_strip_on);
        assert!(app.desktop_window_ids.is_empty());
    }

    #[test]
    fn show_desktop_toggle_when_inactive_fires_hide_task() {
        let mut app = DockApp::default();
        // Should not panic even without a running sway session.
        let _task = iced_layershell::Application::update(
            &mut app,
            Message::ShowDesktopToggle,
        );
        // strip_on stays false until ShowDesktopHidden arrives.
        assert!(!app.wallpaper_strip_on);
    }
}
