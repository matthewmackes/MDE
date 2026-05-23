//! mde-panel — Iced + libcosmic top bar + bottom dock for the
//! Mackes Desktop Environment.
//!
//! Phase E.1 lock (revised 2026-05-21):
//! - Ships **side-by-side** with the legacy GTK3 `mackes-panel`
//!   crate. Both binaries co-exist during the Phase E port; the
//!   spec eventually flips `/usr/bin/mackes-panel` to the
//!   `mde-panel` binary once parity is reached. This avoids
//!   regressing installed v2.0.x boxes mid-port.
//! - Builds on **raw Iced 0.13** with the same feature set as
//!   `mde-workbench` and `mde-files`, so the workspace dep tree
//!   resolves to a single Iced version. `libcosmic` integration
//!   stays optional — it lands at Phase E.1.3 if the
//!   cosmic-theme adapter justifies it; today the `mackes-theme`
//!   crate (E3.1) handles token parsing without cosmic-theme.
//! - **Wayland-first.** Phase E.2 wires `iced_layershell`'s
//!   wlr-layer-shell-v1 anchor (bottom edge, 40 px exclusive zone,
//!   Left|Right stretch so the compositor fills the full output width).
//!
//! Source-file modules (`pub mod`) are added per-port in
//! Phase E.4 → E.29. The skeleton itself ships only the app
//! shell + the cross-cutting `Message`/`Pane` types.

#![forbid(unsafe_code)]

use iced::{Element, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

/// xdg-shell `app_id` advertised to the Wayland compositor. Sway's
/// `for_window [app_id="shell.mackes.Panel"]` rule in
/// `data/sway/config` matches against this string. The reverse-DNS
/// form (vs. the bare `mde-panel`) follows the freedesktop
/// recommendation that the `app_id` match the basename of the
/// `.desktop` file — `shell.mackes.Panel.desktop` ships at
/// `/usr/share/applications/`.
pub const APP_ID: &str = "shell.mackes.Panel";

// v3.0.3 — admin_menu moved to crates/mde-popover/src/admin_menu.rs
// (it's a popover, not panel chrome). The mde-panel never invoked the
// helper at runtime; moving it puts the code in the crate that
// actually mounts the UI. See git history for the original location.
pub mod applet_host;
// v3.0.3 — clipboard.rs moved to crates/mde-popover/src/clipboard.rs
// (the helpers are popover-side; the clip-history popover lives
// there too).
pub mod dock_dnd;
// v3.0.3 — expose.rs moved to crates/mde-popover/src/expose.rs
// (the F3 exposé grid is a popover surface, not panel chrome).
pub mod hero;
pub mod host;
pub mod icon_mapper;
pub mod panel_icons;
pub mod workspaces;
// v3.0.3 — layer_shell.rs retired. The module was a Phase E.2 set
// of config helpers (anchor/exclusive-zone constants) for an
// integration that ended up shipping via `iced_layershell 0.13.7`
// at v3.0.2 instead. The helpers became unreachable at the moment
// they would have been used; deleted 2026-05-22 per §0.12.
pub mod recover;
// v3.0.3 — root_menu.rs retired. The original design was a
// right-click-on-wallpaper menu (Change wallpaper / Open mesh
// share / Send file to peer / Display settings). In MDE the
// wallpaper is owned by swaybg, which has no event hook for
// right-click; sway's `bindsym button3` is global (fires on
// every right-click anywhere including over apps) and not
// suitable. Each of the 4 actions is already exposed elsewhere:
// Change wallpaper → Workbench > Look & Feel; Open mesh share →
// xdg-open ~/QNM-Shared via shell; Send file to peer → mde-files
// per-peer view; Display settings → Workbench > Devices.
// Retired 2026-05-22 per §0.12.
pub mod sliders;
pub mod theme;
// v3.0.3 — toasts.rs moved to crates/mde-popover/src/toasts.rs
// (the render surface + every emit site lives in the popover
// crate now).
pub mod top_bar;
pub mod toplevels;
pub mod toplevels_sub;
// v3.0.3 — watermark.rs and weather.rs both moved to
// crates/mde-popover/src/ (they're popover surfaces, not panel
// chrome). The mde-panel never invoked the helpers at runtime;
// moving puts the data layer in the crate that actually mounts
// the UI. See git history for the original locations.

// ──────────────────────────────────────────────────────────────
// Public layout zones (Phase E lock)
// ──────────────────────────────────────────────────────────────

/// The six named layout zones of the MDE top-bar (1.1.0 Win10 lock).
///
/// Each port (E.4 - E.29) writes its widget into one of these zones;
/// the panel orchestrator owns the spatial composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pane {
    /// Left edge — Start button + admin-menu trigger (right-click).
    Start,
    /// Pinned-app strip immediately right of Start.
    Pinned,
    /// Running-window strip (tasklist hero).
    Tasklist,
    /// SPLIT / LAYOUT / WINDOW sway-IPC chips (E.4.1).
    Cluster,
    /// System tray row (bell + NM + mesh + audio + status).
    Tray,
    /// Date / time pill at far right.
    Clock,
}

