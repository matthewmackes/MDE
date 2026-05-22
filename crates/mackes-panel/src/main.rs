//! mackes-panel — top status bar + bottom dock for Mackes XFCE Workstation.
//!
//! Phase 1.1 + 1.2: `PatternFly` tokens loaded from the shipped
//! `data/css/tokens.css` and the top bar gains three layout slots
//! (left / center / right) so future phases drop the appmenu, clock,
//! and status cluster into named regions.
//!
//!   ┌──────────────────────────────────────────┐  top bar  (20 px Dock)
//!   │ [left]            [center]      [right]  │
//!   ├──────────────────────────────────────────┤
//!   │      <desktop window — wallpaper>        │
//!   ├──────────────────────────────────────────┤  bottom dock (80 px Dock)
//!   └──────────────────────────────────────────┘

#![forbid(unsafe_code)]

mod admin_menu;
mod app_module;
mod app_switcher;
mod apple_menu;
mod clipboard_manager;
mod config_store;
mod desktop_files;
mod dock;
mod dock_dnd;
// Drawer-to-Rust port (Phase 4.3): still using mackes/drawer.py
// (Python). The 1.1.0 `start_menu.rs` is the first Rust slice of
// the eventual drawer surface; remaining sections continue to live
// in Python until the full production port lands.
mod hero;
mod i3_cluster;
mod icon_mapper;
mod icons;
mod mesh_module;
mod mesh_sync;
mod network_manager;
mod notification_bell;
mod notification_center;
mod recents;
mod recover;
mod root_menu;
mod start_menu;
mod status_cluster;
mod strut;
mod toasts;
mod top_bar;
mod watermark;
mod weather;
// window_buttons retired in 1.1.0 (Q11/Q12): i3 has no built-in
// window buttons, modern GTK/Qt apps draw their own CSD chrome, and
// the design's Win10 layout doesn't carry a top-bar control cluster.
// Mod+q / Mod+f / Mod+space (data/i3/config) cover the actions for
// every X11 app, CSD or otherwise.
mod windows;

// Test-only synchronization for env-var-mutating tests across all
// sibling modules in this binary. Without this, parallel tests that
// read/write `HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME` race against
// each other and intermittently fail.
#[cfg(test)]
mod test_env;

use std::path::{Path, PathBuf};

use gdk::prelude::*;
use gdk_pixbuf::Pixbuf;
use gtk::prelude::*;

/// 1.1.0 Win10 layout: single 40 px bottom taskbar (Q3 lock).
/// Replaces the prior 20 px top bar + 48 px Plank-parity dock split.
/// Layout (left → center → right): Start + pinned apps | i3 cluster |
/// status cluster + clock.
const TASKBAR_HEIGHT_PX: i32 = 40;

/// Legacy constants retained for backward compat with any code paths
/// still referencing them. New code should use `TASKBAR_HEIGHT_PX`.
const TOP_BAR_HEIGHT_PX: i32 = 20;
const DOCK_PADDING_PX: i32 = 8;
const APP_ID: &str = "shell.mackes.Panel";

