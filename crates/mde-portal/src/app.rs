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

use std::collections::HashMap;

use iced::widget::scrollable::{self, AbsoluteOffset, Direction, Scrollbar};
use iced::widget::{container, mouse_area, row, text};
use iced::{Color, Element, Length, Padding, Subscription, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;

use crate::fonts::{resolve_icon, FONT_INTEL_ONE_MONO};
use crate::status::StatusInfo;
use crate::workspace::{WindowInfo, WorkspaceInfo, WS_NAME_MAX_CHARS};
use mde_theme::{Icon, IconSize};

// ── marquee constants (Portal-5.b) ────────────────────────────────────────────

/// Fixed display width of a marquee workspace cell in logical pixels.
const WS_MARQUEE_CELL_PX: f32 = 64.0;
/// Intel One Mono at 12 px: ~7.2 px/char (monospace advance).
const WS_MARQUEE_PX_PER_CHAR: f32 = 7.2;
/// Visual gap between the name and its duplicate (spaces as chars).
const WS_MARQUEE_GAP: &str = "   ";
/// px advanced per 20 ms tick → 50 px/sec.
const WS_MARQUEE_ADVANCE_PX: f32 = 1.0;

/// Crate-private app-id constant visible to the layer-shell compositor.
pub(crate) const APP_ID: &str = "dev.mackes.MDE.Portal";

/// Portal-59 (R12-Q24): workspace 99 is the platform's reserved "parked-
/// window" slot. The 5th micro-button (`↓`) parks the focused window here
/// instead of moving it to sway's scratchpad; Mod+Shift+m un-parks the
/// most-recently-focused parked window. Running-zone + mini-tree both
/// filter this number out so a parked window reads as minimized.
pub const PARKED_WORKSPACE_NUM: i32 = 99;

/// Portal-43 (R12-Q3): cap of the visited-workspaces LRU. Exceeding this
/// pushes the oldest entry off the back. Five is the design lock.
pub const PREV_WS_LRU_CAP: usize = 5;

/// Portal-43 (R12-Q3): the transient "previous workspace" breadcrumb
/// segment auto-dismisses this many seconds after the last focus change.
/// Re-focuses inside the window keep the segment alive (the timer
/// resets on every focus change).
pub const PREV_WS_SEGMENT_TTL_SECS: i64 = 5;

/// Portal-50 (R12-Q11): TTL for the prompt-on-change layout banner.
/// Click ✓ / ✕ or the timer firing dismisses the banner. Repeated
/// layout flips within the TTL window collapse — last layout wins —
/// because each flip overwrites the active `LayoutPromptState`.
pub const LAYOUT_PROMPT_TTL_SECS: i64 = 8;

/// Portal-57.b (R12-Q22): TTL for the mini-tree pulse cell-flash.
/// Each `bus/mbadge/pulse` event surfaces the matching workspace
/// cell in tier-red for this many milliseconds before the cell
/// returns to its normal render.
pub const URGENT_PULSE_TTL_MS: i64 = 1200;

/// BUS-2.2.a: TTL for `fleet/announce` breadcrumb segments. Set
/// well past the 200 ms exit-condition so operators have time to
/// read the message before it auto-dismisses; a follow-up
/// BUS-2.2.dismiss task may add explicit click-to-dismiss.
pub const BUS_ANNOUNCE_TTL_SECS: i64 = 6;

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
    /// Portal-43 (R12-Q3): user clicked the transient previous-workspace
    /// breadcrumb segment. Routes through swayipc `workspace number <n>`
    /// to jump back. `num` is the i3/sway workspace number; the rename
    /// worker (Portal-41) preserves it even when the name is prefixed.
    PrevWorkspaceClicked(i32),
    /// Portal-45 (R12-Q5): sway entered a non-default binding mode (or
    /// returned to the default). `None` clears the mode segment;
    /// `Some(name)` renders it on the far-left of the Dock.
    ModeChanged(Option<String>),
    /// Portal-50 (R12-Q11): sway emitted a binding-executed event with
    /// the command string. The Dock parses it for `layout` directives +
    /// may raise the prompt-on-change layout banner.
    BindingExecuted(String),
    /// Portal-50 (R12-Q11): user clicked the ✓ button on the
    /// prompt-on-change layout banner. Updates the owning tag's
    /// `default_layout` field in tag.json + dismisses the banner.
    MakeTagDefaultLayout {
        /// Tag name the prompt belongs to.
        tag_name: String,
        /// New layout to write as the tag default.
        layout: String,
    },
    /// Portal-50 (R12-Q11): banner auto-dismissed (8 s TTL fired)
    /// or operator-initiated dismiss without persisting. Clears
    /// the banner from state with no side effects.
    DismissLayoutPrompt,
    /// Portal-50.b (R12-Q11): user clicked the ✕ button on the
    /// prompt-on-change layout banner — "keep this layout for
    /// this workspace only." Writes a per-workspace override to
    /// `<XDG_DATA_HOME>/mde/workspaces.json` + dismisses the
    /// banner. The tag_layout worker reads the override on
    /// `window::new` events instead of the tag's `default_layout`.
    DeclineTagDefaultLayout {
        /// Workspace the override applies to.
        workspace_num: i32,
        /// Layout to pin for this workspace only.
        layout: String,
    },
    /// Portal-57.b (R12-Q22): a new Bus pulse event arrived from
    /// `bus/mbadge/pulse`. Pushed into `DockApp::recent_pulses`
    /// for the TTL window; mini-tree cells matching the pulse's
    /// workspace render in tier-red.
    UrgentPulse(UrgentPulse),
    /// BUS-2.2.a: a new Bus message arrived from `fleet/announce`.
    /// Pushed into `DockApp::bus_announce_segments` for the TTL
    /// window; rendered as a priority-tinted pill near the host
    /// segment.
    BusAnnounceSegment(BusAnnounceSegment),
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
    /// 20ms tick to advance workspace-name marquee offsets (Portal-5.b, R4-Q64).
    MarqueeTick,
    /// User clicked the show-wallpaper strip (Portal-12, R4-Q72).
    ShowDesktopToggle,
    /// Async result of `show_desktop_hide()` — carries IDs of moved windows.
    ShowDesktopHidden(Vec<i64>),
    /// Window subscription delivered a fresh window list (Portal-8.a).
    WindowList(Vec<WindowInfo>),
    /// User clicked a running-zone cell — focus that window by con_id (Portal-8.a).
    FocusWindowById(i64),
    /// Cursor entered a running-zone group cell — show WM micro-buttons (Portal-8.b).
    HoverRunningGroup(String),
    /// Cursor left a running-zone group cell — hide WM micro-buttons (Portal-8.b).
    UnhoverRunningGroup,
    /// WM micro-button: close the window (Portal-8.b, R4-Q67).
    WmClose(i64),
    /// WM micro-button: toggle floating (Portal-8.b, R4-Q68).
    WmFloat(i64),
    /// WM micro-button: toggle fullscreen (Portal-8.b, R4-Q69).
    WmFull(i64),
    /// WM micro-button: minimize the window by parking it at workspace 99
    /// (Portal-8.b, R4-Q67 reframed by Portal-59 / R12-Q24). The parked
    /// workspace is filtered out of mini-tree + running-zone, so parking
    /// feels like a minimize while sway keeps the window first-class.
    WmMinimize(i64),
    /// WM micro-button: cycle parent layout split→tabbed→stacking (Portal-8.b, R4-Q71).
    WmLayoutCycle(i64),
    /// 30-second sysfs poll result (Portal-9.a: battery/network/backlight).
    StatusUpdate(StatusInfo),
    /// User clicked the Lock glyph — triggers `loginctl lock-session` (Portal-9.a).
    LockClicked,
    /// User clicked the Power glyph — triggers `systemctl suspend` (Portal-9.a).
    PowerClicked,
    /// User clicked the clock segment — spawns `mde-popover clock` calendar (Portal-11.b).
    ClockClicked,
    /// User clicked the volume or brightness glyph — spawns `mde-popover status` (Portal-9.b).
    StatusZoneClicked,
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
    /// App-id key of the currently hovered running-zone group; `None` = no hover.
    hovered_running_group: Option<String>,
    /// Horizontal scroll offsets (px) per workspace name for marquee (Portal-5.b).
    ws_marquee_offsets: HashMap<String, f32>,
    /// Portal-43 (R12-Q3): up to 5 most-recently-visited workspaces, newest
    /// at index 0. Stores `(workspace_num, name)` so the breadcrumb segment
    /// can render the auto-derived name from Portal-41. Pushed on every
    /// focus change; adjacent duplicates (the focused workspace appearing
    /// twice in a row) collapse.
    visited_workspaces_lru: Vec<(i32, String)>,
    /// Portal-43 (R12-Q3): timestamp of the most recent focus change. The
    /// transient previous-workspace segment renders for
    /// [`PREV_WS_SEGMENT_TTL_SECS`] seconds after this stamp. `None` =
    /// segment never spawned yet.
    last_workspace_change: Option<chrono::DateTime<chrono::Local>>,
    /// Portal-45 (R12-Q5): current sway binding mode. `None` = default
    /// mode (no mode segment rendered); `Some(name)` = render the
    /// far-left `MODE: <name>` segment in the Dock breadcrumb.
    current_sway_mode: Option<String>,
    /// Portal-50 (R12-Q11): active prompt-on-change layout banner.
    /// `None` = no banner; `Some(state)` = banner visible until the
    /// TTL fires or the operator clicks ✓ / ✕.
    layout_prompt: Option<LayoutPromptState>,
    /// Portal-57.b (R12-Q22): recent urgent pulses from the Bus
    /// `bus/mbadge/pulse` topic. Each entry stays alive for
    /// [`URGENT_PULSE_TTL_MS`] ms after `spawned_at`. Mini-tree
    /// cells with a workspace_num matching any active pulse render
    /// in tier-red.
    recent_pulses: Vec<UrgentPulse>,
    /// BUS-2.2.a: recent breadcrumb segments from the Bus
    /// `fleet/announce` topic. Each entry stays alive for
    /// [`BUS_ANNOUNCE_TTL_SECS`] seconds. Rendered as a transient
    /// pill near the host segment, tinted by priority class.
    bus_announce_segments: Vec<BusAnnounceSegment>,
}

/// BUS-2.2.a: one breadcrumb segment from the Bus
/// `fleet/announce` topic. Held in `DockApp::bus_announce_segments`
/// for the TTL window so the breadcrumb renderer can show the
/// transient pill near the host segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusAnnounceSegment {
    /// ULID from the BUS-1.4 message envelope.
    pub ulid: String,
    /// Priority class (`urgent` / `high` / `default` / `min` —
    /// `min` is filtered at the parser, never lands here).
    pub priority: String,
    /// Optional title.
    pub title: Option<String>,
    /// Optional body.
    pub body: Option<String>,
    /// Local timestamp at observe. Drives TTL.
    pub spawned_at: chrono::DateTime<chrono::Local>,
    /// BUS-2.2.b: source topic — drives the segment tint. `Announce`
    /// pills use the priority palette; `Clipboard` pills always
    /// render neutral grey per the design lock.
    pub topic: BusSegmentTopic,
}

/// BUS-2.2.b: source-topic discriminator for breadcrumb segments.
/// Drives tint selection in `bus_segment_color`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BusSegmentTopic {
    /// `fleet/announce` — priority palette (red / orange / blue).
    #[default]
    Announce,
    /// `clipboard/sync` — neutral grey regardless of priority. The
    /// design lock says clipboard adds are visually distinct from
    /// notifications so operators can tell them apart at a glance.
    Clipboard,
}

/// Portal-57.b (R12-Q22): one urgent-pulse event in flight. Held in
/// `DockApp::recent_pulses` for the TTL window so the mini-tree
/// segment can render the cell-flash effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrgentPulse {
    /// ULID from the Bus message envelope. Stable per-event.
    pub ulid: String,
    /// `app_id` of the urgent window (from the envelope's `source`).
    pub app_id: String,
    /// sway container ID — used to look up the workspace_num via
    /// `DockApp::running_windows`.
    pub con_id: i64,
    /// When the pulse was observed locally. Used with
    /// [`URGENT_PULSE_TTL_MS`] to compute auto-clear.
    pub spawned_at: chrono::DateTime<chrono::Local>,
}