impl Pane {
    /// Stable ordering of zones, left → right.
    #[must_use]
    pub const fn ordered() -> [Pane; 6] {
        [
            Pane::Start,
            Pane::Pinned,
            Pane::Tasklist,
            Pane::Cluster,
            Pane::Tray,
            Pane::Clock,
        ]
    }

    /// Display label used in test fixtures + accessibility metadata.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Pane::Start => "Start",
            Pane::Pinned => "Pinned apps",
            Pane::Tasklist => "Running windows",
            Pane::Cluster => "Layout cluster",
            Pane::Tray => "System tray",
            Pane::Clock => "Clock",
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Top-level reducer messages
// ──────────────────────────────────────────────────────────────

/// Reducer messages for the panel application.
///
/// Phase E.1.2 ships the no-op variant set; per-port submessages
/// are added as their tasks land.
///
/// `#[to_layer_message]` (Phase E.2) generates the `TryInto<LayershellCustomActions>`
/// impl required by `iced_layershell::Application::run`. Layer-shell action variants
/// (e.g. `AnchorSizeChange`, `SetSizeChange`) are appended by the macro; the
/// user-level `update` function ignores them with `_ => {}`.
#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    /// No-op placeholder — keeps the variant set non-empty so Iced's
    /// pattern matching stays exhaustive.
    Noop,
    /// 1-second tick used by clock + battery + watermark refresh.
    /// Subscription wiring lands at E.17.
    Tick,
    /// Phase E.4-E.29 host wiring — a stdout line from one of the
    /// `mde-applet-*` subprocesses driven by [`applet_host`].
    AppletText(applet_host::AppletKind, String),
    /// Start-button left-click — launches the start-menu popover.
    StartClicked,
    /// v3.0.3 Tier 1D — Start-button right-click. Launches the
    /// admin-menu popover (9 actions, 5 sections, foot --hold).
    /// Iced's built-in `button` is left-click only, so the Start
    /// zone is wrapped in `mouse_area().on_right_press(...)` to
    /// emit this variant.
    StartRightClicked,
    /// Tray-applet click — launches the popover/quick-action bound to
    /// the given applet kind.
    TrayClicked(applet_host::AppletKind),
    /// v4.0.1 BUG-7 — static clipboard-icon click. Toggles the
    /// clipboard-history popover (same surface as Super+V). Lives
    /// outside the AppletKind enum because there's no underlying
    /// `mde-applet-clipboard` subprocess — the panel just spawns
    /// `mde-popover clipboard` on press.
    ClipboardClicked,
    /// v3.0.3 Phase E.3 wiring — one event from the sway-IPC
    /// toplevels subscription. Drives the panel's hero widget,
    /// window-management buttons, and any future tasklist render.
    ToplevelEvent(toplevels::ToplevelEvent),
    /// v3.0.3 Tier 1E (v8.7 window-buttons lock) — minimize the
    /// currently-focused window. Routed via `swaymsg [con_id=N]
    /// move scratchpad`. Best-choice mapping: sway has no
    /// "minimize" concept, so we use the scratchpad hide which
    /// matches the user-visible behavior (window disappears,
    /// reappears via the dock or scratchpad cycle).
    WindowMinimize,
    /// v3.0.3 Tier 1E — toggle floating-fill on the focused window
    /// per the v8.7 lock ("maximize = floating-fill, not
    /// fullscreen"). Routed via `swaymsg [con_id=N] floating
    /// enable, resize set 100ppt 100ppt` (or `floating toggle`
    /// when already floating).
    WindowMaximize,
    /// v3.0.3 Tier 1E — close the focused window. Routed via
    /// `swaymsg [con_id=N] kill`.
    WindowClose,
    /// v4.0.1 BUG-16 — operator clicked one of the Desktop Layout
    /// buttons in the panel's center cluster. Each variant applies
    /// a sway-IPC layout command to the focused workspace per the
    /// Win11 Snap Layouts vocabulary.
    DesktopLayoutSelected(DesktopLayout),
    /// v4.0.1 WM-1 — operator clicked a workspace chip on the
    /// panel. Routes to `swaymsg workspace number <N>`. The number
    /// is the 1-indexed workspace.
    WorkspaceSelected(i32),
}

