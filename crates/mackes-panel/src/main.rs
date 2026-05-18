//! mackes-panel — top status bar + bottom dock for Mackes XFCE Workstation.
//!
//! Phase 0.6: panel + desktop. Three windows on the primary monitor:
//!
//!   ┌──────────────────────────────────────────┐  top bar  (20 px Dock)
//!   │                                          │
//!   │    <desktop window — wallpaper image>    │  fullscreen Desktop hint,
//!   │                                          │  stacks below everything
//!   │                                          │
//!   ├──────────────────────────────────────────┤  bottom dock (80 px Dock)
//!   └──────────────────────────────────────────┘
//!
//! The Desktop-hint window replaces xfdesktop per Q39/Q40 — we own the
//! wallpaper render now. Phase 0.4–0.5 added the two Dock-hint strips;
//! Phase 0.6 adds the desktop layer beneath them.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use gdk::prelude::*;
use gdk_pixbuf::Pixbuf;
use gtk::prelude::*;

const TOP_BAR_HEIGHT_PX: i32 = 20;
const DOCK_HEIGHT_PX: i32 = 80;
const APP_ID: &str = "shell.mackes.Panel";

/// Each window we build gets the same PatternFly-dark surface (#151515)
/// per Q15. Inlined here so the very-first-boot stripe is visible without
/// loading external CSS files.
const PLACEHOLDER_CSS: &[u8] = b"window { background-color: #151515; }";

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
    let geom = primary_monitor_geometry().unwrap_or_default();
    build_desktop(app, &geom);
    build_top_bar(app, &geom);
    build_bottom_dock(app, &geom);
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
    window.set_default_size(geom.width, geom.height);
    window.move_(geom.x, geom.y);
    apply_wallpaper(&window, geom);
    window.show_all();
}

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
    window.set_default_size(geom.width, TOP_BAR_HEIGHT_PX);
    window.move_(geom.x, geom.y);
    apply_placeholder_style(&window);
    window.show_all();
}

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
    window.set_default_size(geom.width, DOCK_HEIGHT_PX);
    window.move_(geom.x, geom.y + geom.height - DOCK_HEIGHT_PX);
    apply_placeholder_style(&window);
    window.show_all();
}

fn apply_placeholder_style(window: &gtk::ApplicationWindow) {
    let style = window.style_context();
    let provider = gtk::CssProvider::new();
    provider
        .load_from_data(PLACEHOLDER_CSS)
        .expect("inline css must parse");
    style.add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
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
    } else {
        apply_placeholder_style(window);
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
