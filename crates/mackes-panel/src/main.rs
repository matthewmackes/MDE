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

mod config_store;
mod dock;
mod icons;
mod mesh_sync;
mod top_bar;

use std::path::{Path, PathBuf};

use gdk::prelude::*;
use gdk_pixbuf::Pixbuf;
use gtk::prelude::*;

const TOP_BAR_HEIGHT_PX: i32 = 20;
const DOCK_HEIGHT_PX: i32 = 80;
const APP_ID: &str = "shell.mackes.Panel";

/// Backup chrome surface so the panel renders even when no token CSS is
/// installed (e.g. running the binary out of `target/release` against an
/// uninstalled tree). Real styling comes from `tokens.css` loaded below.
const PLACEHOLDER_CSS: &[u8] = b"
    window#mackes-top-bar,
    window#mackes-dock {
        background-color: #151515;
    }
    window#mackes-top-bar {
        border-bottom: 1px solid #292929;
    }
    window#mackes-dock {
        border-top: 1px solid #292929;
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

fn build_surfaces(app: &gtk::Application) {
    install_global_styling();
    // Load (or write-default-and-load) panel.toml. The result is unused
    // by the UI right now — Phase 2.5+ wires diffing into the layout.
    let _cfg = config_store::load_or_default();

    // Phase 2.3: hot-reload watcher. The returned monitor must outlive
    // the GTK main loop, so we leak it into a static. (Dropping the
    // monitor cancels the watch, which we don't want for the lifetime
    // of the panel process.)
    let monitor = config_store::watch(|new_cfg| match new_cfg {
        Some(_cfg) => eprintln!("mackes-panel: panel.toml reloaded"),
        None => eprintln!("mackes-panel: panel.toml went away or failed to parse"),
    });
    // Intentionally leak so the watch survives this function. Once
    // Phase 2.5 stores the active config in a long-lived struct,
    // ownership of the monitor moves there.
    std::mem::forget(monitor);

    let geom = primary_monitor_geometry().unwrap_or_default();
    build_desktop(app, &geom);
    build_top_bar(app, &geom);
    build_bottom_dock(app, &geom);
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

/// Fullscreen wallpaper layer that replaces xfdesktop.
fn build_desktop(app: &gtk::Application, geom: &FallbackGeometry) {
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
    apply_wallpaper(&window, geom);
    window.show_all();
}

/// Top status bar — 20 px Dock-hint window with three named layout slots.
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
    right.pack_start(&top_bar::status_cluster(), false, false, 0);

    bar.pack_start(&left, false, false, 0);
    bar.set_center_widget(Some(&center));
    bar.pack_end(&right, false, false, 0);

    window.add(&bar);
    window.show_all();
}

/// Bottom dock — 80 px Dock-hint window (primary monitor only).
fn build_bottom_dock(app: &gtk::Application, geom: &FallbackGeometry) {
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
    window.set_default_size(geom.width, DOCK_HEIGHT_PX);
    window.move_(geom.x, geom.y + geom.height - DOCK_HEIGHT_PX);

    // Single centered slot for the icon strip; Phase 5 populates it.
    let strip = build_slot("mackes-dock-strip");
    window.add(&strip);

    window.show_all();
}

fn build_slot(name: &str) -> gtk::Box {
    let slot = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    slot.set_widget_name(name);
    slot
}

/// Draws the wallpaper as a scaled-to-fit Image inside the desktop window.
/// If no wallpaper is found, falls back to the `PatternFly` dark surface
/// so the user never sees an unconfigured window background.
fn apply_wallpaper(window: &gtk::ApplicationWindow, geom: &FallbackGeometry) {
    let path = resolve_wallpaper_path();
    let pixbuf = path
        .as_deref()
        .and_then(|p| Pixbuf::from_file_at_scale(p, geom.width, geom.height, false).ok());

    if let Some(pb) = pixbuf {
        let image = gtk::Image::from_pixbuf(Some(&pb));
        window.add(&image);
    }
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