/// v4.0.1 BUG-16 — Snap-Layouts-style tile templates. Each maps to
/// a sway-IPC sequence the panel runs when its button is clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopLayout {
    /// Single fullscreen window — `swaymsg fullscreen toggle on`
    /// followed by `swaymsg layout default` so the workspace
    /// returns to a tiled stack when the user picks another
    /// layout afterwards.
    Single,
    /// Two windows side-by-side. `swaymsg splith`. The user
    /// arranges the second window in via standard sway focus
    /// moves; the button just ensures the workspace's container
    /// is horizontal-split.
    Vsplit,
    /// 2×2 grid. Walks the workspace's containers and applies
    /// `splith` at the root + `splitv` on each top-level child.
    /// Best-effort — sway tiling is a tree, so the resulting
    /// shape depends on how many windows are present (4+ →
    /// nominal 2×2; 3 → 2/1; 2 → mirrors Vsplit).
    Grid,
    /// Main pane + sidebar (60/40 horizontal split). `swaymsg
    /// splith` + resize the focused window to 60ppt.
    MainSidebar,
    /// Tabbed layout — `swaymsg layout tabbed` so windows stack
    /// behind sway's native title-row, only the focused one
    /// visible.
    Tabbed,
}

// ──────────────────────────────────────────────────────────────
// Application state
// ──────────────────────────────────────────────────────────────

/// Panel application state.
///
/// Phase E.1.2 skeleton: top-bar state container. Per-port state
/// writers (E.4.1 cluster, E.10 dock, E.11 start menu, etc.)
/// mutate `top_bar` fields as their wiring lands.
#[derive(Debug, Default)]
pub struct App {
    /// Counts how many `Tick` messages have been received — used to
    /// confirm the subscription is wired in tests.
    ticks: u64,
    /// Top-bar zone state. Defaults to demo content; real per-port
    /// state writers replace individual fields.
    top_bar: top_bar::TopBarState,
    /// Running popover children indexed by kind. v3.0.3 fix for
    /// the dedup + zombie defects: the panel keeps the `Child`
    /// handle so a second click on the same tray button kills the
    /// existing popover (toggle behavior) and exited popovers get
    /// reaped via `try_wait` on every spawn — no SIGCHLD ignore
    /// and no fire-and-forget zombie pile-up.
    popovers: std::collections::HashMap<&'static str, std::process::Child>,
    /// Live model of every sway top-level window. v3.0.3 Phase E.3
    /// wiring — fed by the `toplevels_sub` subscription. The hero
    /// widget reads `focused()` for its label; the window-management
    /// buttons read `focused()` for their target id.
    toplevels: toplevels::ToplevelModel,
    /// v3.0.3 Phase E.4.2 wiring — focused-window hero with the
    /// 280ms slide animation. `set_focused()` is called from
    /// `update` when a `ToplevelEvent` lands a new focused window;
    /// `tick()` advances the slide on every `Message::Tick`.
    hero: hero::Hero,
    /// v4.0.1 WM-1 — workspace switcher state. Refreshed on a 2 s
    /// cadence by polling `swaymsg -t get_workspaces` inside the
    /// existing Tick subscription (every 60th tick at ~33 ms).
    workspaces: [Option<workspaces::WorkspaceState>; 4],
}

impl App {
    /// Construct with the demo top-bar state so early Iced launches
    /// render something. Per-port wiring replaces this.
    #[must_use]
    pub fn with_demo_state() -> Self {
        Self {
            ticks: 0,
            top_bar: top_bar::TopBarState::demo(),
            popovers: std::collections::HashMap::new(),
            toplevels: toplevels::ToplevelModel::new(),
            hero: hero::Hero::new(),
            workspaces: [None, None, None, None],
        }
    }