/// Backup chrome surface so the panel renders even when no token CSS is
/// installed (e.g. running the binary out of `target/release` against an
/// uninstalled tree). Real styling comes from `tokens.css` loaded below.
/// Placeholder design system loaded BEFORE `tokens.css` / `mackes.css`.
/// On production installs the latter override this; in dev trees
/// (running the panel out of `target/release/` against a workstation
/// that hasn't installed mackes-shell) these rules keep the chrome from
/// degrading to default-GTK gray-on-gray. Hex colors here mirror the
/// `PatternFly` v6 dark surfaces + Mackes accent that `tokens.css` ships,
/// so the visual remains consistent if the token file goes missing.
const PLACEHOLDER_CSS: &[u8] = b"
    /* --- Surfaces ----------------------------------------------------- */
    window#mackes-top-bar {
        background-color: #151515;
        border-bottom: 1px solid rgba(244, 244, 244, 0.06);
    }
    window#mackes-dock {
        background-color: rgba(21, 21, 21, 0.97);
        border-top: 1px solid rgba(244, 244, 244, 0.08);
    }
    window#mackes-panel-desktop {
        background-color: #151515;
    }
    window#mackes-top-bar label,
    window#mackes-top-bar button,
    window#mackes-dock label,
    window#mackes-dock button {
        color: #f4f4f4;
        background-color: transparent;
    }

    /* --- Button reset for the top bar -------------------------------- */
    window#mackes-top-bar button {
        padding: 4px 8px;
        border: none;
        border-radius: 4px;
        box-shadow: none;
        background-image: none;
        transition: background-color 180ms cubic-bezier(0.2, 0, 0, 1);
    }
    window#mackes-top-bar button:hover {
        background-color: rgba(244, 244, 244, 0.08);
    }
    window#mackes-top-bar button:active,
    window#mackes-top-bar button:checked {
        background-color: rgba(43, 154, 243, 0.14);
        color: #2b9af3;
    }
    #mackes-apple-menu-button { padding: 4px 10px; margin-left: 4px; }
    #mackes-top-clock { padding: 0 12px; margin: 0 4px; }
    #mackes-top-clock-label {
        font-family: 'Red Hat Mono', 'JetBrains Mono', monospace;
        font-size: 12px;
        font-weight: 600;
        font-feature-settings: 'tnum';
        letter-spacing: 0.02em;
    }
    #mackes-status-cluster { margin-right: 6px; }
    #mackes-status-cluster button {
        padding: 2px 6px;
        margin: 0 1px;
        min-height: 0;
    }
    #mackes-status-cluster button box { spacing: 4px; }
    #mackes-status-value {
        font-family: 'Red Hat Mono', 'JetBrains Mono', monospace;
        font-size: 12px;
        font-weight: 600;
        font-feature-settings: 'tnum';
        color: #f4f4f4;
        margin-left: 2px;
    }
    /* Probe failed: dim icon + label, em-dash already in the label. */
    #mackes-status-cluster button.mackes-status-degraded image,
    #mackes-status-cluster button.mackes-status-degraded #mackes-status-value {
        opacity: 0.45;
    }
    #mackes-status-cluster button.mackes-status-degraded #mackes-status-value {
        color: #a8a8a8;
    }

    /* Phase 8.7 - window-management buttons (min/max/close) */
    #mackes-window-buttons {
        margin-left: 8px;
        margin-right: 2px;
    }
    #mackes-window-buttons button {
        padding: 2px 6px;
        margin: 0 1px;
        min-height: 0;
        border-radius: 4px;
    }
    #mackes-window-buttons button.mackes-window-button-disabled image,
    #mackes-window-buttons button.mackes-window-button-disabled label {
        opacity: 0.45;
    }
    #mackes-window-button-close:hover {
        background-color: rgba(250, 77, 86, 0.20);
    }

    /* --- Dock items + state dot -------------------------------------- */
    .mackes-dock-strip > *,
    #mackes-dock-tasklist > * {
        padding: 0 4px;
        transition: background-color 180ms cubic-bezier(0.2, 0, 0, 1),
                    opacity          180ms cubic-bezier(0.2, 0, 0, 1);
    }
    .mackes-dock-strip > *:hover,
    #mackes-dock-tasklist > *:hover {
        background-color: rgba(244, 244, 244, 0.07);
        border-radius: 6px;
    }
    /* Phase 5.7 -- drag-and-drop visual feedback. .dragging dims the
       source row to ~half opacity for the duration of the drag;
       .drop-hover outlines the drop target with the brand accent so
       the user can see where the release will land. */
    .mackes-dock-strip > *.dragging,
    #mackes-dock-tasklist > *.dragging,
    eventbox.dragging {
        opacity: 0.5;
    }
    .mackes-dock-strip > *.drop-hover,
    #mackes-dock-tasklist > *.drop-hover,
    eventbox.drop-hover,
    #mackes-taskbar-pinned.drop-hover {
        background-color: rgba(43, 154, 243, 0.18);
        border-radius: 6px;
        box-shadow: inset 0 0 0 1px rgba(43, 154, 243, 0.55);
    }
    #mackes-dock-state-dot {
        min-height: 2px;
        background-color: transparent;
        border: none;
        border-radius: 1px;
        margin: 1px 0 0 0;
        transition: background-color 180ms cubic-bezier(0.2, 0, 0, 1);
    }
    #mackes-dock-state-dot.muted { background-color: rgba(244, 244, 244, 0.32); }
    #mackes-dock-state-dot.accent { background-color: #2b9af3; }
    #mackes-dock-state-dot.alert  { background-color: #fa4d56; }

    /* --- Popovers ---------------------------------------------------- */
    popover {
        background-color: #1b1d21;
        border: 1px solid rgba(244, 244, 244, 0.10);
        border-radius: 8px;
        padding: 0;
    }
    popover > * {
        padding: 14px 16px;
        color: #f4f4f4;
        font-family: 'Red Hat Text', system-ui, sans-serif;
        font-size: 13px;
        line-height: 18px;
    }
    #mackes-status-popover-title,
    #mackes-weather-title {
        font-family: 'Red Hat Display', 'IBM Plex Sans', system-ui, sans-serif;
        font-size: 14px;
        font-weight: 700;
        letter-spacing: -0.005em;
        color: #f4f4f4;
        margin-bottom: 2px;
    }
    #mackes-status-popover-summary,
    #mackes-weather-footer {
        color: #a8a8a8;
        font-size: 12px;
        line-height: 16px;
    }
    #mackes-weather-temp {
        font-family: 'Red Hat Display', 'IBM Plex Sans', system-ui, sans-serif;
        font-size: 32px;
        font-weight: 300;
        letter-spacing: -0.02em;
    }
    #mackes-status-popover-drawer {
        background-color: rgba(43, 154, 243, 0.12);
        color: #2b9af3;
        border: 1px solid rgba(43, 154, 243, 0.30);
        border-radius: 6px;
        padding: 6px 12px;
        margin-top: 8px;
        font-weight: 600;
        transition: background-color 180ms cubic-bezier(0.2, 0, 0, 1);
    }
    #mackes-status-popover-drawer:hover {
        background-color: rgba(43, 154, 243, 0.20);
    }

    /* --- Menus ------------------------------------------------------- */
    menu, #mackes-apple-menu, #mackes-launcher-menu, #mackes-tasklist-menu {
        background-color: #1b1d21;
        border: 1px solid rgba(244, 244, 244, 0.10);
        border-radius: 8px;
        padding: 6px 0;
        color: #f4f4f4;
        font-family: 'Red Hat Text', system-ui, sans-serif;
        font-size: 13px;
    }
    menu menuitem {
        padding: 6px 14px;
        margin: 0 6px;
        border-radius: 4px;
        transition: background-color 180ms cubic-bezier(0.2, 0, 0, 1);
    }
    menu menuitem:hover {
        background-color: rgba(43, 154, 243, 0.18);
    }
    menu menuitem:disabled {
        color: #a8a8a8;
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.04em;
        padding: 8px 14px 4px 14px;
    }
    menu separator {
        background-color: rgba(244, 244, 244, 0.08);
        min-height: 1px;
        margin: 4px 8px;
    }
";

/// Standard install location for the shipped `PatternFly` tokens.
const TOKEN_CSS_PATHS: &[&str] = &[
    "/usr/share/mackes-shell/data/css/tokens.css",
    "/usr/share/mackes-shell/data/css/mackes.css",
];

/// Fallback wallpaper used when the active preset's path is missing.
const DEFAULT_WALLPAPER: &str = "/usr/share/mackes-shell/branding/standard-wallpaper.png";

fn main() -> glib::ExitCode {
    // Phase 10.6.8 — `--recover` is a read-only preview of the
    // birthright rollback ledger. It prints which steps would be
    // reversed (and the dnf install argv the operator should run via
    // `mackes recover all`) and exits BEFORE we touch GTK. Done by
    // hand rather than via clap to keep mackes-panel dependency-light.
    let argv: Vec<String> = std::env::args().collect();
    if argv.iter().any(|a| a == "--recover" || a == "-R") {
        let code = recover::run_preview();
        return glib::ExitCode::from(code);
    }
    // Phase 6.1 — Super+Tab handler. The i3 config in
    // `data/i3/config.d/mackes-defaults.conf` binds `$mod+Tab` to
    // `mackes-panel --app-switcher`; the binary spins up an isolated
    // GTK main loop, runs the modal, and exits the moment the user
    // commits or dismisses. No `gtk::Application` scaffolding so the
    // overlay never collides with a running panel instance.
    if argv.iter().any(|a| a == "--app-switcher") {
        app_switcher::run_switcher_modal();
        return glib::ExitCode::from(0i32);
    }
    if argv.iter().any(|a| a == "--help" || a == "-h") {
        print_cli_help();
        return glib::ExitCode::from(0i32);
    }

    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::FLAGS_NONE)
        .build();

    app.connect_activate(build_surfaces);

    // Quit cleanly on SIGTERM / SIGINT. unix_signal_add_local runs on the
    // GTK main thread (gtk::Application is !Send). Without this systemd
    // would SIGKILL us after TimeoutStopSec.
    let app_for_sigterm = app.clone();
    glib::unix_signal_add_local(libc::SIGTERM, move || {
        app_for_sigterm.quit();
        glib::ControlFlow::Break
    });
    let app_for_sigint = app.clone();
    glib::unix_signal_add_local(libc::SIGINT, move || {
        app_for_sigint.quit();
        glib::ControlFlow::Break
    });

    app.run()
}

/// Minimal CLI help. Surfaced by `--help` / `-h` (so a user who Cmd-Tabs
/// here from `mackes --help` doesn't see a GTK window spawn). Listing is
/// short on purpose — the real reference doc is `docs/help/cli-reference.md`.
fn print_cli_help() {
    println!("mackes-panel — Mackes XFCE Workstation panel\n");
    println!("USAGE:");
    println!("    mackes-panel                launch the panel (default)");
    println!("    mackes-panel --recover      print the birthright rollback");
    println!("                                preview and exit (Phase 10.6.8)");
    println!("    mackes-panel --app-switcher open the Super+Tab app switcher");
    println!("                                overlay and exit (Phase 6.1)");
    println!("    mackes-panel --help         this message");
}