/// Portal-50 (R12-Q11): state of the active prompt-on-change layout
/// banner. Held by `DockApp::layout_prompt` while a banner is alive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutPromptState {
    /// Workspace where the binding fired.
    pub workspace_num: i32,
    /// Layout name the operator just switched to (`splith` /
    /// `splitv` / `tabbed` / `stacked`).
    pub new_layout: String,
    /// Name of the owning tag (the prompt says "Make <tag>
    /// default?").
    pub tag_name: String,
    /// Tag's `group_color` hex string (e.g. `#42be65`); falls back
    /// to platform-default when None.
    pub tag_color: Option<String>,
    /// Timestamp when the banner was spawned. Used with
    /// [`LAYOUT_PROMPT_TTL_SECS`] to compute auto-dismiss.
    pub spawned_at: chrono::DateTime<chrono::Local>,
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
            hovered_running_group: None,
            ws_marquee_offsets: HashMap::new(),
            visited_workspaces_lru: Vec::new(),
            last_workspace_change: None,
            current_sway_mode: None,
            layout_prompt: None,
            recent_pulses: Vec::new(),
            bus_announce_segments: Vec::new(),
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
                // Portal-16: drive the Portal-full scratchpad surface.
                // prev = None        → show the window + goto the target layer.
                // prev = Some(btn)   → same button clicked again; hide the window.
                // prev = Some(other) → portal already visible; just switch layer.
                let prev = self.active_nav;
                self.active_nav = if prev == Some(btn) { None } else { Some(btn) };
                let layer = btn.portal_layer();
                return match prev {
                    None => Task::batch([
                        Task::perform(
                            crate::workspace::portal_full_scratchpad_toggle(),
                            |_| Message::Noop,
                        ),
                        Task::perform(
                            crate::dbus::portal_full_goto(layer),
                            |_| Message::Noop,
                        ),
                    ]),
                    Some(p) if p == btn => Task::perform(
                        crate::workspace::portal_full_scratchpad_toggle(),
                        |_| Message::Noop,
                    ),
                    Some(_) => Task::perform(
                        crate::dbus::portal_full_goto(layer),
                        |_| Message::Noop,
                    ),
                };
            }
            Message::NavRightClicked(_btn) => {
                // Portal-4: right-click is recorded; the context popover is
                // Portal-16's scratchpad surface. For now the click is a
                // no-op so the button state doesn't change.
            }
            Message::WorkspaceList(list) => {
                // Portal-43 (R12-Q3): track focus changes for the
                // transient previous-workspace breadcrumb segment.
                // Compute the old + new focused workspaces; if they
                // differ, push the old onto the LRU + bump the
                // last-change timestamp so the segment respawns.
                let old_focused = focused_workspace(&self.workspaces);
                let new_focused = focused_workspace(&list);
                if let (Some(old), Some(new)) = (&old_focused, &new_focused) {
                    if old.0 != new.0 {
                        push_visited_workspace(&mut self.visited_workspaces_lru, old.clone());
                        self.last_workspace_change = Some(chrono::Local::now());
                    }
                } else if old_focused.is_none() && new_focused.is_some() {
                    // Very first focus event seeds the timestamp so
                    // the segment doesn't render on cold boot.
                    self.last_workspace_change = Some(chrono::Local::now());
                }
                self.workspaces = list;
            }
            Message::PrevWorkspaceClicked(num) => {
                // Jump back to the previous workspace. swayipc's
                // numeric `workspace number <n>` accepts the bare
                // number even when the rename worker has prefixed
                // the name; sway treats `<n>` as a workspace ID.
                return Task::perform(
                    crate::workspace::focus_workspace_by_num(num),
                    |_| Message::Noop,
                );
            }
            Message::ModeChanged(next) => {
                // Portal-45 (R12-Q5): sway emitted a binding-mode
                // change. None clears the segment (back to default);
                // Some(name) raises the far-left segment.
                self.current_sway_mode = next;
            }
            Message::BindingExecuted(command) => {
                // Portal-50 (R12-Q11): sway just executed a binding-
                // bound command. If it was a `layout <name>` directive
                // AND the focused workspace is tag-owned AND the new
                // layout differs from the tag's default → spawn the
                // prompt-on-change banner.
                if let Some(prompt) = compute_layout_prompt(self, &command) {
                    self.layout_prompt = Some(prompt);
                }
            }
            Message::MakeTagDefaultLayout { tag_name, layout } => {
                // Portal-50 (R12-Q11): ✓ click — write the new layout
                // as the owning tag's default_layout, then dismiss
                // the banner.
                if let Err(e) = update_tag_default_layout(&tag_name, &layout) {
                    tracing::warn!(tag = %tag_name, %layout, error = %e, "MakeTagDefaultLayout: tag-store update failed");
                }
                self.layout_prompt = None;
            }
            Message::DismissLayoutPrompt => {
                // Portal-50 (R12-Q11): auto-dismiss timer fired or
                // banner manually cleared without persisting. No
                // side effects.
                self.layout_prompt = None;
            }
            Message::DeclineTagDefaultLayout { workspace_num, layout } => {
                // Portal-50.b (R12-Q11): ✕ click — write a per-
                // workspace override to workspaces.json so the
                // tag_layout worker (Portal-44) honors this
                // layout for this workspace next time
                // window::new fires there.
                if let Err(e) = write_workspace_layout_override(workspace_num, &layout) {
                    tracing::warn!(workspace_num, %layout, error = %e, "DeclineTagDefaultLayout: workspaces.json write failed");
                }
                self.layout_prompt = None;
            }
            Message::UrgentPulse(pulse) => {
                // Portal-57.b (R12-Q22): record the pulse + sweep
                // out any expired entries. Cap the buffer at 32
                // to keep the live-set bounded even if events
                // arrive faster than they expire.
                let now = chrono::Local::now();
                self.recent_pulses
                    .retain(|p| is_pulse_alive(p, now));
                self.recent_pulses.push(pulse);
                if self.recent_pulses.len() > 32 {
                    let overflow = self.recent_pulses.len() - 32;
                    self.recent_pulses.drain(..overflow);
                }
            }
            Message::BusAnnounceSegment(segment) => {
                // BUS-2.2.a: record the breadcrumb + sweep expired
                // entries. Cap at 8 (Dock breadcrumb has limited
                // horizontal space; older segments evict).
                let now = chrono::Local::now();
                self.bus_announce_segments
                    .retain(|s| is_bus_segment_alive(s, now));
                self.bus_announce_segments.push(segment);
                if self.bus_announce_segments.len() > 8 {
                    let overflow = self.bus_announce_segments.len() - 8;
                    self.bus_announce_segments.drain(..overflow);
                }
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
            Message::MarqueeTick => {
                // Advance scroll offset 1px per tick (50px/sec at 20ms cadence).
                // Only overflow workspaces (name > WS_NAME_MAX_CHARS) scroll.
                let mut tasks: Vec<Task<Message>> = Vec::new();
                for ws in &self.workspaces {
                    if ws.name.chars().count() <= WS_NAME_MAX_CHARS {
                        continue;
                    }
                    let name_len = ws.name.chars().count();
                    let gap_len = WS_MARQUEE_GAP.chars().count();
                    let loop_px = (name_len + gap_len) as f32 * WS_MARQUEE_PX_PER_CHAR;
                    let offset = self.ws_marquee_offsets.entry(ws.name.clone()).or_insert(0.0);
                    *offset = (*offset + WS_MARQUEE_ADVANCE_PX) % loop_px;
                    let x = *offset;
                    let id = scrollable::Id::new(format!("ws-marquee-{}", ws.name));
                    tasks.push(scrollable::scroll_to(id, AbsoluteOffset { x, y: 0.0 }));
                }
                // Drop offsets for workspaces that no longer exist.
                let names: Vec<String> =
                    self.workspaces.iter().map(|w| w.name.clone()).collect();
                self.ws_marquee_offsets.retain(|k, _| names.contains(k));

                if tasks.is_empty() {
                    return Task::none();
                }
                return Task::batch(tasks);
            }
            Message::HostnameClicked => {
                // Portal-6.c: spawn the hostname-info tooltip popover.
                // Portal-6.b cross-peer cycling activates when mesh-home is live.
                return Task::perform(
                    async {
                        let _ = tokio::process::Command::new("mde-popover")
                            .arg("hostname-info")
                            .spawn();
                    },
                    |_| Message::Noop,
                );
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
            Message::HoverRunningGroup(key) => {
                self.hovered_running_group = Some(key);
            }
            Message::UnhoverRunningGroup => {
                self.hovered_running_group = None;
            }
            Message::WmClose(con_id) => {
                return Task::perform(
                    crate::workspace::wm_close(con_id),
                    |_| Message::Noop,
                );
            }
            Message::WmFloat(con_id) => {
                return Task::perform(
                    crate::workspace::wm_float_toggle(con_id),
                    |_| Message::Noop,
                );
            }
            Message::WmFull(con_id) => {
                return Task::perform(
                    crate::workspace::wm_fullscreen_toggle(con_id),
                    |_| Message::Noop,
                );
            }
            Message::WmMinimize(con_id) => {
                return Task::perform(
                    crate::workspace::wm_minimize(con_id),
                    |_| Message::Noop,
                );
            }
            Message::WmLayoutCycle(con_id) => {
                return Task::perform(
                    crate::workspace::wm_layout_cycle(con_id),
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
            Message::ClockClicked => {
                return Task::perform(
                    async {
                        let _ = tokio::process::Command::new("mde-popover")
                            .arg("clock")
                            .spawn();
                    },
                    |_| Message::Noop,
                );
            }
            Message::StatusZoneClicked => {
                return Task::perform(
                    async {
                        let _ = tokio::process::Command::new("mde-popover")
                            .arg("status")
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
            crate::workspace::mode_subscription(),
            crate::workspace::binding_subscription(),
            crate::workspace::bus_pulse_subscription(),
            crate::workspace::bus_announce_subscription(),
            crate::workspace::bus_clipboard_subscription(),
            crate::workspace::bus_peer_topics_subscription(),
            clock_subscription(),
            snapshot_subscription(),
            status_subscription(),
            marquee_subscription(),
        ])
    }

    fn view(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let bg = if theme == Theme::Dark { CHARCOAL } else { OFF_WHITE };
        let fg = if theme == Theme::Dark { Color::WHITE } else { Color::BLACK };

        let mode_seg = build_mode_segment(self);
        let ws_seg = build_workspace_segment(self, fg);
        let prev_ws_seg = build_prev_workspace_segment(self, fg);
        let layout_prompt_seg = build_layout_prompt_segment(self);
        let bus_announce_seg = build_bus_announce_segments(self);
        let host_seg = build_hostname_segment(self, fg);
        let running_zone = build_running_zone(self, fg);
        let status_seg = build_status_segment(self, fg);
        let clock_seg = build_clock_segment(self, fg);
        let nav_row = build_nav_row(self, fg);
        let wallpaper_strip = build_wallpaper_strip(self);

        container(
            row![
                mode_seg,
                ws_seg,
                prev_ws_seg,
                layout_prompt_seg,
                bus_announce_seg,
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

/// 20ms marquee tick subscription (Portal-5.b, 50px/sec).
fn marquee_subscription() -> Subscription<Message> {
    Subscription::run_with_id(
        "mde-portal-marquee",
        async_stream::stream! {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                yield Message::MarqueeTick;
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
/// Click spawns `mde-popover clock` — monthly calendar overlay (Portal-11.b).
fn build_clock_segment<'a>(app: &DockApp, fg: Color) -> Element<'a, Message> {
    use iced::widget::column;
    let time_str = app.clock_now.format("%H:%M").to_string();
    let date_str = app.clock_now.format("%b %d").to_string();

    mouse_area(
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
        .padding(Padding::from([0, 10])),
    )
    .on_press(Message::ClockClicked)
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

/// Portal-45 (R12-Q5) — far-left mode segment in the Dock breadcrumb.
///
/// Returns an empty zero-width space when sway is in the default
/// (no-mode) binding mode so the row layout stays flush. When a
/// non-default mode is active, renders `MODE: <name>` in cyan
/// against a translucent backdrop, anchored at the far-left of
/// the strip (left of mini-tree). Sway-internal modes (`resize`,
/// any operator-defined modes) keep the cyan default; per
/// Portal-47's tag-modes the segment will pick up the owning tag
/// color once that worker ships (the R12-Q21 tinting fallback is
/// cyan until then).
///
/// Mode names are truncated to 24 chars with `…` so a sway config
/// that names a mode something egregious doesn't blow out the
/// Dock row width.
fn build_mode_segment<'a>(app: &DockApp) -> Element<'a, Message> {
    let Some(mode_name) = app.current_sway_mode.as_deref() else {
        return iced::widget::Space::new(0.0, Length::Fill).into();
    };
    let display: String = if mode_name.chars().count() > 24 {
        let prefix: String = mode_name.chars().take(23).collect();
        format!("{prefix}…")
    } else {
        mode_name.to_string()
    };
    // Cyan tinted backdrop with the platform's translucency. The
    // exact cyan is the sway-internal default for binding modes.
    let cyan = Color { r: 0.0, g: 0.7, b: 0.85, a: 0.85 };
    container(
        text(format!("MODE: {display}"))
            .size(11.0)
            .color(Color::WHITE),
    )
    .style(move |_theme: &Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(cyan)),
        border: iced::Border {
            radius: iced::border::Radius::from(4.0),
            ..Default::default()
        },
        ..Default::default()
    })
    .height(Length::Fill)
    .align_y(iced::alignment::Vertical::Center)
    .padding(Padding::from([2, 8]))
    .into()
}

/// BUS-2.2.a — Dock breadcrumb segments for `fleet/announce`
/// notifications. Renders one priority-tinted pill per alive
/// segment in `DockApp::bus_announce_segments`. Text combines
/// title + body in `<title>: <body>` form (either may be None);
/// segments with neither title nor body collapse to the topic
/// label as a fallback.
///
/// Currently renders plain text; typewriter entry (R4-Q22 +
/// R4-Q28 tick audio) ships as BUS-2.2.typewriter when
/// Portal-14 typewriter primitive is available.
fn build_bus_announce_segments<'a>(app: &DockApp) -> Element<'a, Message> {
    let now = chrono::Local::now();
    let alive: Vec<&BusAnnounceSegment> = app
        .bus_announce_segments
        .iter()
        .filter(|s| is_bus_segment_alive(s, now))
        .collect();
    if alive.is_empty() {
        return iced::widget::Space::new(0.0, Length::Fill).into();
    }
    let mut pills: Vec<Element<'a, Message>> = Vec::new();
    for segment in alive {
        let label = format_bus_segment_label(segment);
        let tint = bus_segment_color(segment.topic, &segment.priority);
        pills.push(
            container(text(label).size(11.0).color(Color::WHITE))
                .style(move |_theme: &Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(tint)),
                    border: iced::Border {
                        radius: iced::border::Radius::from(4.0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Center)
                .padding(Padding::from([2, 8]))
                .into(),
        );
    }
    iced::widget::row(pills)
        .spacing(4)
        .align_y(iced::Alignment::Center)
        .into()
}

/// BUS-2.2.a: pure helper — build the segment's display label
/// from optional title + body. Falls back to the literal string
/// "fleet/announce" when neither is set (operator published an
/// empty announce). Long combined strings truncate to 60 chars
/// with `…` so the breadcrumb row doesn't blow out the Dock
/// width.
#[must_use]
pub fn format_bus_segment_label(segment: &BusAnnounceSegment) -> String {
    let combined = match (segment.title.as_deref(), segment.body.as_deref()) {
        (Some(t), Some(b)) if !t.is_empty() && !b.is_empty() => format!("{t}: {b}"),
        (Some(t), _) if !t.is_empty() => t.to_string(),
        (_, Some(b)) if !b.is_empty() => b.to_string(),
        _ => "fleet/announce".to_string(),
    };
    if combined.chars().count() > 60 {
        let prefix: String = combined.chars().take(59).collect();
        format!("{prefix}…")
    } else {
        combined
    }
}

/// Portal-50 (R12-Q11) — prompt-on-change layout banner segment.
///
/// Sits between the mini-tree and the hostname segment, similar
/// placement to Portal-43's previous-workspace cell. Renders only
/// when `app.layout_prompt` is `Some(state)` AND the TTL hasn't
/// expired yet. Shape: `Make <tag>? <new_layout> ✓ ✕` in the tag's
/// `group_color` (or the platform default when None).
///
/// Click ✓ → `Message::MakeTagDefaultLayout` writes the new layout
/// to tag.json + dismisses the banner.
/// Click ✕ → `Message::DismissLayoutPrompt` clears the banner only
/// (Portal-50.b will add the per-workspace override write).
///
/// Auto-dismiss happens on the next render after the TTL expires —
/// the 1 s clock subscription drives this implicitly within ~1 s
/// of the deadline.
fn build_layout_prompt_segment<'a>(app: &DockApp) -> Element<'a, Message> {
    let now = chrono::Local::now();
    let Some(state) = app.layout_prompt.as_ref() else {
        return iced::widget::Space::new(0.0, Length::Fill).into();
    };
    if !layout_prompt_visible(state, now) {
        return iced::widget::Space::new(0.0, Length::Fill).into();
    }
    // Background uses the platform-default Carbon-blue / indigo for
    // v1.0. The tag's `group_color` is preserved in state for a
    // future tint pass that mirrors Portal-56's border-tinting hex
    // → Color conversion; until that lands, all banners share the
    // platform accent (consistent + safe against malformed hex).
    let bg = COLOR_INDIGO;
    let _tag_color_for_future_tint = state.tag_color.as_deref();
    let tag_name = state.tag_name.clone();
    let layout = state.new_layout.clone();
    let prompt_label = format!("Make {tag_name} default? {layout}");
    let yes_btn: Element<'a, Message> = mouse_area(
        text("✓").size(13.0).color(Color::WHITE),
    )
    .on_press(Message::MakeTagDefaultLayout {
        tag_name: tag_name.clone(),
        layout: layout.clone(),
    })
    .into();
    let workspace_num = state.workspace_num;
    let layout_for_no = layout.clone();
    let no_btn: Element<'a, Message> = mouse_area(
        text("✕").size(13.0).color(Color::WHITE),
    )
    .on_press(Message::DeclineTagDefaultLayout {
        workspace_num,
        layout: layout_for_no,
    })
    .into();
    container(
        iced::widget::row![
            text(prompt_label).size(11.0).color(Color::WHITE),
            iced::widget::horizontal_space().width(Length::Fixed(8.0)),
            yes_btn,
            iced::widget::horizontal_space().width(Length::Fixed(4.0)),
            no_btn,
        ]
        .align_y(iced::Alignment::Center),
    )
    .style(move |_theme: &Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(bg)),
        border: iced::Border {
            radius: iced::border::Radius::from(4.0),
            ..Default::default()
        },
        ..Default::default()
    })
    .height(Length::Fill)
    .align_y(iced::alignment::Vertical::Center)
    .padding(Padding::from([2, 8]))
    .into()
}

/// Portal-43 (R12-Q3) — transient previous-workspace breadcrumb segment.
///
/// Sits between the mini-tree (`build_workspace_segment`) and the
/// hostname segment. When [`prev_workspace_segment_visible`] reports
/// `false` (no LRU entry, segment expired, etc.), renders an empty
/// zero-width space so the breadcrumb layout doesn't reflow.
///
/// When visible, renders the LRU front entry's auto-derived name
/// (Portal-41's `<num>: <app_id>`) in the platform's Carbon blue.
/// The full Round 12 design colors the segment with the owning-tag
/// color (R12-Q21) once Portal-56 ships the per-workspace tinting
/// worker — until then the fallback is the platform default. Click
/// jumps to that workspace via `swayipc workspace number <n>`.
fn build_prev_workspace_segment<'a>(app: &DockApp, fg: Color) -> Element<'a, Message> {
    let now = chrono::Local::now();
    if !prev_workspace_segment_visible(
        app.last_workspace_change,
        now,
        app.visited_workspaces_lru.len(),
    ) {
        return iced::widget::Space::new(0.0, Length::Fill).into();
    }
    let Some((prev_num, prev_name)) = app.visited_workspaces_lru.first().cloned() else {
        return iced::widget::Space::new(0.0, Length::Fill).into();
    };
    // Fallback color until Portal-56 ships per-tag tinting. Carbon
    // blue is the platform default workspace focus color, same as
    // `data/sway/config:60 client.focused`.
    let _ = fg; // kept for future tag-color fallback path
    let segment_color = COLOR_INDIGO;
    let truncated: String = if prev_name.chars().count() > 16 {
        let prefix: String = prev_name.chars().take(15).collect();
        format!("{prefix}…")
    } else {
        prev_name
    };
    mouse_area(
        container(
            text(format!("‹ {truncated}"))
                .size(11.0)
                .color(Color::WHITE),
        )
        .style(move |_theme: &Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(segment_color)),
            border: iced::Border {
                radius: iced::border::Radius::from(4.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .height(Length::Fill)
        .align_y(iced::alignment::Vertical::Center)
        .padding(Padding::from([2, 8])),
    )
    .on_press(Message::PrevWorkspaceClicked(prev_num))
    .into()
}

/// Build the workspace segment (Portal-5 / Portal-5.b): `[ws1›][ws2›][dev…›][+]`.
///
/// Each cell has the workspace label + inline `›` right-chevron (R4-Q63).
/// Short names (≤ 8 chars): static cell with shrink width, 24 px floor (R4-Q64).
/// Long names (> 8 chars): fixed 64 px cell with horizontal-scrollable marquee
/// at 50 px/sec; the name is duplicated with a gap for a seamless loop (Portal-5.b).
fn build_workspace_segment<'a>(app: &DockApp, fg: Color) -> Element<'a, Message> {
    let current_output: &str = app
        .workspaces
        .iter()
        .find(|w| w.focused)
        .map(|w| w.output.as_str())
        .unwrap_or("");

    let mut cells: Vec<Element<'a, Message>> = Vec::new();

    // Portal-59 (R12-Q24): workspace 99 is the platform's reserved
    // park slot for the minimize button. Filter it out of the
    // mini-tree so a parked window feels minimized to the operator.
    let now = chrono::Local::now();
    for ws in mini_tree_visible_workspaces(&app.workspaces) {
        let is_focused = ws.focused;
        let is_current_output = ws.output == current_output;
        let is_urgent = ws.urgent;
        let is_overflow = ws.name.chars().count() > WS_NAME_MAX_CHARS;
        // Portal-57.b (R12-Q22): does this workspace have an active
        // bus/mbadge/pulse event? If yes, the cell renders in
        // tier-red regardless of focus/visible state — the urgency
        // signal trumps the normal hierarchy.
        let pulse_con_id = active_pulse_con_id_for_workspace(
            ws.num,
            &app.recent_pulses,
            &app.running_windows,
            now,
        );
        let has_pulse = pulse_con_id.is_some();

        let text_color = if is_focused || has_pulse {
            Color::WHITE
        } else if is_current_output {
            fg
        } else {
            Color { a: 0.5, ..fg }
        };

        let cell_bg: Option<Color> = if has_pulse {
            // Portal-57.b: tier-red (R3-Q4 crit color). Slightly
            // brighter than the sway-internal `is_urgent` red so
            // operator can tell the Bus-driven pulse from sway's
            // own urgent hint when both fire.
            Some(Color::from_rgb(0.95, 0.2, 0.2))
        } else if is_urgent {
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

        let ws_name = ws.name.clone();

        let cell: Element<'a, Message> = if is_overflow {
            // Portal-5.b marquee: duplicate name with gap for seamless loop.
            let marquee_text = format!("{}{WS_MARQUEE_GAP}{}", ws.name, ws.name);
            let scroll_id = scrollable::Id::new(format!("ws-marquee-{}", ws.name));

            // Zero-height scrollbar — no visible UI chrome, just the clipping.
            let marquee = iced::widget::scrollable(
                text(marquee_text).size(12.0).color(text_color),
            )
            .id(scroll_id)
            .direction(Direction::Horizontal(
                Scrollbar::new().width(0.0).scroller_width(0.0),
            ))
            .width(WS_MARQUEE_CELL_PX)
            .height(Length::Fill);

            let cell_content = row![
                marquee,
                text("›").size(10.0).color(chevron_color),
            ]
            .spacing(2)
            .align_y(iced::Alignment::Center);

            mouse_area(
                container(cell_content)
                    .height(Length::Fill)
                    .align_y(iced::alignment::Vertical::Center)
                    .padding(Padding::from([0, 4]))
                    .style(move |_: &Theme| iced::widget::container::Style {
                        background: cell_bg.map(iced::Background::Color),
                        ..Default::default()
                    }),
            )
            .on_press(match pulse_con_id {
                Some(con_id) => Message::FocusWindowById(con_id),
                None => Message::FocusWorkspace(ws_name),
            })
            .into()
        } else {
            // Short name: static shrink-width cell (original Portal-5 design).
            let label = ws.display_label();
            let cell_content = row![
                text(label).size(12.0).color(text_color),
                text("›").size(10.0).color(chevron_color),
            ]
            .spacing(2)
            .align_y(iced::Alignment::Center);

            mouse_area(
                container(cell_content)
                    .width(Length::Shrink)
                    .height(Length::Fill)
                    .align_y(iced::alignment::Vertical::Center)
                    .padding(Padding::from([0, 8]))
                    .style(move |_: &Theme| iced::widget::container::Style {
                        background: cell_bg.map(iced::Background::Color),
                        ..Default::default()
                    }),
            )
            .on_press(match pulse_con_id {
                Some(con_id) => Message::FocusWindowById(con_id),
                None => Message::FocusWorkspace(ws_name),
            })
            .into()
        };

        cells.push(cell);
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

/// Build the running-zone segment (Portal-8.a / Portal-8.b, R3-Q15, R4-Q67–Q71).
///
/// Groups windows by `app_id`.  Each group is a cell showing:
///   - Short app label (first 10 chars of app_id, or title fallback)
///   - Workspace-number badge when the group spans multiple workspaces (R3-Q15)
///   - Count badge when there are 2+ windows in the group
///   - Indigo background when any window in the group is focused
///
/// Clicking the label area focuses the window (focused one or first in group).
/// Hovering the cell reveals 5 WM micro-buttons (Portal-8.b, R4-Q67–Q71):
///   `✕` close · `◱` float · `□` fullscreen · `↓` scratchpad · `≡` layout-cycle
/// Portal-59 (R12-Q24): pure filter that the running-zone renderer
/// applies before grouping by app_id. Hides windows parked at the
/// reserved workspace ([`PARKED_WORKSPACE_NUM`]). Exposed for tests.
fn running_zone_visible_windows(windows: &[WindowInfo]) -> impl Iterator<Item = &WindowInfo> {
    windows
        .iter()
        .filter(|w| w.workspace_num != PARKED_WORKSPACE_NUM)
}

/// Portal-43 (R12-Q3): pull `(num, name)` for the currently-focused
/// workspace, or `None` if no workspace is focused. The parked
/// workspace ([`PARKED_WORKSPACE_NUM`]) is excluded so a brief stop
/// there during `wm_minimize` doesn't push it into the LRU.
fn focused_workspace(workspaces: &[WorkspaceInfo]) -> Option<(i32, String)> {
    workspaces
        .iter()
        .find(|w| w.focused && w.num != PARKED_WORKSPACE_NUM)
        .map(|w| (w.num, w.name.clone()))
}

/// Portal-43 (R12-Q3): push the given workspace onto the LRU front
/// (newest at index 0). Adjacent duplicates (the workspace already
/// at index 0) collapse; oldest entry is dropped once the LRU
/// exceeds [`PREV_WS_LRU_CAP`].
fn push_visited_workspace(lru: &mut Vec<(i32, String)>, entry: (i32, String)) {
    if let Some(front) = lru.first() {
        if front.0 == entry.0 {
            // Refresh name in case Portal-41 rename arrived since
            // the entry was first captured.
            lru[0] = entry;
            return;
        }
    }
    lru.insert(0, entry);
    if lru.len() > PREV_WS_LRU_CAP {
        lru.truncate(PREV_WS_LRU_CAP);
    }
}

/// Portal-43 (R12-Q3): `true` if the transient previous-workspace
/// breadcrumb segment should be visible right now. Visible when
/// the LRU has at least one entry AND the most-recent focus change
/// is younger than [`PREV_WS_SEGMENT_TTL_SECS`] seconds.
fn prev_workspace_segment_visible(
    last_change: Option<chrono::DateTime<chrono::Local>>,
    now: chrono::DateTime<chrono::Local>,
    lru_len: usize,
) -> bool {
    if lru_len == 0 {
        return false;
    }
    match last_change {
        None => false,
        Some(t) => {
            let elapsed = now.signed_duration_since(t);
            elapsed.num_seconds() >= 0 && elapsed.num_seconds() < PREV_WS_SEGMENT_TTL_SECS
        }
    }
}

/// Portal-59 (R12-Q24): pure filter that the mini-tree renderer
/// applies. Drops negative-numbered slots (sway scratchpad meta) AND
/// the parked workspace. Exposed for tests.
fn mini_tree_visible_workspaces(workspaces: &[WorkspaceInfo]) -> impl Iterator<Item = &WorkspaceInfo> {
    workspaces
        .iter()
        .filter(|w| w.num >= 0 && w.num != PARKED_WORKSPACE_NUM)
}

/// Portal-50 (R12-Q11): pure helper — decide whether the binding
/// command should raise the prompt-on-change layout banner.
/// Returns `Some(LayoutPromptState)` when:
///   1. `command` parses as `layout <splith|splitv|tabbed|stacked>`.
///   2. The focused workspace is owned by a tag (Portal-18.a
///      tag-store membership).
///   3. The owning tag's `default_layout` differs from the new
///      layout (or is unset — counts as "different").
///
/// Returns `None` otherwise — natural no-op for layout-cycle binds
/// on untagged workspaces or tags that don't pin a default.
///
/// The function reads the tag store fresh per call. Portal-50 fires
/// at human-keystroke rate (a few per minute at peak), so the
/// extra JSON parse is bounded.
pub fn compute_layout_prompt(app: &DockApp, command: &str) -> Option<LayoutPromptState> {
    let new_layout = crate::workspace::parse_layout_command(command)?.to_string();
    let focused = app.workspaces.iter().find(|w| w.focused)?;
    if focused.num == PARKED_WORKSPACE_NUM {
        return None;
    }
    let store = mackes_mesh_types::TagStore::load_default().ok()?;
    let owning = store
        .tags
        .iter()
        .find(|t| {
            t.members.iter().any(|m| matches!(
                m,
                mackes_mesh_types::TagMember::Workspace { num } if *num == focused.num
            ))
        })?;
    let tag_default = owning.default_layout.as_deref().unwrap_or("");
    if tag_default == new_layout {
        return None;
    }
    Some(LayoutPromptState {
        workspace_num: focused.num,
        new_layout,
        tag_name: owning.name.clone(),
        tag_color: owning.group_color.clone(),
        spawned_at: chrono::Local::now(),
    })
}

/// Portal-50 (R12-Q11): pure helper — `true` if the layout-prompt
/// banner should still be visible at `now`. Visible while
/// `now - state.spawned_at < LAYOUT_PROMPT_TTL_SECS`.
#[must_use]
pub fn layout_prompt_visible(
    state: &LayoutPromptState,
    now: chrono::DateTime<chrono::Local>,
) -> bool {
    let elapsed = now.signed_duration_since(state.spawned_at);
    elapsed.num_seconds() >= 0 && elapsed.num_seconds() < LAYOUT_PROMPT_TTL_SECS
}

/// Portal-50 (R12-Q11): writes the tag's `default_layout` to disk
/// via `mackes_mesh_types::TagStore` atomic save. Returns an error
/// if the tag store can't be loaded or saved.
pub fn update_tag_default_layout(
    tag_name: &str,
    new_layout: &str,
) -> Result<(), mackes_mesh_types::TagStoreError> {
    let mut store = mackes_mesh_types::TagStore::load_default()?;
    if let Some(tag) = store.find_by_name_mut(tag_name) {
        tag.default_layout = Some(new_layout.to_string());
    }
    store.save_default()?;
    Ok(())
}

/// Portal-50.b (R12-Q11): writes a per-workspace layout override
/// to `<XDG_DATA_HOME>/mde/workspaces.json` via
/// `mackes_mesh_types::WorkspaceOverridesFile` atomic save.
/// Returns an error if the file can't be loaded or saved.
pub fn write_workspace_layout_override(
    workspace_num: i32,
    layout: &str,
) -> Result<(), mackes_mesh_types::OverridesError> {
    let mut overrides = mackes_mesh_types::WorkspaceOverridesFile::load_default()?;
    overrides.set_layout_override(workspace_num, layout);
    overrides.save_default()?;
    Ok(())
}

/// Portal-57.b (R12-Q22): pure helper — `true` if the pulse is
/// still within its TTL at `now`. Pulses past the TTL are swept
/// out on the next UrgentPulse message + on every render.
#[must_use]
pub fn is_pulse_alive(pulse: &UrgentPulse, now: chrono::DateTime<chrono::Local>) -> bool {
    let elapsed = now.signed_duration_since(pulse.spawned_at);
    elapsed.num_milliseconds() >= 0 && elapsed.num_milliseconds() < URGENT_PULSE_TTL_MS
}

/// Portal-57.b (R12-Q22): pure helper — resolve the workspace_num
/// for a pulse via the live `running_windows` snapshot. Returns
/// `None` when the pulse's con_id isn't in the current window
/// list (window closed before the pulse propagated, or the pulse
/// targets an off-tree container).
#[must_use]
pub fn pulse_workspace_num(
    pulse: &UrgentPulse,
    running_windows: &[WindowInfo],
) -> Option<i32> {
    running_windows
        .iter()
        .find(|w| w.con_id == pulse.con_id)
        .map(|w| w.workspace_num)
}

/// Portal-57.b (R12-Q22): pure helper — `true` if workspace
/// `ws_num` has any active pulse at `now`. Mini-tree cells use
/// this to decide whether to render in tier-red. Delegates to
/// [`active_pulse_con_id_for_workspace`] so the alive+match
/// logic lives in one place. Retained for downstream consumers
/// + the test-locked contract (the predicate form is easier to
/// read in `assert!`s than `Option::is_some`).
#[allow(dead_code)]
#[must_use]
pub fn workspace_has_active_pulse(
    ws_num: i32,
    pulses: &[UrgentPulse],
    running_windows: &[WindowInfo],
    now: chrono::DateTime<chrono::Local>,
) -> bool {
    active_pulse_con_id_for_workspace(ws_num, pulses, running_windows, now).is_some()
}

/// BUS-2.2.a: pure helper — `true` if the bus-announce segment
/// is still within its TTL at `now`. Expired segments are swept
/// out on the next BusAnnounceSegment message + on every render.
#[must_use]
pub fn is_bus_segment_alive(
    segment: &BusAnnounceSegment,
    now: chrono::DateTime<chrono::Local>,
) -> bool {
    let elapsed = now.signed_duration_since(segment.spawned_at);
    elapsed.num_seconds() >= 0 && elapsed.num_seconds() < BUS_ANNOUNCE_TTL_SECS
}

/// BUS-2.2.a: pure helper — map a priority class string
/// (`urgent` / `high` / `default` / anything else) to its
/// breadcrumb-pill tint Color. Unknown classes fall back to the
/// platform-default indigo.
#[must_use]
pub fn bus_priority_color(priority: &str) -> Color {
    match priority {
        // Carbon red 30 — urgent
        "urgent" => Color { r: 0.91, g: 0.30, b: 0.36, a: 1.0 },
        // Carbon orange 30 — high
        "high" => Color { r: 0.93, g: 0.55, b: 0.21, a: 1.0 },
        // Carbon blue 40 — default
        "default" => Color { r: 0.20, g: 0.69, b: 1.0, a: 1.0 },
        _ => COLOR_INDIGO,
    }
}

/// BUS-2.2.b: pure helper — select the breadcrumb-pill tint based
/// on the segment's source topic AND priority. `Clipboard` segments
/// always render neutral grey regardless of priority (per the design
/// lock — clipboard adds are visually distinct from notifications).
/// `Announce` segments delegate to `bus_priority_color`.
#[must_use]
pub fn bus_segment_color(topic: BusSegmentTopic, priority: &str) -> Color {
    match topic {
        BusSegmentTopic::Clipboard => {
            // Carbon grey 50 — neutral, distinct from any priority
            // color so clipboard adds don't compete for attention.
            Color { r: 0.55, g: 0.55, b: 0.55, a: 1.0 }
        }
        BusSegmentTopic::Announce => bus_priority_color(priority),
    }
}

/// Portal-57.b.click (R12-Q22): pure helper — return the con_id of
/// the most-recent active pulse on workspace `ws_num`, if any.
/// Click on a pulsing cell focuses this con_id directly via
/// swayipc rather than just the workspace, so the operator lands
/// on the urgent window. Returns `None` when no active pulse maps
/// to the workspace; the caller falls back to `Message::FocusWorkspace`.
#[must_use]
pub fn active_pulse_con_id_for_workspace(
    ws_num: i32,
    pulses: &[UrgentPulse],
    running_windows: &[WindowInfo],
    now: chrono::DateTime<chrono::Local>,
) -> Option<i64> {
    pulses
        .iter()
        .rev() // newest-first → click targets the latest urgent
        .find(|p| {
            is_pulse_alive(p, now)
                && pulse_workspace_num(p, running_windows) == Some(ws_num)
        })
        .map(|p| p.con_id)
}

/// Portal-49 (R12-Q9): pure helper — return the first taxonomy mark
/// from a list. Operator marks that don't match the Portal-48
/// taxonomy are skipped so the pill only renders for auto-marks.
/// Returns `None` when no taxonomy mark is present.
fn first_taxonomy_mark(marks: &[String]) -> Option<&str> {
    marks.iter().find_map(|m| match m.as_str() {
        "editor" | "web" | "shell" | "mail" | "chat" => Some(m.as_str()),
        _ => None,
    })
}

/// Portal-49 (R12-Q9): pure helper — convert a taxonomy mark name
/// to its pill color. Unknown marks return `None` (no pill rendered).
/// Color values lifted verbatim from the R12-Q9 design lock; the
/// pill stays opaque so it reads cleanly on every Dock background.
fn mark_pill_color(mark: &str) -> Option<Color> {
    match mark {
        // editor=#42be65 — Carbon green 40
        "editor" => Some(Color { r: 0.259, g: 0.745, b: 0.396, a: 1.0 }),
        // web=#33b1ff — Carbon blue 40
        "web" => Some(Color { r: 0.200, g: 0.694, b: 1.000, a: 1.0 }),
        // shell=#8d8d8d — Carbon grey 50
        "shell" => Some(Color { r: 0.553, g: 0.553, b: 0.553, a: 1.0 }),
        // mail=#ff8389 — Carbon red 30
        "mail" => Some(Color { r: 1.000, g: 0.514, b: 0.537, a: 1.0 }),
        // chat=#be95ff — Carbon purple 30
        "chat" => Some(Color { r: 0.745, g: 0.584, b: 1.000, a: 1.0 }),
        _ => None,
    }
}

/// Portal-49 (R12-Q9): render the 8 px mark pill for a running-zone
/// card. Returns `None` if the window has no taxonomy mark. The pill
/// is a square 8×8 px (rounded to a full-radius circle) so it reads
/// at Dock-scale without competing with the label or WM micro-button
/// cluster.
fn mark_pill_element<'a>(color: Color) -> Element<'a, Message> {
    container(iced::widget::Space::new(0.0, 0.0))
        .style(move |_theme: &Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(color)),
            border: iced::Border {
                radius: iced::border::Radius::from(4.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .width(Length::Fixed(8.0))
        .height(Length::Fixed(8.0))
        .into()
}

fn build_running_zone<'a>(app: &DockApp, fg: Color) -> Element<'a, Message> {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, Vec<&WindowInfo>> = BTreeMap::new();
    // Portal-59 (R12-Q24): hide windows parked at workspace 99 — they
    // read as minimized to the operator. Mod+Shift+m un-parks them
    // back into the focused workspace.
    for w in running_zone_visible_windows(&app.running_windows) {
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
        let is_hovered =
            app.hovered_running_group.as_deref() == Some(label_key.as_str());

        // Best window to focus on click: the focused one or the first.
        let target_window: &&WindowInfo = group
            .iter()
            .find(|w| w.focused)
            .unwrap_or(&&group[0]);
        let target_id = target_window.con_id;

        // Portal-49 (R12-Q9): pull the taxonomy mark from the
        // target window. Operator marks outside the taxonomy are
        // skipped via `first_taxonomy_mark` so the pill only
        // renders for Portal-48 auto-marks.
        let mark_pill: Option<Element<'a, Message>> = first_taxonomy_mark(&target_window.marks)
            .and_then(mark_pill_color)
            .map(mark_pill_element);

        // Workspace numbers in this group (for multi-WS badge).
        let mut ws_nums: Vec<i32> = group.iter().map(|w| w.workspace_num).collect();
        ws_nums.dedup();

        let bg: Option<Color> = if any_focused { Some(COLOR_INDIGO) } else { None };
        let text_color = if any_focused { Color::WHITE } else { fg };

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

        if count > 1 {
            label_parts.push(
                container(text(count.to_string()).size(8.0).color(Color::WHITE))
                    .style(|_: &Theme| iced::widget::container::Style {
                        background: Some(iced::Background::Color(Color {
                            r: 0.5, g: 0.5, b: 0.6, a: 1.0,
                        })),
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

        if ws_nums.len() > 1 {
            label_parts.push(
                text("·").size(8.0).color(Color { a: 0.5, ..text_color }).into(),
            );
        }

        // Portal-49 (R12-Q9): inline taxonomy-mark pill (8 × 8 px,
        // rounded). Appears immediately after the multi-WS dot
        // indicator so the visual order is `<label> <count?> <ws-dot?>
        // <mark-pill?>`. The pill only renders for windows the
        // Portal-48 auto-mark daemon has classified.
        if let Some(pill) = mark_pill {
            label_parts.push(pill);
        }

        // Label section: clickable for focus; left padding carries the bg indent.
        let label_section: Element<'a, Message> = mouse_area(
            container(row(label_parts).spacing(2).align_y(iced::Alignment::Center))
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Center)
                .padding(Padding::from([0, 6])),
        )
        .on_press(Message::FocusWindowById(target_id))
        .into();

        // WM micro-buttons — revealed on hover (Portal-8.b).
        let btn_color = Color { r: 0.85, g: 0.87, b: 0.95, a: 0.90 };
        let wm_section: Element<'a, Message> = if is_hovered {
            row![
                wm_micro_btn("✕", btn_color, Message::WmClose(target_id)),
                wm_micro_btn("◱", btn_color, Message::WmFloat(target_id)),
                wm_micro_btn("□", btn_color, Message::WmFull(target_id)),
                wm_micro_btn("↓", btn_color, Message::WmMinimize(target_id)),
                wm_micro_btn("≡", btn_color, Message::WmLayoutCycle(target_id)),
            ]
            .spacing(1)
            .align_y(iced::Alignment::Center)
            .height(Length::Fill)
            .padding(Padding::from([0, 3]))
            .into()
        } else {
            iced::widget::Space::new(0.0, Length::Fill).into()
        };

        // Outer container: applies the bg (indigo when focused, transparent when not).
        let cell = container(
            row![label_section, wm_section]
                .spacing(0)
                .height(Length::Fill)
                .align_y(iced::Alignment::Center),
        )
        .height(Length::Fill)
        .style(move |_: &Theme| iced::widget::container::Style {
            background: bg.map(iced::Background::Color),
            ..Default::default()
        });

        // Outer mouse_area: manages hover state only (no on_press here so WM
        // buttons don't double-fire with the label's FocusWindowById handler).
        cells.push(
            mouse_area(cell)
                .on_enter(Message::HoverRunningGroup(label_key.clone()))
                .on_exit(Message::UnhoverRunningGroup)
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

/// Tiny WM action button for the running-zone hover overlay (Portal-8.b).
///
/// Outer mouse_area of the group cell does NOT carry an `on_press`, so only
/// this button's own press handler fires when clicked.
fn wm_micro_btn<'a>(glyph: &'static str, color: Color, msg: Message) -> Element<'a, Message> {
    mouse_area(
        container(text(glyph).size(10.0).color(color))
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Center)
            .padding(Padding::from([0, 3])),
    )
    .on_press(msg)
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

    // ── Volume (clickable → mde-popover status, Portal-9.b) ──────────────────
    let vol_glyph = resolve_icon(Icon::Sound, IconSize::Inline).fallback_glyph;
    items.push(
        mouse_area(
            container(text(vol_glyph).size(11.0).color(Color { a: 0.55, ..fg }))
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Center)
                .padding(Padding::from([0, 4])),
        )
        .on_press(Message::StatusZoneClicked)
        .into(),
    );

    // ── Brightness (clickable → mde-popover status, Portal-9.b) ─────────────
    if let Some(bri) = si.brightness_pct {
        let bri_glyph = resolve_icon(Icon::Display, IconSize::Inline).fallback_glyph;
        let label = format!("{bri_glyph}{bri}%");
        items.push(
            mouse_area(
                container(text(label).size(10.0).color(Color { a: 0.6, ..fg }))
                    .height(Length::Fill)
                    .align_y(iced::alignment::Vertical::Center)
                    .padding(Padding::from([0, 4])),
            )
            .on_press(Message::StatusZoneClicked)
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
    fn clock_clicked_fires_task_without_panic() {
        let mut app = DockApp::default();
        // ClockClicked spawns mde-popover clock; state itself is unchanged.
        let _task =
            iced_layershell::Application::update(&mut app, Message::ClockClicked);
        assert_eq!(app.active_nav, None);
    }

    #[test]
    fn status_zone_clicked_fires_task_without_panic() {
        let mut app = DockApp::default();
        // StatusZoneClicked spawns mde-popover status; state itself unchanged.
        let _task =
            iced_layershell::Application::update(&mut app, Message::StatusZoneClicked);
        assert_eq!(app.active_nav, None);
    }

    #[test]
    fn hostname_clicked_fires_task_without_panic() {
        let mut app = DockApp::default();
        // HostnameClicked spawns mde-popover hostname-info; state itself unchanged.
        let _task =
            iced_layershell::Application::update(&mut app, Message::HostnameClicked);
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
            marks: Vec::new(),
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

    // ── Portal-5.b marquee tests ──────────────────────────────────────────────

    #[test]
    fn ws_marquee_offsets_start_empty() {
        let app = DockApp::default();
        assert!(app.ws_marquee_offsets.is_empty());
    }

    #[test]
    fn marquee_tick_noop_when_no_overflow_workspaces() {
        let mut app = DockApp::default();
        // Only short-name workspaces — no overflow.
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::WorkspaceList(vec![make_ws(1, "1", true, true, "eDP-1")]),
        );
        let _task = iced_layershell::Application::update(&mut app, Message::MarqueeTick);
        // No offsets should be created for short names.
        assert!(app.ws_marquee_offsets.is_empty());
    }

    #[test]
    fn marquee_tick_creates_and_advances_offset_for_overflow_workspace() {
        let mut app = DockApp::default();
        let long_name = "my-very-long-project"; // > 8 chars → overflow
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::WorkspaceList(vec![make_ws(1, long_name, true, true, "eDP-1")]),
        );
        // First tick: offset initialised at 0, then advanced by ADVANCE_PX.
        let _task = iced_layershell::Application::update(&mut app, Message::MarqueeTick);
        let offset = app.ws_marquee_offsets.get(long_name).copied().unwrap_or(-1.0);
        assert!(
            (offset - WS_MARQUEE_ADVANCE_PX).abs() < f32::EPSILON,
            "offset should equal ADVANCE_PX after first tick, got {offset}"
        );
    }

    #[test]
    fn marquee_offset_wraps_at_loop_width() {
        let mut app = DockApp::default();
        let name = "long-workspace"; // 14 chars → loop_width = (14 + 3) * 7.2 = 122.4 px
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::WorkspaceList(vec![make_ws(2, name, true, true, "eDP-1")]),
        );
        let gap_len = WS_MARQUEE_GAP.chars().count();
        let loop_px = (name.chars().count() + gap_len) as f32 * WS_MARQUEE_PX_PER_CHAR;
        // Seed offset just below the loop boundary.
        app.ws_marquee_offsets.insert(name.to_string(), loop_px - 0.5);
        let _task = iced_layershell::Application::update(&mut app, Message::MarqueeTick);
        let offset = app.ws_marquee_offsets.get(name).copied().unwrap_or(-1.0);
        assert!(
            offset < loop_px,
            "offset {offset} should have wrapped below loop_px {loop_px}"
        );
    }

    #[test]
    fn marquee_tick_cleans_up_removed_workspaces() {
        let mut app = DockApp::default();
        let name = "long-workspace";
        // Seed an offset for a workspace that is now gone.
        app.ws_marquee_offsets.insert(name.to_string(), 10.0);
        // Workspace list no longer contains that name.
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::WorkspaceList(vec![make_ws(1, "1", true, true, "eDP-1")]),
        );
        let _task = iced_layershell::Application::update(&mut app, Message::MarqueeTick);
        assert!(
            !app.ws_marquee_offsets.contains_key(name),
            "stale offset should be cleaned up after workspace removed"
        );
    }

    // ── Portal-8.b WM-buttons-on-hover tests ─────────────────────────────────

    #[test]
    fn hovered_running_group_starts_none() {
        let app = DockApp::default();
        assert!(app.hovered_running_group.is_none());
    }

    #[test]
    fn hover_running_group_sets_key() {
        let mut app = DockApp::default();
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::HoverRunningGroup("foot".to_string()),
        );
        assert_eq!(app.hovered_running_group.as_deref(), Some("foot"));
    }

    #[test]
    fn unhover_running_group_clears_key() {
        let mut app = DockApp::default();
        app.hovered_running_group = Some("foot".to_string());
        let _ = iced_layershell::Application::update(&mut app, Message::UnhoverRunningGroup);
        assert!(app.hovered_running_group.is_none());
    }

    #[test]
    fn hover_then_hover_different_group_switches_key() {
        let mut app = DockApp::default();
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::HoverRunningGroup("foot".to_string()),
        );
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::HoverRunningGroup("firefox".to_string()),
        );
        assert_eq!(app.hovered_running_group.as_deref(), Some("firefox"));
    }

    #[test]
    fn wm_close_fires_task_without_panic() {
        let mut app = DockApp::default();
        let _task = iced_layershell::Application::update(&mut app, Message::WmClose(42));
    }

    #[test]
    fn wm_float_fires_task_without_panic() {
        let mut app = DockApp::default();
        let _task = iced_layershell::Application::update(&mut app, Message::WmFloat(42));
    }

    #[test]
    fn wm_full_fires_task_without_panic() {
        let mut app = DockApp::default();
        let _task = iced_layershell::Application::update(&mut app, Message::WmFull(42));
    }

    #[test]
    fn wm_minimize_fires_task_without_panic() {
        let mut app = DockApp::default();
        let _task =
            iced_layershell::Application::update(&mut app, Message::WmMinimize(42));
    }

    // ── Portal-59 (R12-Q24) scratchpad-retirement filter tests ──────────────

    /// Three windows on three different workspaces, all parked at 99:
    /// the running-zone input collapses to zero. Mirrors the bench
    /// acceptance "park three windows from three different workspaces
    /// → minified-zone shows none."
    #[test]
    fn parked_windows_filtered_from_running_zone_input() {
        let windows = vec![
            make_window(101, "firefox", PARKED_WORKSPACE_NUM, false),
            make_window(102, "foot", PARKED_WORKSPACE_NUM, false),
            make_window(103, "helix", PARKED_WORKSPACE_NUM, false),
        ];
        let visible: Vec<&WindowInfo> = running_zone_visible_windows(&windows).collect();
        assert!(
            visible.is_empty(),
            "windows on the parked workspace must not appear in the running-zone"
        );
    }

    /// Mixed input — two parked, one live — collapses to the live one.
    /// Locks the filter against accidentally dropping non-parked windows.
    #[test]
    fn live_windows_pass_through_running_zone_filter() {
        let windows = vec![
            make_window(101, "firefox", PARKED_WORKSPACE_NUM, false),
            make_window(102, "foot", 1, true),
            make_window(103, "helix", PARKED_WORKSPACE_NUM, false),
        ];
        let visible: Vec<&WindowInfo> = running_zone_visible_windows(&windows).collect();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].con_id, 102);
        assert_eq!(visible[0].app_id.as_deref(), Some("foot"));
    }

    /// The mini-tree must also hide the parked workspace itself, so
    /// an operator with parked windows doesn't see the `99` cell
    /// stranded at the right edge of the workspace strip.
    #[test]
    fn parked_workspace_filtered_from_mini_tree() {
        let workspaces = vec![
            make_ws(1, "1: firefox", true, true, "eDP-1"),
            make_ws(2, "2", false, false, "eDP-1"),
            make_ws(PARKED_WORKSPACE_NUM, "99", false, false, "eDP-1"),
        ];
        let visible: Vec<&WorkspaceInfo> = mini_tree_visible_workspaces(&workspaces).collect();
        let nums: Vec<i32> = visible.iter().map(|w| w.num).collect();
        assert_eq!(nums, vec![1, 2], "parked workspace must not appear");
    }

    /// Scratchpad meta-workspaces (sway's internal `-1` for windows
    /// in the actual scratchpad — Portal-full uses these) STILL get
    /// filtered, locking the pre-existing mini-tree contract that
    /// negative workspace numbers stay hidden.
    #[test]
    fn negative_meta_workspaces_still_filtered() {
        let workspaces = vec![
            make_ws(-1, "__internal__", false, false, ""),
            make_ws(1, "1", true, true, "eDP-1"),
        ];
        let visible: Vec<&WorkspaceInfo> = mini_tree_visible_workspaces(&workspaces).collect();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].num, 1);
    }

    // ── Portal-43 (R12-Q3) previous-workspace breadcrumb tests ──────────────

    /// Mirrors the bench acceptance:
    /// "focus ws1 → focus ws2 → previous-segment shows `1: firefox`".
    #[test]
    fn workspace_focus_change_pushes_old_onto_lru() {
        let mut app = DockApp::default();
        // Seed: ws1 focused, named "1: firefox".
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::WorkspaceList(vec![make_ws(1, "1: firefox", true, true, "eDP-1")]),
        );
        assert!(app.visited_workspaces_lru.is_empty(), "no LRU push on initial focus");
        // Focus changes to ws2.
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::WorkspaceList(vec![
                make_ws(1, "1: firefox", false, false, "eDP-1"),
                make_ws(2, "2", true, true, "eDP-1"),
            ]),
        );
        assert_eq!(app.visited_workspaces_lru.len(), 1);
        assert_eq!(app.visited_workspaces_lru[0], (1, "1: firefox".to_string()));
        assert!(app.last_workspace_change.is_some());
    }

    /// Adjacent duplicates collapse — focusing the same workspace twice
    /// in a row doesn't push twice. Locks the `push_visited_workspace`
    /// dedup contract.
    #[test]
    fn lru_collapses_adjacent_duplicates() {
        let mut lru: Vec<(i32, String)> = Vec::new();
        push_visited_workspace(&mut lru, (1, "1: foot".into()));
        push_visited_workspace(&mut lru, (1, "1: foot".into()));
        push_visited_workspace(&mut lru, (1, "1: foot".into()));
        assert_eq!(lru.len(), 1);
    }

    /// LRU cap: pushing more than [`PREV_WS_LRU_CAP`] entries drops the
    /// oldest off the back.
    #[test]
    fn lru_truncates_to_cap() {
        let mut lru: Vec<(i32, String)> = Vec::new();
        for i in 1..=7 {
            push_visited_workspace(&mut lru, (i, format!("{i}")));
        }
        assert_eq!(lru.len(), PREV_WS_LRU_CAP);
        // Newest stays at front, oldest dropped.
        assert_eq!(lru[0].0, 7);
        let oldest_kept = lru.last().unwrap().0;
        assert!(oldest_kept >= 7 - PREV_WS_LRU_CAP as i32 + 1);
    }

    /// Segment visibility: hidden when LRU is empty; visible right after
    /// a push; hidden after the TTL expires.
    #[test]
    fn previous_workspace_segment_respects_ttl() {
        let now = chrono::Local::now();
        // Empty LRU → always hidden.
        assert!(!prev_workspace_segment_visible(Some(now), now, 0));
        // Fresh push → visible.
        assert!(prev_workspace_segment_visible(Some(now), now, 1));
        // 4 s old → still visible.
        let four_s_ago = now - chrono::Duration::seconds(4);
        assert!(prev_workspace_segment_visible(Some(four_s_ago), now, 1));
        // 6 s old → expired.
        let six_s_ago = now - chrono::Duration::seconds(6);
        assert!(!prev_workspace_segment_visible(Some(six_s_ago), now, 1));
        // Never-focused → hidden.
        assert!(!prev_workspace_segment_visible(None, now, 1));
    }

    /// Click handler fires `PrevWorkspaceClicked(num)` → produces a
    /// Task without panicking even when sway isn't running.
    #[test]
    fn prev_workspace_clicked_fires_task_without_panic() {
        let mut app = DockApp::default();
        let _task =
            iced_layershell::Application::update(&mut app, Message::PrevWorkspaceClicked(1));
    }

    // ── Portal-45 (R12-Q5) mode-segment tests ──────────────────────────────

    /// Sway emits `default` when a binding mode exits — the Dock must
    /// translate that to `None` so the segment disappears.
    #[test]
    fn mode_default_clears_segment_state() {
        let mut app = DockApp::default();
        // Simulate sway emitting a non-default mode first.
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::ModeChanged(crate::workspace::mode_change_to_message("resize")),
        );
        assert_eq!(app.current_sway_mode.as_deref(), Some("resize"));
        // Default mode → segment clears.
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::ModeChanged(crate::workspace::mode_change_to_message("default")),
        );
        assert!(app.current_sway_mode.is_none());
    }

    /// Non-default mode names round-trip through the pure mapper and
    /// land in DockApp state. Mirrors the bench acceptance "enter
    /// resize mode via legacy binding → segment appears."
    #[test]
    fn mode_entered_stores_name() {
        let mut app = DockApp::default();
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::ModeChanged(crate::workspace::mode_change_to_message("resize")),
        );
        assert_eq!(app.current_sway_mode.as_deref(), Some("resize"));
        // Switching to a different mode replaces the name in place.
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::ModeChanged(crate::workspace::mode_change_to_message("Dev")),
        );
        assert_eq!(app.current_sway_mode.as_deref(), Some("Dev"));
    }

    // ── Portal-49 (R12-Q9) mark-pill tests ────────────────────────────────

    /// All five taxonomies map to a renderable pill color.
    #[test]
    fn mark_pill_color_covers_all_taxonomies() {
        for mark in ["editor", "web", "shell", "mail", "chat"] {
            assert!(
                mark_pill_color(mark).is_some(),
                "missing pill color for {mark}"
            );
        }
    }

    /// Unknown marks return None — no pill rendered.
    #[test]
    fn mark_pill_color_unknown_returns_none() {
        assert!(mark_pill_color("unknown").is_none());
        assert!(mark_pill_color("").is_none());
        assert!(mark_pill_color("WEB").is_none()); // case-sensitive
    }

    /// `first_taxonomy_mark` returns the FIRST taxonomy mark in the list.
    /// Operator marks outside the taxonomy are skipped so the pill only
    /// shows for Portal-48 auto-marks.
    #[test]
    fn first_taxonomy_mark_picks_first_match() {
        // Auto-marked editor → pill renders editor.
        let marks = vec!["editor".to_string()];
        assert_eq!(first_taxonomy_mark(&marks), Some("editor"));
        // Operator-marked "work" + auto-marked "editor" → pill renders editor.
        let marks = vec!["work".to_string(), "editor".to_string()];
        assert_eq!(first_taxonomy_mark(&marks), Some("editor"));
        // Operator-only marks → no pill.
        let marks = vec!["work".to_string(), "side-project".to_string()];
        assert_eq!(first_taxonomy_mark(&marks), None);
        // Empty marks → no pill.
        let marks: Vec<String> = Vec::new();
        assert_eq!(first_taxonomy_mark(&marks), None);
    }

    /// `mode_change_to_message` pure-function contract: `default` → None
    /// for ANY casing? Spec says exact match. Lock the contract.
    #[test]
    fn mode_change_to_message_only_lowercase_default_clears() {
        use crate::workspace::mode_change_to_message;
        assert_eq!(mode_change_to_message("default"), None);
        // Sway never emits non-lowercase "default", but if an
        // operator names a mode "Default" / "DEFAULT" it's a
        // legitimate non-default mode.
        assert_eq!(mode_change_to_message("Default"), Some("Default".to_string()));
        assert_eq!(mode_change_to_message("DEFAULT"), Some("DEFAULT".to_string()));
        assert_eq!(mode_change_to_message(""), Some("".to_string()));
        assert_eq!(mode_change_to_message("resize"), Some("resize".to_string()));
    }

    // ── Portal-50 (R12-Q11) prompt-on-change layout banner tests ──────────

    fn make_layout_prompt_state(workspace_num: i32, layout: &str) -> LayoutPromptState {
        LayoutPromptState {
            workspace_num,
            new_layout: layout.to_string(),
            tag_name: "Dev".to_string(),
            tag_color: Some("#42be65".to_string()),
            spawned_at: chrono::Local::now(),
        }
    }

    /// Banner visible at spawn-time + within TTL; auto-dismiss after.
    #[test]
    fn layout_prompt_respects_ttl() {
        let now = chrono::Local::now();
        let state = make_layout_prompt_state(1, "tabbed");
        assert!(layout_prompt_visible(&state, now));
        // 4 s in → still visible.
        let four_s = now + chrono::Duration::seconds(4);
        assert!(layout_prompt_visible(&state, four_s));
        // 9 s in → expired (TTL is 8 s).
        let nine_s = now + chrono::Duration::seconds(9);
        assert!(!layout_prompt_visible(&state, nine_s));
    }

    /// `DismissLayoutPrompt` clears the banner.
    #[test]
    fn dismiss_message_clears_banner() {
        let mut app = DockApp::default();
        app.layout_prompt = Some(make_layout_prompt_state(1, "tabbed"));
        let _ = iced_layershell::Application::update(&mut app, Message::DismissLayoutPrompt);
        assert!(app.layout_prompt.is_none());
    }

    /// `MakeTagDefaultLayout` clears the banner (the tag-store
    /// update may fail when tag.json doesn't exist; the banner
    /// still dismisses).
    #[test]
    fn make_default_clears_banner() {
        let mut app = DockApp::default();
        app.layout_prompt = Some(make_layout_prompt_state(1, "tabbed"));
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::MakeTagDefaultLayout {
                tag_name: "Dev".to_string(),
                layout: "tabbed".to_string(),
            },
        );
        assert!(app.layout_prompt.is_none());
    }

    // ── Portal-57.b (R12-Q22) mini-tree pulse tests ─────────────────────────

    fn make_urgent_pulse(con_id: i64, app_id: &str) -> UrgentPulse {
        UrgentPulse {
            ulid: "01HZAZTESTULIDXYZ012345678".to_string(),
            app_id: app_id.to_string(),
            con_id,
            spawned_at: chrono::Local::now(),
        }
    }

    /// Pulse is alive within TTL; expires after.
    #[test]
    fn pulse_lifetime_respects_ttl() {
        let pulse = make_urgent_pulse(42, "foot");
        let now = pulse.spawned_at;
        assert!(is_pulse_alive(&pulse, now));
        // 500 ms in → still alive.
        let halfway = now + chrono::Duration::milliseconds(500);
        assert!(is_pulse_alive(&pulse, halfway));
        // 1500 ms in → expired (TTL is 1200).
        let past = now + chrono::Duration::milliseconds(1500);
        assert!(!is_pulse_alive(&pulse, past));
    }

    /// `pulse_workspace_num` looks up the con_id via running_windows.
    #[test]
    fn pulse_workspace_num_resolves_via_window_list() {
        let windows = vec![
            make_window(42, "foot", 3, false),
            make_window(99, "firefox", 1, true),
        ];
        let pulse = make_urgent_pulse(42, "foot");
        assert_eq!(pulse_workspace_num(&pulse, &windows), Some(3));
        // Pulse with no matching con_id returns None.
        let missing = make_urgent_pulse(7777, "ghost");
        assert!(pulse_workspace_num(&missing, &windows).is_none());
    }

    /// `workspace_has_active_pulse` returns true only when at least
    /// one alive pulse maps to that workspace.
    #[test]
    fn workspace_has_active_pulse_combines_alive_check_and_lookup() {
        let windows = vec![make_window(42, "foot", 3, false)];
        let pulse = make_urgent_pulse(42, "foot");
        let pulses = vec![pulse.clone()];
        let now = pulse.spawned_at;
        // Workspace 3 has the pulse.
        assert!(workspace_has_active_pulse(3, &pulses, &windows, now));
        // Workspace 1 doesn't.
        assert!(!workspace_has_active_pulse(1, &pulses, &windows, now));
        // Expired pulse → no.
        let future = now + chrono::Duration::milliseconds(2000);
        assert!(!workspace_has_active_pulse(3, &pulses, &windows, future));
        // No matching con_id in windows → no.
        let empty_windows: Vec<WindowInfo> = Vec::new();
        assert!(!workspace_has_active_pulse(3, &pulses, &empty_windows, now));
    }

    // ── BUS-2.2.a Dock breadcrumb tests ─────────────────────────────────────

    fn make_bus_segment(priority: &str, title: Option<&str>, body: Option<&str>) -> BusAnnounceSegment {
        BusAnnounceSegment {
            ulid: "01HZAZTESTBUSULID12345".to_string(),
            priority: priority.to_string(),
            title: title.map(String::from),
            body: body.map(String::from),
            spawned_at: chrono::Local::now(),
            topic: BusSegmentTopic::Announce,
        }
    }

    fn make_clipboard_segment(title: Option<&str>, body: Option<&str>) -> BusAnnounceSegment {
        BusAnnounceSegment {
            ulid: "01HZAZTESTCLIPULID12345".to_string(),
            priority: "default".to_string(),
            title: title.map(String::from),
            body: body.map(String::from),
            spawned_at: chrono::Local::now(),
            topic: BusSegmentTopic::Clipboard,
        }
    }

    /// Segment alive within TTL; expires after.
    #[test]
    fn bus_segment_lifetime_respects_ttl() {
        let segment = make_bus_segment("default", Some("hi"), Some("everyone"));
        let now = segment.spawned_at;
        assert!(is_bus_segment_alive(&segment, now));
        // 4 s in → still alive.
        let later = now + chrono::Duration::seconds(4);
        assert!(is_bus_segment_alive(&segment, later));
        // 7 s in → expired (TTL is 6 s).
        let past = now + chrono::Duration::seconds(7);
        assert!(!is_bus_segment_alive(&segment, past));
    }

    /// Priority → tint lookup covers all four locked classes +
    /// falls back to indigo for anything else.
    #[test]
    fn bus_priority_color_locks_four_classes_plus_fallback() {
        assert_eq!(
            bus_priority_color("urgent"),
            Color { r: 0.91, g: 0.30, b: 0.36, a: 1.0 }
        );
        assert_eq!(
            bus_priority_color("high"),
            Color { r: 0.93, g: 0.55, b: 0.21, a: 1.0 }
        );
        assert_eq!(
            bus_priority_color("default"),
            Color { r: 0.20, g: 0.69, b: 1.0, a: 1.0 }
        );
        // Unknown priority → indigo fallback.
        assert_eq!(bus_priority_color("unknown"), COLOR_INDIGO);
        // `min` should never reach the renderer (filtered at
        // parse), but the lookup tolerates it via the fallback.
        assert_eq!(bus_priority_color("min"), COLOR_INDIGO);
    }

    /// `format_bus_segment_label` combines title + body when both
    /// present; falls back gracefully for partial / empty inputs;
    /// truncates long combined strings.
    #[test]
    fn format_bus_segment_label_handles_combinations() {
        let with_both = make_bus_segment("default", Some("Backup"), Some("complete"));
        assert_eq!(format_bus_segment_label(&with_both), "Backup: complete");

        let title_only = make_bus_segment("default", Some("Deploy started"), None);
        assert_eq!(format_bus_segment_label(&title_only), "Deploy started");

        let body_only = make_bus_segment("default", None, Some("alert message"));
        assert_eq!(format_bus_segment_label(&body_only), "alert message");

        let empty = make_bus_segment("default", None, None);
        assert_eq!(format_bus_segment_label(&empty), "fleet/announce");

        let empty_strings = make_bus_segment("default", Some(""), Some(""));
        assert_eq!(format_bus_segment_label(&empty_strings), "fleet/announce");

        // Truncation: 100-char body should truncate to 59 + ellipsis.
        let long_body = "x".repeat(100);
        let long = make_bus_segment("default", None, Some(&long_body));
        let label = format_bus_segment_label(&long);
        assert!(label.ends_with('…'));
        assert_eq!(label.chars().count(), 60);
    }

    /// `Message::BusAnnounceSegment` appends to state + sweeps
    /// expired entries.
    #[test]
    fn bus_announce_message_appends_to_state() {
        let mut app = DockApp::default();
        let seg = make_bus_segment("urgent", Some("alert"), Some("disk full"));
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::BusAnnounceSegment(seg),
        );
        assert_eq!(app.bus_announce_segments.len(), 1);
        assert_eq!(app.bus_announce_segments[0].priority, "urgent");
    }

    /// Buffer cap: 12 segments queued → 8 retained, oldest dropped.
    #[test]
    fn bus_announce_buffer_caps_at_8() {
        let mut app = DockApp::default();
        for i in 0..12 {
            let title = format!("msg-{i}");
            let _ = iced_layershell::Application::update(
                &mut app,
                Message::BusAnnounceSegment(make_bus_segment(
                    "default",
                    Some(&title),
                    None,
                )),
            );
        }
        assert!(app.bus_announce_segments.len() <= 8);
        // Latest survives.
        assert_eq!(
            app.bus_announce_segments
                .last()
                .and_then(|s| s.title.as_deref()),
            Some("msg-11")
        );
    }

    /// `parse_announce_file` extracts the StoredMessage envelope
    /// fields directly (priority + title + body live on the outer
    /// object — unlike the urgent-pulse payload which nests a JSON
    /// string in `body`).
    #[test]
    fn parse_announce_file_extracts_envelope() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("01HZAZ123.json");
        let outer = serde_json::json!({
            "ulid": "01HZAZ123",
            "topic": "fleet/announce",
            "priority": "high",
            "title": "Backup started",
            "body": "peer-A → peer-B",
            "ts_unix_ms": 1700000000000_i64,
            "file_path": "fleet/announce/01HZAZ123.json",
        });
        std::fs::write(&path, outer.to_string()).unwrap();
        let seg = crate::workspace::parse_announce_file(&path, "01HZAZ123").unwrap();
        assert_eq!(seg.priority, "high");
        assert_eq!(seg.title.as_deref(), Some("Backup started"));
        assert_eq!(seg.body.as_deref(), Some("peer-A → peer-B"));
        assert_eq!(seg.ulid, "01HZAZ123");
    }

    /// `min` priority messages are silently filtered — design lock
    /// says `min produces no segment (silent)`.
    #[test]
    fn parse_announce_file_filters_min_priority() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("01HZAZ456.json");
        let outer = serde_json::json!({
            "ulid": "01HZAZ456",
            "priority": "min",
            "title": "quiet event",
            "body": null,
        });
        std::fs::write(&path, outer.to_string()).unwrap();
        assert!(crate::workspace::parse_announce_file(&path, "01HZAZ456").is_none());
    }

    /// Missing priority defaults to `default`.
    #[test]
    fn parse_announce_file_defaults_priority() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("01HZAZ789.json");
        let outer = serde_json::json!({
            "ulid": "01HZAZ789",
            "title": "no priority field",
            "body": null,
        });
        std::fs::write(&path, outer.to_string()).unwrap();
        let seg = crate::workspace::parse_announce_file(&path, "01HZAZ789").unwrap();
        assert_eq!(seg.priority, "default");
    }

    // ── BUS-2.2.b Clipboard topic tests ─────────────────────────────────────

    /// Clipboard segments ALWAYS render neutral grey regardless of
    /// priority. Mirrors the design lock: clipboard adds are
    /// visually distinct from notifications.
    #[test]
    fn bus_segment_color_clipboard_always_grey() {
        let grey = Color { r: 0.55, g: 0.55, b: 0.55, a: 1.0 };
        assert_eq!(bus_segment_color(BusSegmentTopic::Clipboard, "urgent"), grey);
        assert_eq!(bus_segment_color(BusSegmentTopic::Clipboard, "high"), grey);
        assert_eq!(bus_segment_color(BusSegmentTopic::Clipboard, "default"), grey);
        assert_eq!(bus_segment_color(BusSegmentTopic::Clipboard, "unknown"), grey);
    }

    /// Announce segments use the priority palette via
    /// `bus_priority_color`. Direct delegation, no override.
    #[test]
    fn bus_segment_color_announce_delegates_to_priority() {
        assert_eq!(
            bus_segment_color(BusSegmentTopic::Announce, "urgent"),
            bus_priority_color("urgent")
        );
        assert_eq!(
            bus_segment_color(BusSegmentTopic::Announce, "high"),
            bus_priority_color("high")
        );
        assert_eq!(
            bus_segment_color(BusSegmentTopic::Announce, "default"),
            bus_priority_color("default")
        );
    }

    /// `parse_breadcrumb_file` with the Clipboard topic tags the
    /// resulting segment correctly.
    #[test]
    fn parse_breadcrumb_file_tags_clipboard_topic() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("01HZACLIP01.json");
        let outer = serde_json::json!({
            "ulid": "01HZACLIP01",
            "priority": "default",
            "title": "copy",
            "body": "hello world",
        });
        std::fs::write(&path, outer.to_string()).unwrap();
        let seg = crate::workspace::parse_breadcrumb_file(
            &path,
            "01HZACLIP01",
            BusSegmentTopic::Clipboard,
        )
        .unwrap();
        assert_eq!(seg.topic, BusSegmentTopic::Clipboard);
        assert_eq!(seg.body.as_deref(), Some("hello world"));
    }

    /// Clipboard segments push through the Message handler the
    /// same way Announce segments do (no special-casing in the
    /// handler — just the tint differs).
    #[test]
    fn clipboard_segment_appends_to_state() {
        let mut app = DockApp::default();
        let seg = make_clipboard_segment(None, Some("hello"));
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::BusAnnounceSegment(seg),
        );
        assert_eq!(app.bus_announce_segments.len(), 1);
        assert_eq!(
            app.bus_announce_segments[0].topic,
            BusSegmentTopic::Clipboard
        );
    }

    // ── BUS-2.2.b.peer tests ────────────────────────────────────────────────

    /// `local_hostname` returns a non-empty string (never an empty
    /// string, even on malformed /proc — fallback to
    /// `"unknown-host"`).
    #[test]
    fn local_hostname_never_empty() {
        let h = crate::workspace::local_hostname();
        assert!(!h.is_empty());
        assert!(!h.contains('\n'), "hostname must be trimmed");
    }

    /// `peer_alerts_topic_dir` composes the expected layout under
    /// XDG_DATA_HOME.
    #[test]
    fn peer_alerts_topic_dir_composes_path() {
        let path = crate::workspace::peer_alerts_topic_dir("fedora").unwrap();
        let s = path.to_string_lossy();
        assert!(s.contains("mde/bus/peer/fedora/alerts"));
    }

    /// `peer_system_topic_dir` composes the expected layout.
    #[test]
    fn peer_system_topic_dir_composes_path() {
        let path = crate::workspace::peer_system_topic_dir("fedora-laptop").unwrap();
        let s = path.to_string_lossy();
        assert!(s.contains("mde/bus/peer/fedora-laptop/system"));
    }

    /// Hostnames with unusual characters (still per RFC 1123) round-
    /// trip through path composition.
    #[test]
    fn peer_topic_dirs_handle_dashes_and_dots() {
        let p1 = crate::workspace::peer_alerts_topic_dir("host.with.dots").unwrap();
        assert!(p1.to_string_lossy().contains("host.with.dots/alerts"));
        let p2 = crate::workspace::peer_system_topic_dir("host-with-dashes").unwrap();
        assert!(p2.to_string_lossy().contains("host-with-dashes/system"));
    }

    /// Malformed JSON returns None — subscription stays alive.
    #[test]
    fn parse_announce_file_returns_none_on_malformed() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("bad.json");
        std::fs::write(&path, "not json").unwrap();
        assert!(crate::workspace::parse_announce_file(&path, "bad").is_none());
    }

    // ── Portal-57.b.click tests ─────────────────────────────────────────────

    /// `active_pulse_con_id_for_workspace` returns the con_id of
    /// the newest active pulse on the workspace, or None when no
    /// pulse maps to it.
    #[test]
    fn active_pulse_con_id_picks_newest_match() {
        let windows = vec![
            make_window(42, "foot", 3, false),
            make_window(99, "firefox", 3, false),
        ];
        let now = chrono::Local::now();
        let older = UrgentPulse {
            ulid: "01HZAZ0001".into(),
            app_id: "foot".into(),
            con_id: 42,
            spawned_at: now - chrono::Duration::milliseconds(300),
        };
        let newer = UrgentPulse {
            ulid: "01HZAZ0002".into(),
            app_id: "firefox".into(),
            con_id: 99,
            spawned_at: now,
        };
        let pulses = vec![older, newer];
        // Newest match wins → con_id 99 (firefox).
        assert_eq!(
            active_pulse_con_id_for_workspace(3, &pulses, &windows, now),
            Some(99)
        );
        // Untargeted workspace → None.
        assert!(
            active_pulse_con_id_for_workspace(7, &pulses, &windows, now).is_none()
        );
    }

    /// Expired pulses are ignored even when they'd otherwise match.
    #[test]
    fn active_pulse_con_id_skips_expired() {
        let windows = vec![make_window(42, "foot", 3, false)];
        let now = chrono::Local::now();
        let stale = UrgentPulse {
            ulid: "01HZAZ0001".into(),
            app_id: "foot".into(),
            con_id: 42,
            spawned_at: now - chrono::Duration::milliseconds(5000),
        };
        let pulses = vec![stale];
        assert!(
            active_pulse_con_id_for_workspace(3, &pulses, &windows, now).is_none()
        );
    }

    /// `Message::UrgentPulse` appends to `recent_pulses` + sweeps
    /// expired entries.
    #[test]
    fn urgent_pulse_message_appends_to_state() {
        let mut app = DockApp::default();
        let pulse_a = make_urgent_pulse(42, "foot");
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::UrgentPulse(pulse_a.clone()),
        );
        assert_eq!(app.recent_pulses.len(), 1);
        assert_eq!(app.recent_pulses[0].con_id, 42);
        // Second pulse appends.
        let pulse_b = make_urgent_pulse(99, "firefox");
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::UrgentPulse(pulse_b),
        );
        assert_eq!(app.recent_pulses.len(), 2);
    }

    /// Buffer cap: more than 32 pulses queued → oldest drop out.
    #[test]
    fn urgent_pulse_buffer_caps_at_32() {
        let mut app = DockApp::default();
        for i in 0..40 {
            let _ = iced_layershell::Application::update(
                &mut app,
                Message::UrgentPulse(make_urgent_pulse(i, "foot")),
            );
        }
        assert!(app.recent_pulses.len() <= 32);
        // Oldest pulses dropped → latest con_id is still 39.
        assert_eq!(app.recent_pulses.last().unwrap().con_id, 39);
    }

    /// `parse_pulse_file` extracts envelope from a BUS-1.4 message
    /// file. Mirrors the Portal-57.a JSON shape:
    /// `{"tier":"crit","source":"<app>","con_id":<n>}` nested
    /// inside `StoredMessage.body`.
    #[test]
    fn parse_pulse_file_extracts_envelope() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("01HZAZ123.json");
        let body_json = r#"{"tier":"crit","source":"foot","con_id":42}"#;
        let outer = serde_json::json!({
            "ulid": "01HZAZ123",
            "topic": "bus/mbadge/pulse",
            "priority": "crit",
            "title": null,
            "body": body_json,
            "ts_unix_ms": 1700000000000_i64,
            "file_path": "bus/mbadge/pulse/01HZAZ123.json",
        });
        std::fs::write(&path, outer.to_string()).unwrap();
        let pulse = crate::workspace::parse_pulse_file(&path, "01HZAZ123").unwrap();
        assert_eq!(pulse.con_id, 42);
        assert_eq!(pulse.app_id, "foot");
        assert_eq!(pulse.ulid, "01HZAZ123");
    }

    /// Malformed pulse files return None — the subscription
    /// continues without panicking on garbage on the topic.
    #[test]
    fn parse_pulse_file_returns_none_on_malformed() {
        let tmp = tempfile::tempdir().unwrap();
        // Not JSON.
        let bad1 = tmp.path().join("01HZAZ001.json");
        std::fs::write(&bad1, "not json").unwrap();
        assert!(crate::workspace::parse_pulse_file(&bad1, "01HZAZ001").is_none());
        // No `body` field.
        let bad2 = tmp.path().join("01HZAZ002.json");
        std::fs::write(&bad2, r#"{"ulid":"01HZAZ002"}"#).unwrap();
        assert!(crate::workspace::parse_pulse_file(&bad2, "01HZAZ002").is_none());
        // `body` is a string but not parseable JSON.
        let bad3 = tmp.path().join("01HZAZ003.json");
        std::fs::write(&bad3, r#"{"body":"not json"}"#).unwrap();
        assert!(crate::workspace::parse_pulse_file(&bad3, "01HZAZ003").is_none());
        // body parses but missing con_id.
        let bad4 = tmp.path().join("01HZAZ004.json");
        std::fs::write(
            &bad4,
            r#"{"body":"{\"tier\":\"crit\",\"source\":\"foot\"}"}"#,
        )
        .unwrap();
        assert!(crate::workspace::parse_pulse_file(&bad4, "01HZAZ004").is_none());
    }

    /// Portal-50.b: `DeclineTagDefaultLayout` clears the banner.
    /// The workspaces.json write may fail in the test env (no XDG
    /// data home configured) — the banner still dismisses.
    #[test]
    fn decline_message_clears_banner() {
        let mut app = DockApp::default();
        app.layout_prompt = Some(make_layout_prompt_state(1, "tabbed"));
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::DeclineTagDefaultLayout {
                workspace_num: 1,
                layout: "tabbed".to_string(),
            },
        );
        assert!(app.layout_prompt.is_none());
    }

    /// `BindingExecuted` with a non-layout command leaves
    /// `layout_prompt` alone.
    #[test]
    fn binding_executed_non_layout_command_is_no_op() {
        let mut app = DockApp::default();
        assert!(app.layout_prompt.is_none());
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::BindingExecuted("focus left".to_string()),
        );
        assert!(app.layout_prompt.is_none());
    }

    /// `compute_layout_prompt` returns None when:
    ///   - parse fails (non-layout command).
    ///   - no focused workspace.
    ///   - focused workspace is the parked slot.
    /// (Tag-store interaction is tested live via the bench
    /// acceptance — verifying it here would mock TagStore which
    /// isn't worth the complexity for an integration test.)
    #[test]
    fn compute_layout_prompt_skips_non_layout_commands() {
        let app = DockApp::default();
        assert!(compute_layout_prompt(&app, "focus left").is_none());
        assert!(compute_layout_prompt(&app, "workspace number 2").is_none());
        assert!(compute_layout_prompt(&app, "layout toggle split").is_none());
    }

    #[test]
    fn compute_layout_prompt_skips_with_no_focused_workspace() {
        let app = DockApp::default();
        // Empty workspaces list → no focused → no prompt.
        assert!(compute_layout_prompt(&app, "layout tabbed").is_none());
    }

    #[test]
    fn compute_layout_prompt_skips_parked_workspace() {
        let mut app = DockApp::default();
        app.workspaces = vec![make_ws(PARKED_WORKSPACE_NUM, "99", true, true, "eDP-1")];
        assert!(compute_layout_prompt(&app, "layout tabbed").is_none());
    }

    /// `parse_layout_command` recognises the four locked layout
    /// names + strips leading `[con_id=N]` criterion blocks.
    #[test]
    fn parse_layout_command_recognises_locked_names() {
        use crate::workspace::parse_layout_command;
        assert_eq!(parse_layout_command("layout splith"), Some("splith"));
        assert_eq!(parse_layout_command("layout splitv"), Some("splitv"));
        assert_eq!(parse_layout_command("layout tabbed"), Some("tabbed"));
        assert_eq!(parse_layout_command("layout stacked"), Some("stacked"));
        // `stacking` is the legacy form sway accepts; map to `stacked`.
        assert_eq!(parse_layout_command("layout stacking"), Some("stacked"));
        // Leading whitespace + criterion blocks stripped.
        assert_eq!(parse_layout_command("  layout splith"), Some("splith"));
        assert_eq!(
            parse_layout_command("[con_id=42] layout tabbed"),
            Some("tabbed")
        );
    }

    #[test]
    fn parse_layout_command_rejects_non_pin_forms() {
        use crate::workspace::parse_layout_command;
        // Toggle forms don't pin a specific layout.
        assert!(parse_layout_command("layout toggle split").is_none());
        // Non-layout commands.
        assert!(parse_layout_command("focus left").is_none());
        assert!(parse_layout_command("workspace number 2").is_none());
        // Unknown layout name.
        assert!(parse_layout_command("layout invalid").is_none());
        // Empty input.
        assert!(parse_layout_command("").is_none());
        assert!(parse_layout_command("   ").is_none());
    }

    /// Portal-43 + Portal-59 interaction: focusing the parked workspace
    /// (briefly during `wm_minimize`) must NOT push it onto the LRU —
    /// otherwise the breadcrumb would offer to jump back to the
    /// hidden park slot.
    #[test]
    fn parked_workspace_excluded_from_lru() {
        let mut app = DockApp::default();
        // Seed: ws1 focused.
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::WorkspaceList(vec![make_ws(1, "1", true, true, "eDP-1")]),
        );
        // Park transition: ws1 unfocused, ws 99 focused (the wm_minimize
        // helper switches there for a single tick before back_and_forth).
        let _ = iced_layershell::Application::update(
            &mut app,
            Message::WorkspaceList(vec![
                make_ws(1, "1", false, false, "eDP-1"),
                make_ws(PARKED_WORKSPACE_NUM, "99", true, true, "eDP-1"),
            ]),
        );
        // focused_workspace skips ws 99 → no LRU push, last_workspace_change
        // stays at its seed value.
        assert!(
            app.visited_workspaces_lru.is_empty(),
            "park transition must not pollute the previous-workspace LRU"
        );
    }

    #[test]
    fn wm_layout_cycle_fires_task_without_panic() {
        let mut app = DockApp::default();
        let _task =
            iced_layershell::Application::update(&mut app, Message::WmLayoutCycle(42));
    }

    #[test]
    fn wm_messages_do_not_alter_hover_state() {
        let mut app = DockApp::default();
        app.hovered_running_group = Some("foot".to_string());
        let _ = iced_layershell::Application::update(&mut app, Message::WmClose(1));
        assert_eq!(
            app.hovered_running_group.as_deref(),
            Some("foot"),
            "WM actions should not clear hover state"
        );
    }
}