    /// Spawn a `mde-popover <kind>` child with dedup + reap. v3.0.3
    /// fix for three concurrent defects in the previous fire-and-
    /// forget spawn:
    ///
    /// 1. **Toggle dedup:** clicking a tray icon while its popover
    ///    is already open closes the popover (rather than stacking
    ///    a second instance on top of the first).
    /// 2. **Zombie reap:** every spawn first reaps any previously-
    ///    spawned popover children that have exited (the user
    ///    pressed Esc, picked an app, etc.). No SIGCHLD=SIG_IGN
    ///    needed; the held `Child` handle is the reap path.
    /// 3. **Process count cap:** the HashMap is keyed by kind, so
    ///    at most one popover per kind can exist at a time.
    fn toggle_or_spawn_popover(&mut self, kind: &'static str) {
        // First, reap any popovers that have already exited so the
        // HashMap reflects current reality (user Esc'd, etc.). We
        // mutate in two passes because borrow-checker.
        let dead_kinds: Vec<&'static str> = self
            .popovers
            .iter_mut()
            .filter_map(|(k, child)| match child.try_wait() {
                Ok(Some(_status)) => Some(*k),
                Ok(None) | Err(_) => None,
            })
            .collect();
        for k in dead_kinds {
            self.popovers.remove(k);
        }

        // Toggle: if this kind is already open, close it.
        if let Some(mut child) = self.popovers.remove(kind) {
            let _ = child.kill();
            let _ = child.wait();
            tracing::debug!(kind, "popover toggle: closed existing");
            return;
        }

        // Open: spawn fresh, hold handle for future reap/toggle.
        match std::process::Command::new("mde-popover")
            .arg(kind)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(child) => {
                tracing::debug!(kind, pid = child.id(), "popover spawned");
                self.popovers.insert(kind, child);
            }
            Err(e) => {
                tracing::warn!(kind, error = %e, "popover spawn failed");
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Phase E.2 — wlr-layer-shell-v1 anchor via iced_layershell
// ──────────────────────────────────────────────────────────────

impl iced_layershell::Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Task<Message>) {
        // Start with the loading placeholder strings — the first
        // applet emit happens within ~1s of spawn, so the user only
        // sees these for a beat.
        (
            Self {
                ticks: 0,
                top_bar: top_bar::TopBarState::loading(),
                popovers: std::collections::HashMap::new(),
                toplevels: toplevels::ToplevelModel::new(),
                hero: hero::Hero::new(),
                workspaces: [None, None, None, None],
            },
            Task::none(),
        )
    }

    /// Layer-shell namespace — sway surfaces this as the surface role name.
    fn namespace(&self) -> String {
        APP_ID.to_string()
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Noop => {}
            Message::Tick => {
                self.ticks = self.ticks.saturating_add(1);
                // v3.0.3 — advance the hero slide animation. Tick is
                // wired to a 33ms (~30Hz) subscription which keeps
                // the 280ms slide smooth without burning CPU when
                // no animation is in progress (hero::tick is a
                // no-op when there's no incoming entry).
                self.hero.tick(std::time::Instant::now());
                // v4.0.1 WM-1 — workspace switcher poll. Refresh
                // every 60th tick (= ~2s at 33ms/tick) so the chip
                // row picks up sway focus/window changes without
                // burning CPU every frame.
                if self.ticks.is_multiple_of(60) {
                    let raw = std::process::Command::new("swaymsg")
                        .args(["-t", "get_workspaces"])
                        .output()
                        .ok()
                        .and_then(|o| {
                            if o.status.success() {
                                String::from_utf8(o.stdout).ok()
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default();
                    let parsed = workspaces::parse_workspaces(&raw);
                    self.workspaces = workspaces::fixed_four_slots(&parsed);
                }
            }
            Message::AppletText(kind, text) => {
                tracing::debug!(
                    applet = ?kind,
                    text = %text,
                    "applet text received"
                );
                self.top_bar.set_applet_text(kind, text);
            }
            Message::ClipboardClicked => {
                // v4.0.1 BUG-7 — operator reported no clipboard
                // surface on the panel. Super+V already wires
                // mde-popover clipboard via sway config; this
                // duplicates the path via a visible tray icon.
                self.toggle_or_spawn_popover("clipboard");
            }
            Message::StartClicked => {
                // v3.0.3 — toggle the start-menu popover. Second
                // click on M closes the existing instance instead
                // of stacking a second one on top.
                self.toggle_or_spawn_popover("start-menu");
            }
            Message::StartRightClicked => {
                // v3.0.3 Tier 1D — toggle the admin-menu popover.
                // Bug fix for "right-click on the start menu does
                // not work": this variant was never emitted because
                // the Start button was a plain Iced `button` (left-
                // click only). v3.0.3 wraps it in `mouse_area` to
                // get the right-press event.
                self.toggle_or_spawn_popover("admin-menu");
            }
            Message::ToplevelEvent(ev) => {
                // v3.0.3 Phase E.3 wiring — apply the event to the
                // in-memory model. Hero + window-management buttons
                // read from `self.toplevels` on their next view().
                let changed = self.toplevels.apply(ev);
                if changed {
                    tracing::debug!(
                        live_count = self.toplevels.len(),
                        "toplevels model updated"
                    );
                    // v3.0.3 Phase E.4.2 wiring — push the new
                    // focused window's title + app_id into the
                    // hero. `set_focused` is idempotent on the same
                    // entry, so this is cheap to call on every
                    // change.
                    if let Some(focused) = self.toplevels.focused() {
                        self.hero
                            .set_focused(focused.title.clone(), focused.app_id.clone());
                    }
                }
            }
            Message::WindowMinimize => {
                // v3.0.3 Tier 1E — route through the focused
                // toplevel's id. Sway has no native "minimize"; the
                // scratchpad-hide flow is the closest semantic
                // (window disappears, reappears via dock or
                // scratchpad cycle).
                if let Some(focused) = self.toplevels.focused() {
                    swaymsg_window_command(focused.id, "move scratchpad");
                }
            }
            Message::WindowMaximize => {
                // v3.0.3 Tier 1E + v8.7 lock — "maximize = floating-
                // fill, not fullscreen". Toggle floating, then resize
                // to 100ppt × 100ppt so the float covers the output.
                if let Some(focused) = self.toplevels.focused() {
                    if focused.state.fullscreen {
                        // Already in floating-fill (or fullscreen);
                        // toggle off to restore tiled layout.
                        swaymsg_window_command(focused.id, "floating disable");
                    } else {
                        swaymsg_window_command(
                            focused.id,
                            "floating enable, resize set 100ppt 100ppt",
                        );
                    }
                }
            }
            Message::WindowClose => {
                // v3.0.3 Tier 1E — sway's `kill` sends SIGTERM to
                // the window's process. No confirmation dialog at
                // the panel layer; apps that want one (e.g. an
                // editor with unsaved changes) handle it themselves.
                if let Some(focused) = self.toplevels.focused() {
                    swaymsg_window_command(focused.id, "kill");
                }
            }
            Message::DesktopLayoutSelected(kind) => {
                // v4.0.1 BUG-16 — operator clicked a Desktop Layout
                // button. Apply the sway-IPC layout sequence to the
                // current workspace. Each variant translates to a
                // small handful of `swaymsg` commands; we shell out
                // through the standard `Command` helper because the
                // sequence is short + non-blocking.
                let cmd = match kind {
                    DesktopLayout::Single => {
                        // Toggle fullscreen on the focused window.
                        // Not a tile arrangement strictly, but the
                        // "single visible window" is what the Win11
                        // single-template means visually.
                        "fullscreen toggle"
                    }
                    DesktopLayout::Vsplit => "splith",
                    DesktopLayout::Grid => "splith; focus next; splitv",
                    DesktopLayout::MainSidebar => {
                        "splith; resize set width 60ppt"
                    }
                    DesktopLayout::Tabbed => "layout tabbed",
                };
                tracing::info!(layout = ?kind, cmd, "desktop layout selected");
                let _ = std::process::Command::new("swaymsg")
                    .arg(cmd)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
            }
            Message::WorkspaceSelected(num) => {
                tracing::info!(workspace = num, "workspace chip clicked");
                let arg = format!("workspace number {num}");
                let _ = std::process::Command::new("swaymsg")
                    .arg(&arg)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
            }
            Message::TrayClicked(kind) => {
                // v3.0.3 — toggle the popover for the clicked
                // tray applet. Each kind has its own slot so an
                // audio popover and a clock popover can both be
                // open at once; clicking the same icon twice
                // closes that kind's popover.
                match kind {
                    applet_host::AppletKind::Audio => {
                        self.toggle_or_spawn_popover("audio");
                    }
                    applet_host::AppletKind::Network => {
                        self.toggle_or_spawn_popover("network");
                    }
                    applet_host::AppletKind::NotificationBell => {
                        self.toggle_or_spawn_popover("notifications");
                    }
                    applet_host::AppletKind::Clock => {
                        self.toggle_or_spawn_popover("clock");
                    }
                    _ => {
                        // Sway-cluster / mesh-status / status-cluster /
                        // dock don't have popovers yet — clicking
                        // them is a no-op until v3.1.
                    }
                }
            }
            // Layer-shell action variants injected by #[to_layer_message] —
            // the runtime intercepts them before they reach user code but
            // the exhaustiveness check requires this arm.
            _ => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        // Phase E.17 + Phase E.4-E.29 host wiring — render the live
        // applet text per zone. v3.0.3 Phase E.4.2 wiring — hero
        // shows the focused-window title; v3.0.3 Tier 1E — window-
        // management buttons read the same focused state for their
        // greyed-out check.
        top_bar::view(
            &self.top_bar,
            &self.hero,
            self.toplevels.focused(),
            &self.workspaces,
        )
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        // v3.0.3 — batch the applet-host stream + the toplevels
        // subscription + the animation tick. Adding more
        // subscriptions later (clipboard, foreign-toplevel via
        // wlr protocol, etc.) extends the batch.
        iced::Subscription::batch([
            applet_host::subscription(|t| Message::AppletText(t.kind, t.text)),
            toplevels_sub::subscription(Message::ToplevelEvent),
            // ~30Hz animation tick — drives `hero::tick` so the
            // 280ms slide stays smooth. 33ms is a balance between
            // visual smoothness (>=10 frames per slide) and CPU
            // wake-ups when the panel is idle.
            iced::time::every(std::time::Duration::from_millis(33)).map(|_| Message::Tick),
        ])
    }

    fn theme(&self) -> Theme {
        // Phase E.1.3 — load tokens.css if available, fall back to
        // Theme::Dark for dev builds.
        theme::load_theme()
    }
}

// v3.0.3 — the previous free-functions `spawn_popover` and
// `spawn_detached` were both fire-and-forget that dropped the
// `Child` handle (leaking zombies on every popover exit) and did
// zero deduplication (every click stacked a new instance). They're
// replaced by `App::toggle_or_spawn_popover` above, which keeps
// the handle for reap + implements toggle dedup. The free
// functions are removed entirely per §0.12 (no dead code); if
// non-App callers need to spawn detached children in the future,
// they should hold their own `Child` handle for reap.

/// v3.0.3 Tier 1E helper — issue a sway-IPC command targeting one
/// container by id. Used by the window-management buttons
/// (minimize/maximize/close) in the panel's right cluster.
///
/// Fire-and-forget is acceptable here: `swaymsg` is a short-lived
/// child that exits in milliseconds and produces no useful stdout/
/// stderr for the panel to consume. We `wait()` on the handle so
/// no zombie accumulates (matches the popover reap pattern; cost
/// is one short blocking call in the GUI thread, which is fine
/// because swaymsg returns ~immediately).
fn swaymsg_window_command(con_id: toplevels::ToplevelId, command: &str) {
    let selector = format!("[con_id={con_id}] {command}");
    tracing::debug!(con_id, command, "swaymsg window command");
    match std::process::Command::new("swaymsg")
        .arg(&selector)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            let _ = child.wait();
        }
        Err(e) => {
            tracing::warn!(con_id, command, error = %e, "swaymsg spawn failed");
        }
    }
}

/// Load system fallback fonts so the audio / status / mesh glyphs
/// render instead of tofu boxes. Iced 0.13 + cosmic-text uses these
/// for glyph-fallback when the default font lacks a code point.
///
/// Order matters: the first font that contains a glyph wins. We try
/// Noto Emoji (monochrome, ~880 KB, matches the panel's dark
/// aesthetic) first, then Symbola (~2.4 MB, broader Unicode
/// coverage) as a last resort. Missing fonts are silently skipped —
/// the panel still renders, just with question-mark boxes for
/// uncovered code points.
fn load_fallback_fonts() -> Vec<std::borrow::Cow<'static, [u8]>> {
    const CANDIDATES: &[&str] = &[
        "/usr/share/fonts/google-noto-emoji-fonts/NotoEmoji-Regular.ttf",
        "/usr/share/fonts/gdouros-symbola/Symbola.ttf",
        "/usr/share/fonts/google-noto/NotoSansSymbols2-Regular.ttf",
    ];
    let mut out = Vec::new();
    for path in CANDIDATES {
        if let Ok(bytes) = std::fs::read(path) {
            tracing::info!(font = path, bytes = bytes.len(), "loaded fallback font");
            out.push(std::borrow::Cow::Owned(bytes));
        }
    }
    if out.is_empty() {
        tracing::warn!(
            "no fallback fonts found — emoji / symbol glyphs will render as tofu boxes"
        );
    }
    out
}

impl App {
    /// Launch the panel anchored to the bottom edge via wlr-layer-shell-v1.
    ///
    /// Phase E.2: `iced_layershell` replaces the plain `iced::application`
    /// functional builder. The compositor (sway) receives:
    ///   - `anchor = Bottom | Left | Right` → stretches full output width
    ///   - `exclusive_zone = 40` → reserves 40 px; tiled windows won't overlap
    ///   - `layer = Top` → above normal windows, below overlays
    ///   - `keyboard_interactivity = OnDemand` → popovers can grab keys
    ///
    /// Also registers a glyph-coverage fallback font (Noto Emoji or
    /// Symbola — whichever the system has) so the audio-applet
    /// speaker glyphs (🔇/🔈/🔉/🔊), the status-cluster lightning bolt
    /// (⚡), and the mesh-status chevrons render instead of tofu boxes.
    pub fn run() -> iced_layershell::Result {
        <App as iced_layershell::Application>::run(Settings {
            id: Some(APP_ID.to_string()),
            fonts: load_fallback_fonts(),
            layer_settings: LayerShellSettings {
                size: Some((0, u32::from(top_bar::TOP_BAR_HEIGHT_PX))),
                exclusive_zone: i32::from(top_bar::TOP_BAR_HEIGHT_PX),
                anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
                layer: Layer::Top,
                keyboard_interactivity: KeyboardInteractivity::OnDemand,
                ..Default::default()
            },
            ..Default::default()
        })
    }
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    // Phase E.2 — `update` / `view` / `theme` are now trait methods on
    // `iced_layershell::Application`. Bring the trait into scope so the
    // existing tests can keep calling them as inherent-style methods.
    use iced_layershell::Application as _;