fn build_surfaces(app: &gtk::Application) {
    install_global_styling();
    let cfg = config_store::load_or_default();

    // Phase 2.3 hot-reload watcher. The returned monitor must outlive
    // the GTK main loop; leak it intentionally. (Dropping the monitor
    // cancels the watch, which we don't want for the panel's lifetime.)
    let monitor = config_store::watch(|new_cfg| match new_cfg {
        Some(_cfg) => eprintln!("mackes-panel: panel.toml reloaded"),
        None => eprintln!("mackes-panel: panel.toml went away or failed to parse"),
    });
    std::mem::forget(monitor);

    // 1.1.0 (#16) — render one wallpaper layer per connected monitor.
    // The Win10 watermark is rendered as an overlay child on the
    // primary monitor's window only (current `apply_wallpaper`
    // behavior); secondary monitors get the wallpaper without the
    // watermark, mirroring Win10's "Recent Windows-1 desktop"
    // convention.
    let monitors = all_monitor_geometries();
    if monitors.is_empty() {
        let geom = FallbackGeometry::default();
        build_desktop(app, &geom, true);
        build_bottom_taskbar(app, &geom, &cfg);
        return;
    }
    for (i, geom) in monitors.iter().enumerate() {
        build_desktop(app, geom, i == 0);
    }
    // 1.1.0 (Q1/Q2/Q3 lock): single bottom taskbar replaces the
    // prior top bar + Plank dock split. The taskbar renders on the
    // primary monitor only (Win10 behavior — secondary monitors
    // show wallpaper only).
    build_bottom_taskbar(app, &monitors[0], &cfg);
}

