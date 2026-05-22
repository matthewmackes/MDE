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

pub mod admin_menu;
pub mod applet_host;
pub mod clipboard;
pub mod dock_dnd;
pub mod expose;
pub mod hero;
pub mod host;
pub mod icon_mapper;
pub mod layer_shell;
pub mod recover;
pub mod root_menu;
pub mod sliders;
pub mod theme;
pub mod toasts;
pub mod top_bar;
pub mod toplevels;
pub mod watermark;
pub mod weather;

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
    /// Tray-applet click — launches the popover/quick-action bound to
    /// the given applet kind.
    TrayClicked(applet_host::AppletKind),
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
}

impl App {
    /// Construct with the demo top-bar state so early Iced launches
    /// render something. Per-port wiring replaces this.
    #[must_use]
    pub fn with_demo_state() -> Self {
        Self {
            ticks: 0,
            top_bar: top_bar::TopBarState::demo(),
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
            }
            Message::AppletText(kind, text) => {
                tracing::debug!(
                    applet = ?kind,
                    text = %text,
                    "applet text received"
                );
                self.top_bar.set_applet_text(kind, text);
            }
            Message::StartClicked => {
                // v3.0.2 — spawn the unified `mde-popover` host with
                // the start-menu kind. Layer-shell overlay opens
                // above the M button.
                spawn_popover("start-menu");
            }
            Message::TrayClicked(kind) => {
                // v3.0.2 — route per-applet clicks through the
                // unified `mde-popover` host. Only `start-menu` has a
                // real popover today; the rest dispatch to stub
                // branches that exit 0 (v3.1 follow-up scope).
                match kind {
                    applet_host::AppletKind::Audio => spawn_popover("audio"),
                    applet_host::AppletKind::Network => spawn_popover("network"),
                    applet_host::AppletKind::NotificationBell => {
                        spawn_popover("notifications");
                    }
                    applet_host::AppletKind::Clock => spawn_popover("clock"),
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
        // applet text per zone. The state is mutated by `update`
        // through `AppletText` messages emitted by `applet_host`.
        top_bar::view(&self.top_bar)
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        applet_host::subscription(|t| Message::AppletText(t.kind, t.text))
    }

    fn theme(&self) -> Theme {
        // Phase E.1.3 — load tokens.css if available, fall back to
        // Theme::Dark for dev builds.
        theme::load_theme()
    }
}

/// Spawn a binary detached (stdio devnull, fully owned by the OS
/// once it exits). Used by `update` to launch popover/overlay
/// applets in response to clicks; the panel doesn't supervise the
/// child — it's expected to manage its own lifetime.
fn spawn_detached(binary: &str) {
    use std::process::{Command, Stdio};
    let _ = Command::new(binary)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

/// Spawn `mde-popover <kind>` detached — the unified popover host
/// binary handles all overlay surfaces (start-menu today, audio /
/// notifications / clock / network stubs ready for v3.1 ports).
/// Per-popover dedup is left to the popover process itself (each
/// uses its own `app_id`); a second click while the popover is open
/// just spawns a redundant process that the compositor coalesces.
fn spawn_popover(kind: &str) {
    use std::process::{Command, Stdio};
    let _ = Command::new("mde-popover")
        .arg(kind)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
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