    #[test]
    fn pane_ordering_has_six_distinct_zones() {
        let panes = Pane::ordered();
        assert_eq!(panes.len(), 6);
        for (i, a) in panes.iter().enumerate() {
            for (j, b) in panes.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn app_id_matches_sway_config_lock() {
        // sway's `for_window [app_id="shell.mackes.Panel"]` rule in
        // data/sway/config matches against APP_ID. If this string ever
        // changes, the sway config rule (and the .desktop file basename)
        // must be updated in lockstep — this test catches the drift.
        assert_eq!(APP_ID, "shell.mackes.Panel");
    }

    #[test]
    fn pane_labels_match_lock() {
        assert_eq!(Pane::Start.label(), "Start");
        assert_eq!(Pane::Pinned.label(), "Pinned apps");
        assert_eq!(Pane::Tasklist.label(), "Running windows");
        assert_eq!(Pane::Cluster.label(), "Layout cluster");
        assert_eq!(Pane::Tray.label(), "System tray");
        assert_eq!(Pane::Clock.label(), "Clock");
    }

    #[test]
    fn pane_is_copy_and_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for p in Pane::ordered() {
            set.insert(p);
        }
        assert_eq!(set.len(), 6);
    }

    #[test]
    fn app_default_is_initial_state() {
        let app = App::default();
        assert_eq!(app.ticks, 0);
    }

    #[test]
    fn tick_increments_counter() {
        let mut app = App::default();
        let _ = app.update(Message::Tick);
        assert_eq!(app.ticks, 1);
        let _ = app.update(Message::Tick);
        assert_eq!(app.ticks, 2);
    }

    #[test]
    fn noop_is_idempotent() {
        let mut app = App::default();
        app.ticks = 7;
        let _ = app.update(Message::Noop);
        assert_eq!(app.ticks, 7);
    }

    #[test]
    fn tick_counter_saturates_at_u64_max() {
        let mut app = App::default();
        app.ticks = u64::MAX;
        let _ = app.update(Message::Tick);
        assert_eq!(app.ticks, u64::MAX);
    }
}