/// Load `PatternFly` tokens (data/css/tokens.css) into the screen-wide
/// `StyleContext` so every window we build inherits the dark surfaces,
/// font stack, and accent palette. Followed by the inline backup CSS so
/// the panel chrome still renders on uninstalled / dev trees.
fn install_global_styling() {
    let Some(screen) = gdk::Screen::default() else {
        return;
    };

    for path in TOKEN_CSS_PATHS {
        if !Path::new(path).is_file() {
            continue;
        }
        let provider = gtk::CssProvider::new();
        if provider.load_from_path(path).is_ok() {
            gtk::StyleContext::add_provider_for_screen(
                &screen,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    // Load the active preset's accent override CSS so the GTK panel
    // stays in sync with the Iced apps and the Python Workbench.
    if let Some(preset) = mackes_theme::read_active_preset() {
        if let Some(accent_path) = mackes_theme::locate_accent_css(&preset) {
            let provider = gtk::CssProvider::new();
            if provider
                .load_from_path(accent_path.to_str().unwrap_or(""))
                .is_ok()
            {
                gtk::StyleContext::add_provider_for_screen(
                    &screen,
                    &provider,
                    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 5,
                );
            }
        }
    }

    // Inline backup — overlays the tokens with our panel-specific bits
    // (window IDs, hairline borders). Higher priority so it wins on the
    // panel IDs without stomping general token rules.
    let backup = gtk::CssProvider::new();
    if backup.load_from_data(PLACEHOLDER_CSS).is_ok() {
        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &backup,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 10,
        );
    }
}

/// Fullscreen wallpaper layer that replaces xfdesktop. `with_watermark`
/// controls whether the Win10-style update watermark is overlaid in
/// the lower-right — primary monitor gets the watermark, secondary
/// monitors get wallpaper only (1.1.0 #16 multi-monitor lock).
///
/// Phase 8.4 — right-clicking anywhere on the wallpaper drops the
/// `root_menu` (Change wallpaper / Open mesh share / Send file to peer
/// / Display settings). We bind `button-press-event` directly on this
/// Desktop-type window rather than `XGrabButton`-ing the X11 root,
/// because:
///
///  1. The window already covers every pixel of the wallpaper.
///  2. `WindowTypeHint::Desktop` keeps it strictly below every other
///     window, so panel / app clicks still take priority by Z-order.
///  3. GTK gives us event delivery without an external `x11`/`xcb`
///     dependency or manual root-grab teardown on shutdown.
///
/// Left-click falls through (matches macOS's "click on Desktop does
/// nothing"); middle and other buttons also fall through. Only
/// `event.button() == 3` opens the menu.
fn build_desktop(app: &gtk::Application, geom: &FallbackGeometry, with_watermark: bool) {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("mackes-panel-desktop")
        .decorated(false)
        .skip_taskbar_hint(true)
        .skip_pager_hint(true)
        .resizable(false)
        .accept_focus(false)
        .type_hint(gdk::WindowTypeHint::Desktop)
        .build();
    window.set_widget_name("mackes-desktop");
    window.set_default_size(geom.width, geom.height);
    window.move_(geom.x, geom.y);
    apply_wallpaper(&window, geom, with_watermark);

    // GTK toplevels don't pick up BUTTON_PRESS_MASK by default — and
    // because we set `accept_focus(false)` above, GTK is even more
    // conservative about the events it requests. Add the masks
    // explicitly so right-click reaches our handler.
    window.add_events(gdk::EventMask::BUTTON_PRESS_MASK);

    window.connect_button_press_event(|_, ev| {
        if ev.button() == 3 {
            let menu = root_menu::build();
            menu.show_all();
            menu.popup_easy(3, ev.time());
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });

    window.show_all();
}

/// 1.1.0 — Single bottom taskbar replacing the 1.0.x top bar + Plank
/// dock split (Q1/Q2/Q3 lock). Layout:
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────────┐
/// │ [M] [pinned…]      [SPLIT][LAYOUT][WINDOW]      [tray…] [clock] │
/// └─────────────────────────────────────────────────────────────────┘
/// ```
///
/// - **Left**: Start button (`apple_menu_button` — left-click drops the
///   apple menu; right-click drops the 9-item Fedora admin menu) + the
///   pinned-apps dock strip (re-skin of the prior Plank dock, same
///   logic + sources, just 18 px icons in 40 px slots per Q9).
/// - **Center**: i3 cluster (`i3_cluster::build()`).
/// - **Right**: status cluster (`status_cluster::build()`) + the
///   two-line clock (Q13 lock — `top_bar::clock()` carries the date
///   line per its 1.1.0 update).
///
/// `_NET_WM_STRUT_PARTIAL` is set via the bottom-strut helper so i3
/// reserves the row for non-floating windows.
fn build_bottom_taskbar(
    app: &gtk::Application,
    geom: &FallbackGeometry,
    cfg: &mackes_config::PanelConfig,
) {
    // .desktop scan shared with the dock strip — only one filesystem
    // walk per panel boot.
    let by_id: std::rc::Rc<std::collections::HashMap<String, desktop_files::DesktopEntry>> =
        std::rc::Rc::new(
            desktop_files::scan()
                .into_iter()
                .map(|e| (e.id.clone(), e))
                .collect(),
        );

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("mackes-panel-taskbar")
        .decorated(false)
        .skip_taskbar_hint(true)
        .skip_pager_hint(true)
        .resizable(false)
        .type_hint(gdk::WindowTypeHint::Dock)
        .build();
    window.set_widget_name("mackes-taskbar");
    window.set_default_size(geom.width, TASKBAR_HEIGHT_PX);
    window.move_(geom.x, geom.y + geom.height - TASKBAR_HEIGHT_PX);

    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    bar.set_widget_name("mackes-taskbar-layout");

    // ---- LEFT: Start + pinned-apps strip ------------------------------
    let left = build_slot("mackes-taskbar-left");
    left.set_spacing(2);
    left.pack_start(&top_bar::apple_menu_button(), false, false, 0);

    let pinned = build_slot("mackes-taskbar-pinned");
    pinned.set_spacing(0);
    let tasklist = build_tasklist_strip();
    tasklist.set_spacing(0);
    // Phase 5.7 — install the pinned-strip drop target ONCE (its
    // children are torn down + rebuilt every 2 s, but the strip
    // itself outlives the panel). Tasklist items dragged here call
    // `mackes_config::pin_app` via `config_store::with_mut`.
    dock_dnd::attach_pinned_strip_target(&pinned);
    refresh_dock(&pinned, &tasklist, cfg, &by_id);
    left.pack_start(&pinned, false, false, 0);
    left.pack_start(&tasklist, false, false, 0);

    // ---- CENTER: hero slot + i3 cluster ------------------------------
    // 1.1.0 (Q10): the focused-app hero lives immediately to the left
    // of the i3 cluster. Hidden by default; revealed via GTK's native
    // slide-left transition when an i3 window::focus event arrives.
    let center = build_slot("mackes-taskbar-center");
    center.pack_start(&hero::build(), false, false, 0);
    center.pack_start(&i3_cluster::build(), false, false, 0);

    // ---- RIGHT: NM icon + tray (status cluster) + bell + clock -------
    // 1.1.0 (#24): the NetworkManager button sits to the LEFT of the
    // status cluster — live interface state + click-popover with the
    // full nmcli surface (connections, Wi-Fi scan/connect, airplane,
    // editor launch).
    // 1.1.0 (Rust-Desktop notification handoff): bell + unread badge
    // sits between the status cluster and the clock. Click opens the
    // mesh-synced Notification Center modal (70% screen, dimmed
    // backdrop, Esc / outside-click dismiss).
    let right = build_slot("mackes-taskbar-right");
    right.set_spacing(2);
    right.pack_start(&network_manager::build(), false, false, 0);
    right.pack_start(&status_cluster::build(), false, false, 0);
    right.pack_start(&notification_bell::build(), false, false, 0);
    right.pack_start(&top_bar::clock(), false, false, 0);

    bar.pack_start(&left, false, false, 0);
    bar.set_center_widget(Some(&center));
    bar.pack_end(&right, false, false, 0);

    window.add(&bar);
    window.show_all();

    // Reserve the row via `_NET_WM_STRUT_PARTIAL` so i3 + xfsettingsd
    // don't tile windows beneath us.
    strut::set_bottom_strut(&window, geom, TASKBAR_HEIGHT_PX);

    // 2 s refresh on the pinned + tasklist segments — same cadence as
    // the prior dock. Live-reloads panel.toml so Pin / Unpin actions
    // from right-click menus surface promptly.
    {
        // The dock refresh timer is the last consumer of `pinned` /
        // `tasklist`, so move them directly into the closure rather
        // than cloning.
        let by_id_for_timer = std::rc::Rc::clone(&by_id);
        glib::timeout_add_seconds_local(2, move || {
            let live_cfg = config_store::load_or_default();
            refresh_dock(&pinned, &tasklist, &live_cfg, &by_id_for_timer);
            glib::ControlFlow::Continue
        });
    }
    // Track realized height — the GTK layout may push past
    // TASKBAR_HEIGHT_PX once fonts + icons settle. Same polling
    // pattern as the legacy `build_top_bar` (size-allocate is
    // unreliable on Dock-hint toplevels).
    {
        let geom_for_strut = *geom;
        let last_h: std::rc::Rc<std::cell::Cell<i32>> =
            std::rc::Rc::new(std::cell::Cell::new(TASKBAR_HEIGHT_PX));
        glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
            let h = window.allocated_height();
            if h > 0 && h != last_h.get() {
                last_h.set(h);
                let new_y = geom_for_strut.y + geom_for_strut.height - h;
                window.move_(geom_for_strut.x, new_y);
                strut::set_bottom_strut(&window, &geom_for_strut, h);
            }
            glib::ControlFlow::Continue
        });
    }
}

/// Dead code as of 1.1.0 — kept compiling so any external reference
/// (tests, helper scripts) still links. The actual surface is gone;
/// `build_surfaces` calls `build_bottom_taskbar` instead. Remove
/// entirely on the next release cut.
#[allow(dead_code)]
fn build_top_bar(app: &gtk::Application, geom: &FallbackGeometry) {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("mackes-panel-top")
        .decorated(false)
        .skip_taskbar_hint(true)
        .skip_pager_hint(true)
        .resizable(false)
        .type_hint(gdk::WindowTypeHint::Dock)
        .build();
    window.set_widget_name("mackes-top-bar");
    window.set_default_size(geom.width, TOP_BAR_HEIGHT_PX);
    window.move_(geom.x, geom.y);

    // Three-slot horizontal layout: left / center / right.
    // `gtk::Box` with center widget property gives us a true three-region
    // layout where the center stays centered even when left/right vary.
    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    bar.set_widget_name("mackes-top-bar-layout");

    let left = build_slot("mackes-top-left");
    let center = build_slot("mackes-top-center");
    let right = build_slot("mackes-top-right");

    left.pack_start(&top_bar::apple_menu_button(), false, false, 0);
    center.pack_start(&top_bar::clock(), false, false, 0);
    right.pack_start(&status_cluster::build(), false, false, 0);
    // 1.1.0: window_buttons retired (i3-native + CSD only).

    bar.pack_start(&left, false, false, 0);
    bar.set_center_widget(Some(&center));
    bar.pack_end(&right, false, false, 0);

    window.add(&bar);
    window.show_all();
    // Strut height has to match the realized window height, not the
    // requested TOP_BAR_HEIGHT_PX — the bar grows past 20 px once the
    // clock font + icon padding lay out, and a stale 20-px strut leaves
    // maximized windows overlapping the top edge by the delta.
    //
    // GTK3's "size-allocate" signal does not fire reliably on top-level
    // Dock-hint windows (verified empirically: the closure registered
    // here never ran). A 500 ms polling timer is reliable and cheap;
    // it gates the xprop call on a real height change so we don't churn.
    strut::set_top_strut(&window, geom, TOP_BAR_HEIGHT_PX);
    {
        let geom_for_strut = *geom;
        let last_h: std::rc::Rc<std::cell::Cell<i32>> =
            std::rc::Rc::new(std::cell::Cell::new(TOP_BAR_HEIGHT_PX));
        glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
            let h = window.allocated_height();
            if h > 0 && h != last_h.get() {
                last_h.set(h);
                strut::set_top_strut(&window, &geom_for_strut, h);
            }
            glib::ControlFlow::Continue
        });
    }
}

/// Dead code as of 1.1.0 — superseded by `build_bottom_taskbar` which
/// fuses the dock + top bar contents into a single 40 px surface. Kept
/// in-tree for one release cycle so any helper that calls into it
/// keeps compiling; remove at 1.2.0.
#[allow(dead_code)]
fn build_bottom_dock(
    app: &gtk::Application,
    geom: &FallbackGeometry,
    cfg: &mackes_config::PanelConfig,
) {
    // .desktop scan is the only filesystem walk; share across ticks.
    let by_id: std::rc::Rc<std::collections::HashMap<String, desktop_files::DesktopEntry>> =
        std::rc::Rc::new(
            desktop_files::scan()
                .into_iter()
                .map(|e| (e.id.clone(), e))
                .collect(),
        );

    // 5 % spacing tweak (14 → 15) + 24 px end-cap margin per design.
    let strip = build_slot("mackes-dock-strip");
    strip.set_halign(gtk::Align::Center);
    strip.set_spacing(15);
    let tasklist = build_tasklist_strip();
    tasklist.set_spacing(15);

    // Initial render before measuring whether the dock is non-empty.
    refresh_dock(&strip, &tasklist, cfg, &by_id);
    let static_count = strip.children().len();
    let live_count = tasklist.children().len();
    if static_count == 0 && live_count == 0 {
        return;
    }

    let height = dock::DOCK_ICON_PX + DOCK_PADDING_PX;
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("mackes-panel-dock")
        .decorated(false)
        .skip_taskbar_hint(true)
        .skip_pager_hint(true)
        .resizable(false)
        .type_hint(gdk::WindowTypeHint::Dock)
        .build();
    window.set_widget_name("mackes-dock");
    window.set_default_size(geom.width, height);
    window.move_(geom.x, geom.y + geom.height - height);

    let row = gtk::Box::new(gtk::Orientation::Horizontal, 15);
    row.set_halign(gtk::Align::Center);
    row.set_margin_start(24);
    row.set_margin_end(24);
    row.pack_start(&strip, false, false, 0);
    row.pack_start(&tasklist, false, false, 0);

    window.add(&row);
    window.show_all();

    // Poll every 2 s: rebuilds both segments from current window state.
    // 1.0.7: re-reads panel.toml each tick so Pin/Unpin actions from
    // the dock right-click menus (and from Workbench → Window Manager)
    // surface within ~2 s without a separate file-watch path.
    {
        let by_id_for_timer = std::rc::Rc::clone(&by_id);
        glib::timeout_add_seconds_local(2, move || {
            let live_cfg = config_store::load_or_default();
            refresh_dock(&strip, &tasklist, &live_cfg, &by_id_for_timer);
            glib::ControlFlow::Continue
        });
    }
    // Same allocated-height tracking as the top bar — polling because
    // GTK3's size-allocate signal doesn't fire reliably on Dock-hint
    // toplevels. Initial strut already set above (via the requested
    // `height`); this catches the case where the dock grows once the
    // tasklist segment renders, or where the user installs a
    // Mackes-Carbon override that ships oversized SVGs.
    strut::set_bottom_strut(&window, geom, height);
    {
        let geom_for_strut = *geom;
        let last_h: std::rc::Rc<std::cell::Cell<i32>> =
            std::rc::Rc::new(std::cell::Cell::new(height));
        glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
            let h = window.allocated_height();
            if h > 0 && h != last_h.get() {
                last_h.set(h);
                let new_y = geom_for_strut.y + geom_for_strut.height - h;
                window.move_(geom_for_strut.x, new_y);
                strut::set_bottom_strut(&window, &geom_for_strut, h);
            }
            glib::ControlFlow::Continue
        });
    }
}

/// Lower-case `WM_CLASS` token a pinned launcher matches against. Uses
/// the entry's explicit `StartupWMClass=` when set, otherwise the
/// `.desktop` basename minus extension (e.g. `firefox.desktop` → "firefox").
fn launcher_class(entry: &desktop_files::DesktopEntry) -> String {
    entry
        .startup_wm_class
        .as_deref()
        .unwrap_or_else(|| entry.id.trim_end_matches(".desktop"))
        .to_ascii_lowercase()
}

/// Snapshot consumed by every refresh pass: open windows, their classes,
/// and the currently active window id. Caching the per-tick lookup
/// avoids re-running `xprop` for every launcher × every tick.
struct DockSnapshot {
    windows: Vec<windows::OpenWindow>,
    classes: std::collections::HashMap<String, String>,
    active: Option<String>,
}

impl DockSnapshot {
    fn capture() -> Self {
        let windows = windows::list_open_windows();
        let mut classes = std::collections::HashMap::with_capacity(windows.len());
        for w in &windows {
            if let Some(c) = windows::window_wm_class(&w.window_id) {
                classes.insert(w.window_id.clone(), c.to_ascii_lowercase());
            }
        }
        Self {
            windows,
            classes,
            active: windows::active_window_id_str(),
        }
    }

    fn windows_for_class(&self, class: &str) -> Vec<&windows::OpenWindow> {
        self.windows
            .iter()
            .filter(|w| self.classes.get(&w.window_id).is_some_and(|c| c == class))
            .collect()
    }
}

/// Rebuild both dock segments from the current window snapshot.
/// Idempotent — every tick clears children and re-adds.
fn refresh_dock(
    strip: &gtk::Box,
    tasklist: &gtk::Box,
    cfg: &mackes_config::PanelConfig,
    by_id: &std::collections::HashMap<String, desktop_files::DesktopEntry>,
) {
    for c in strip.children() {
        strip.remove(&c);
    }
    for c in tasklist.children() {
        tasklist.remove(&c);
    }

    let snap = DockSnapshot::capture();
    let mut pinned_classes: std::collections::HashSet<String> = std::collections::HashSet::new();

    // `cfg.dock.items` is the source-of-truth ordering; the visible
    // index drives the drag payload (Phase 5.7). Mesh entries with an
    // unrecognised id are skipped without consuming a slot, so we
    // increment manually rather than using `.enumerate()`.
    let mut slot_index: usize = 0;
    for item in &cfg.dock.items {
        match item {
            mackes_config::DockItem::App { desktop } => {
                let Some(entry) = by_id.get(desktop) else {
                    eprintln!("mackes-panel: dock item references unknown .desktop: {desktop}");
                    continue;
                };
                let class = launcher_class(entry);
                let app_windows = snap.windows_for_class(&class);
                let widget = build_launcher_item(entry, &app_windows, snap.active.as_deref());
                // Phase 5.7 — drag source + drop target. Source emits
                // the current slot index; drop calls
                // `mackes_config::reorder_dock(cfg, from, slot_index)`.
                dock_dnd::attach_dock_slot(&widget, slot_index);
                pinned_classes.insert(class);
                strip.pack_start(&widget, false, false, 0);
                slot_index += 1;
            }
            mackes_config::DockItem::Mesh { id } => {
                if let Some(resource) = mesh_module::parse_id(id) {
                    let module = mesh_module::MeshModule::new(resource.clone());
                    let widget = dock::render_module(&module);
                    let module_for_click = module.clone();
                    let widget_for_anchor = widget.clone();
                    widget.connect_button_release_event(move |_, _| {
                        use dock::DockModule;
                        if let mackes_mesh_types::MeshResource::Peer { name, .. } =
                            module_for_click.resource()
                        {
                            let popover = mesh_module::build_peer_popover(
                                widget_for_anchor.upcast_ref::<gtk::Widget>(),
                                name,
                            );
                            popover.show_all();
                            popover.popup();
                        } else {
                            module_for_click.on_click();
                        }
                        glib::Propagation::Stop
                    });
                    // Mesh items participate in the same reorder grammar
                    // as App items — same atom, same index space.
                    dock_dnd::attach_dock_slot(&widget, slot_index);
                    strip.pack_start(&widget, false, false, 0);
                    slot_index += 1;
                } else {
                    eprintln!("mackes-panel: unrecognised mesh dock id: {id}");
                }
            }
        }
    }

    // Tasklist: every open window that ISN'T already grouped under a
    // pinned launcher AND isn't one of our own panel windows.
    for w in &snap.windows {
        if is_panel_owned_window(w) {
            continue;
        }
        let class = snap.classes.get(&w.window_id).cloned().unwrap_or_default();
        if pinned_classes.contains(&class) {
            continue;
        }
        tasklist.pack_start(
            &build_tasklist_item(w, &class, &snap, by_id),
            false,
            false,
            0,
        );
    }

    strip.show_all();
    tasklist.show_all();
}

/// Render one pinned-launcher dock entry. Uses the launcher's `.desktop`
/// for icon + categories (Carbon-only fallback) and computes state from
/// the supplied `app_windows` slice:
///   - any window is the active window → `Focused` (accent dot)
///   - one or more windows exist (none focused) → `Running` (muted dot)
///   - no windows → `Idle` (no dot)
///
/// Click: left → launch (or activate first matching window); right →
/// rich context menu (Open New / Bring to Front: <title> per-window /
/// Close All Windows).
fn build_launcher_item(
    entry: &desktop_files::DesktopEntry,
    app_windows: &[&windows::OpenWindow],
    active: Option<&str>,
) -> gtk::EventBox {
    let state = if app_windows.is_empty() {
        dock::DockState::Idle
    } else if app_windows
        .iter()
        .any(|w| Some(w.window_id.as_str()) == active)
    {
        dock::DockState::Focused
    } else {
        dock::DockState::Running
    };

    let mut module = app_module::AppModule::new(entry.clone());
    module.set_state(state);
    let widget = dock::render_module(&module);

    // Multi-window indicator: replace the default single-bar state-dot
    // with one tick per open window (1, 2, 3+). We modify the column's
    // last child (the dot row) in place.
    if app_windows.len() > 1 {
        if let Some(column) = widget.child().and_then(|c| c.downcast::<gtk::Box>().ok()) {
            let kids = column.children();
            if let Some(old_dot) = kids.last() {
                column.remove(old_dot);
                column.pack_start(
                    &multi_window_indicator(app_windows.len(), state),
                    false,
                    false,
                    0,
                );
            }
        }
    }

    let exec = entry.exec.clone();
    let class = launcher_class(entry);
    let name = entry.name.clone();
    let desktop_id = entry.id.clone();
    let terminal = entry.terminal;
    let windows_for_menu: Vec<(String, String)> = app_windows
        .iter()
        .map(|w| (w.window_id.clone(), w.title.clone()))
        .collect();
    widget.connect_button_press_event(move |_, ev| match ev.button() {
        1 => {
            let open = windows::list_open_windows();
            let first_match = open
                .iter()
                .find(|w| {
                    windows::window_wm_class(&w.window_id)
                        .is_some_and(|c| c.to_ascii_lowercase() == class)
                })
                .map(|w| w.window_id.clone())
                .or_else(|| windows::find_window_for_app(&name, &exec, &open));
            if let Some(window_id) = first_match {
                windows::toggle_window(&window_id);
            } else {
                top_bar::launch_exec(&exec, terminal);
            }
            glib::Propagation::Stop
        }
        3 => {
            launcher_context_menu(&name, &desktop_id, &exec, terminal, &windows_for_menu);
            glib::Propagation::Stop
        }
        _ => glib::Propagation::Proceed,
    });
    widget
}

/// Indicator under a launcher when it has multiple open windows: one
/// short Carbon bar per window, up to 3 (4+ collapses to 3 bars).
/// CSS class `mackes-dock-state-dot` + state class gets the same blue
/// accent / muted treatment as the single-window indicator.
fn multi_window_indicator(count: usize, state: dock::DockState) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 3);
    row.set_widget_name("mackes-dock-multi");
    row.set_halign(gtk::Align::Center);
    let visible = count.min(3);
    for _ in 0..visible {
        let dot = gtk::Frame::new(None);
        dot.set_widget_name("mackes-dock-state-dot");
        dot.set_size_request(6, 2);
        if let Some(cls) = state.dot_class() {
            dot.style_context().add_class(cls);
        }
        row.pack_start(&dot, false, false, 0);
    }
    row
}

/// Right-click menu for a pinned launcher. `desktop_id` is the
/// .desktop basename (e.g. `firefox.desktop`) — needed for the
/// Unpin action that rewrites panel.toml.
fn launcher_context_menu(
    name: &str,
    desktop_id: &str,
    exec: &str,
    terminal: bool,
    app_windows: &[(String, String)],
) {
    let menu = gtk::Menu::new();
    menu.set_widget_name("mackes-launcher-menu");

    let header = gtk::MenuItem::with_label(name);
    header.set_sensitive(false);
    menu.append(&header);
    menu.append(&gtk::SeparatorMenuItem::new());

    let open_new = gtk::MenuItem::with_label("Open New Window");
    let exec_owned = exec.to_owned();
    open_new.connect_activate(move |_| top_bar::launch_exec(&exec_owned, terminal));
    menu.append(&open_new);

    if !app_windows.is_empty() {
        menu.append(&gtk::SeparatorMenuItem::new());
        for (wid, title) in app_windows {
            let label = format!(
                "Bring to Front: {}",
                title.chars().take(40).collect::<String>()
            );
            let item = gtk::MenuItem::with_label(&label);
            let wid_owned = wid.clone();
            item.connect_activate(move |_| windows::activate_window(&wid_owned));
            menu.append(&item);
        }
        menu.append(&gtk::SeparatorMenuItem::new());
        let close_all = gtk::MenuItem::with_label("Close All Windows");
        let wids: Vec<String> = app_windows.iter().map(|(w, _)| w.clone()).collect();
        close_all.connect_activate(move |_| {
            for w in &wids {
                windows::close_window(w);
            }
        });
        menu.append(&close_all);
    }

    menu.append(&gtk::SeparatorMenuItem::new());

    // 1.1.0 (#14) — Carbon Icon Mapper integration. Right-click on
    // any dock app shows "Change icon…" which drops a popover with
    // every Mackes-Carbon icon as a tappable thumbnail; clicking
    // writes a user override to ~/.local/share/applications/<id>.
    let change_icon = gtk::MenuItem::with_label("Change icon…");
    let id_for_icon = desktop_id.to_owned();
    let name_for_icon = name.to_owned();
    change_icon.connect_activate(move |item| {
        // Anchor the popover to the menu item itself — gives a
        // sensible visual handoff from the right-click cascade.
        icon_mapper::open_for(
            item.upcast_ref::<gtk::Widget>(),
            &id_for_icon,
            &name_for_icon,
        );
    });
    menu.append(&change_icon);

    let unpin = gtk::MenuItem::with_label("Unpin from Dock");
    let id_owned = desktop_id.to_owned();
    unpin.connect_activate(move |_| config_store::unpin_app(&id_owned));
    menu.append(&unpin);

    menu.show_all();
    menu.popup_easy(3, gtk::current_event_time());
}

fn build_slot(name: &str) -> gtk::Box {
    let slot = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    slot.set_widget_name(name);
    slot
}

/// Empty tasklist container. `refresh_tasklist` populates it from the
/// current `wmctrl -lp` output.
fn build_tasklist_strip() -> gtk::Box {
    let strip = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    strip.set_widget_name("mackes-dock-tasklist");
    strip
}

/// True if the window belongs to mackes-panel itself (top bar, dock,
/// desktop wallpaper, transient popovers) OR is a desktop-shell window
/// like xfdesktop's "Desktop" toplevel. Filtered out of the tasklist
/// so the panel doesn't render its own surfaces as tasks.
fn is_panel_owned_window(w: &windows::OpenWindow) -> bool {
    if w.title.is_empty() || w.title == "mackes-panel" {
        return true;
    }
    if w.title.starts_with("mackes-panel-") {
        return true;
    }
    // Desktop-shell windows (xfdesktop, our own wallpaper layer). These
    // have a Desktop window-type hint but wmctrl lists them anyway; the
    // user doesn't think of them as apps.
    if w.title.eq_ignore_ascii_case("desktop") || w.title.eq_ignore_ascii_case("xfdesktop") {
        return true;
    }
    // Cross-check by PID: the panel's own windows share our pid.
    w.pid == std::process::id()
}

/// Build one tasklist entry for an open window. Visual structure mirrors
/// `dock::render_module` exactly so launcher items and tasklist items
/// align on the same baseline. Icon resolution:
///   1. Look up a `.desktop` entry whose `StartupWMClass` or basename
///      matches this window's `WM_CLASS` — gets us a Carbon-mapped icon
///      and category metadata for fallback.
///   2. If no `DesktopEntry` match, derive icon from `WM_CLASS` alone
///      (still Carbon-only, falls through to `applications-other-symbolic`).
///
/// Click bindings:
///   - left:  `windows::toggle_window` (activate, or minimize if focused)
///   - right: context menu with Bring to Front / Maximize / Restore /
///     Minimize / Close.
fn build_tasklist_item(
    w: &windows::OpenWindow,
    class: &str,
    snap: &DockSnapshot,
    by_id: &std::collections::HashMap<String, desktop_files::DesktopEntry>,
) -> gtk::EventBox {
    // Find a DesktopEntry whose launcher_class matches this window's
    // WM_CLASS; gives us its Icon + Categories for Carbon resolution.
    let entry: Option<&desktop_files::DesktopEntry> =
        by_id.values().find(|e| launcher_class(e) == class);
    let icon_name = entry.and_then(|e| e.icon.as_deref()).unwrap_or(class);
    let categories: &[String] = entry.map_or(&[][..], |e| &e.categories);

    let is_focused = snap.active.as_deref() == Some(w.window_id.as_str());
    let state = if is_focused {
        dock::DockState::Focused
    } else {
        dock::DockState::Running
    };

    let event_box = gtk::EventBox::new();
    event_box.set_widget_name("mackes-tasklist-item");
    event_box.set_above_child(true);
    event_box.set_tooltip_text(Some(&w.title));

    let column = gtk::Box::new(gtk::Orientation::Vertical, 2);
    column.set_widget_name("mackes-dock-item-column");

    let overlay = gtk::Overlay::new();
    overlay.set_size_request(dock::DOCK_ICON_PX, dock::DOCK_ICON_PX);
    let icon_widget: gtk::Widget =
        icons::load_with_fallback(Some(icon_name), categories, dock::DOCK_ICON_PX).map_or_else(
            || {
                let label = w.title.chars().take(12).collect::<String>();
                gtk::Label::new(Some(&label)).upcast::<gtk::Widget>()
            },
            |pb| gtk::Image::from_pixbuf(Some(&pb)).upcast::<gtk::Widget>(),
        );
    overlay.add(&icon_widget);
    column.pack_start(&overlay, false, false, 0);

    // State dot matches the launcher's: accent (blue) when focused,
    // muted otherwise. Always running for tasklist entries.
    let dot = gtk::Frame::new(None);
    dot.set_widget_name("mackes-dock-state-dot");
    dot.set_size_request(dock::DOCK_ICON_PX, 2);
    if let Some(cls) = state.dot_class() {
        dot.style_context().add_class(cls);
    }
    column.pack_start(&dot, false, false, 0);

    event_box.add(&column);

    let window_id = w.window_id.clone();
    let title = w.title.clone();
    // Resolve a .desktop id for the Pin action — only present when we
    // found a matching DesktopEntry; tasklist items spawned by apps
    // without a `.desktop` (random Qt tools, etc.) can't be pinned.
    let pin_target: Option<String> = entry.map(|e| e.id.clone());

    // Phase 5.7 — drag source. The user can drag a running tasklist
    // item onto the pinned strip to pin it. Only wired when we have a
    // resolvable .desktop id (matches the right-click "Pin to Dock"
    // menu's availability rule). The payload is the basename, e.g.
    // `firefox.desktop`; the strip's drop handler calls
    // `mackes_config::pin_app(cfg, desktop_id)`.
    dock_dnd::attach_tasklist_source(&event_box, pin_target.as_deref());

    event_box.connect_button_press_event(move |_, ev| match ev.button() {
        1 => {
            windows::toggle_window(&window_id);
            glib::Propagation::Stop
        }
        3 => {
            tasklist_context_menu(&window_id, &title, pin_target.as_deref());
            glib::Propagation::Stop
        }
        _ => glib::Propagation::Proceed,
    });

    event_box
}

/// Right-click context menu for a tasklist item. Three actions:
///   - Activate: bring window to front (handy when the icon is already
///     showing for the focused window — left-click would minimize it).
///   - Close: send EWMH `_NET_CLOSE_WINDOW`; the app handles shutdown.
///   - Maximize / Restore: toggle EWMH maximize hints. We don't track
///     maximize state from the panel, so we always offer both labels and
///     let the WM no-op the one that doesn't apply.
///   - Minimize: hide to taskbar.
fn tasklist_context_menu(window_id: &str, title: &str, pin_target: Option<&str>) {
    let menu = gtk::Menu::new();
    menu.set_widget_name("mackes-tasklist-menu");

    // Header: window title, disabled — purely informational.
    let header_label = title.chars().take(40).collect::<String>();
    let header = gtk::MenuItem::with_label(&header_label);
    header.set_sensitive(false);
    menu.append(&header);
    menu.append(&gtk::SeparatorMenuItem::new());

    let activate = gtk::MenuItem::with_label("Bring to Front");
    let wid = window_id.to_owned();
    activate.connect_activate(move |_| windows::activate_window(&wid));
    menu.append(&activate);

    let maximize = gtk::MenuItem::with_label("Maximize");
    let wid = window_id.to_owned();
    maximize.connect_activate(move |_| windows::maximize_window(&wid));
    menu.append(&maximize);

    let restore = gtk::MenuItem::with_label("Restore");
    let wid = window_id.to_owned();
    restore.connect_activate(move |_| windows::unmaximize_window(&wid));
    menu.append(&restore);

    let minimize = gtk::MenuItem::with_label("Minimize");
    let wid = window_id.to_owned();
    minimize.connect_activate(move |_| windows::minimize_window(&wid));
    menu.append(&minimize);

    // Pin to Dock — only available when we resolved a matching
    // DesktopEntry for this window. Pinning writes the .desktop id
    // to panel.toml; the dock's 2-s refresh tick picks it up.
    if let Some(target) = pin_target {
        menu.append(&gtk::SeparatorMenuItem::new());
        let pin = gtk::MenuItem::with_label("Pin to Dock");
        let target_owned = target.to_owned();
        pin.connect_activate(move |_| config_store::pin_app(&target_owned));
        menu.append(&pin);
    }

    menu.append(&gtk::SeparatorMenuItem::new());
    let close = gtk::MenuItem::with_label("Close");
    let wid = window_id.to_owned();
    close.connect_activate(move |_| windows::close_window(&wid));
    menu.append(&close);

    menu.show_all();
    menu.popup_easy(3, gtk::current_event_time());
}

/// Draws the wallpaper as a scaled-to-fit Image inside the desktop window.
/// If no wallpaper is found, falls back to the `PatternFly` dark surface
/// so the user never sees an unconfigured window background.
fn apply_wallpaper(window: &gtk::ApplicationWindow, geom: &FallbackGeometry, with_watermark: bool) {
    let path = resolve_wallpaper_path();
    let pixbuf = path
        .as_deref()
        .and_then(|p| Pixbuf::from_file_at_scale(p, geom.width, geom.height, false).ok());

    // 1.1.0 (Q19/Q20/Q21 + suggestions #2/#10): wrap the wallpaper in
    // a Gtk.Overlay so the Win10-style watermark can render in the
    // lower-right corner — but only on the primary monitor (#16
    // multi-monitor lock: secondary monitors get wallpaper without
    // the watermark, mirroring Win10's primary-only chrome).
    let overlay = gtk::Overlay::new();
    if let Some(pb) = pixbuf {
        let image = gtk::Image::from_pixbuf(Some(&pb));
        overlay.add(&image);
    }
    if with_watermark {
        overlay.add_overlay(&watermark::build());
    }
    window.add(&overlay);
}

/// Locate the active wallpaper. Looks in the running user's mackes-shell
/// state.json first; falls back to the standard wallpaper shipped under
/// `/usr/share/mackes-shell/branding/`.
fn resolve_wallpaper_path() -> Option<PathBuf> {
    if let Some(p) = wallpaper_from_state() {
        if Path::new(&p).is_file() {
            return Some(PathBuf::from(p));
        }
    }
    let fallback = PathBuf::from(DEFAULT_WALLPAPER);
    if fallback.is_file() {
        Some(fallback)
    } else {
        None
    }
}

fn wallpaper_from_state() -> Option<String> {
    let home = std::env::var_os("HOME")?;
    let state = PathBuf::from(home).join(".config/mackes-shell/state.json");
    let text = std::fs::read_to_string(&state).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.get("wallpaper")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

/// Rectangle covering the primary monitor in CSS pixels.
#[derive(Debug, Clone, Copy)]
struct FallbackGeometry {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl Default for FallbackGeometry {
    /// Last-resort defaults for headless/CI environments where no display
    /// is connected. 1920×1080 is the most common pixel-perfect target.
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }
    }
}

/// Primary monitor's geometry in CSS pixels. Returns `None` if there's no
/// connected display (CI / sandboxed builds) so callers fall back.
/// Superseded in 1.1.0 by `all_monitor_geometries` (#16); kept around
/// for the deprecated `build_top_bar` / `build_bottom_dock` paths.
#[allow(dead_code)]
fn primary_monitor_geometry() -> Option<FallbackGeometry> {
    let display = gdk::Display::default()?;
    let monitor = display.primary_monitor()?;
    let rect = monitor.geometry();
    Some(FallbackGeometry {
        x: rect.x(),
        y: rect.y(),
        width: rect.width(),
        height: rect.height(),
    })
}

/// 1.1.0 (#16) — every connected monitor's geometry in CSS pixels.
/// `gdk::Display::n_monitors` + `monitor(i)` enumerates the live
/// outputs. Returns an empty vec when no display is connected (CI /
/// sandboxed builds). The first entry is the primary monitor when
/// the display reports one — used by the taskbar + watermark.
fn all_monitor_geometries() -> Vec<FallbackGeometry> {
    let Some(display) = gdk::Display::default() else {
        return Vec::new();
    };
    let n = display.n_monitors();
    let primary = display.primary_monitor();
    let mut out: Vec<FallbackGeometry> = Vec::with_capacity(usize::try_from(n).unwrap_or(0));
    // Push the primary first so callers that index [0] get the
    // canonical surface.
    if let Some(m) = primary.as_ref() {
        let r = m.geometry();
        out.push(FallbackGeometry {
            x: r.x(),
            y: r.y(),
            width: r.width(),
            height: r.height(),
        });
    }
    for i in 0..n {
        if let Some(m) = display.monitor(i) {
            if primary
                .as_ref()
                .is_some_and(|pm| pm.model() == m.model() && pm.geometry() == m.geometry())
            {
                continue;
            }
            let r = m.geometry();
            out.push(FallbackGeometry {
                x: r.x(),
                y: r.y(),
                width: r.width(),
                height: r.height(),
            });
        }
    }
    out
}
