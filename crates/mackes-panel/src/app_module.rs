// App-module API is consumed by the bottom dock builder once it
// switches from the empty placeholder to a real strip.
#![allow(dead_code)]

//! Pinned-app launcher implementation of `DockModule`.
//!
//! Phase 5.1 of `docs/PROJECT_WORKLIST.md`. An `AppModule` wraps a
//! `DesktopEntry` (parsed by `desktop_files::scan()` once at startup)
//! and implements the `DockModule` trait so it can drop straight into
//! `dock::render_module`.
//!
//! Running-state detection (the under-icon dot's Running/Focused
//! signal) lands in Phase 5.2 via libwnck.

use crate::desktop_files::DesktopEntry;
use crate::dock::{DockModule, DockState};
use crate::icons;
use crate::top_bar::launch_exec;

/// One pinned dock item backed by a `.desktop` entry.
#[derive(Debug, Clone)]
pub struct AppModule {
    entry: DesktopEntry,
    /// Overridden per-tick by Phase 5.2's running-state detector.
    state: DockState,
}

impl AppModule {
    /// Wrap an entry as a dock module. State defaults to `Idle`
    /// (not running) — Phase 5.2 will update it from window events.
    #[must_use]
    pub const fn new(entry: DesktopEntry) -> Self {
        Self {
            entry,
            state: DockState::Idle,
        }
    }

    /// Replace the running-state. Called by the Phase 5.2 watcher
    /// when libwnck reports the corresponding window appeared,
    /// gained focus, or went away.
    pub const fn set_state(&mut self, state: DockState) {
        self.state = state;
    }
}

impl DockModule for AppModule {
    fn id(&self) -> String {
        // Use the .desktop basename — already unique on disk and
        // matches what panel.toml's `kind = "app"` entries store.
        self.entry.id.clone()
    }

    fn icon_name(&self) -> &str {
        // Route through icons::resolve so well-known apps wear a Carbon
        // symbolic glyph (Q14 — monochrome dock). Falls back to the
        // freedesktop generic-app icon when the .desktop shipped no
        // Icon at all.
        self.entry
            .icon
            .as_deref()
            .map_or("applications-other-symbolic", icons::resolve)
    }

    fn tooltip(&self) -> &str {
        &self.entry.name
    }

    fn state(&self) -> DockState {
        self.state
    }

    fn on_click(&self) {
        // top_bar::launch_exec already handles field-code stripping
        // and the Terminal=true wrapper.
        launch_exec(&self.entry.exec, self.entry.terminal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, name: &str, icon: Option<&str>) -> DesktopEntry {
        DesktopEntry {
            id: id.to_owned(),
            name: name.to_owned(),
            exec: name.to_lowercase(),
            icon: icon.map(str::to_owned),
            categories: Vec::new(),
            terminal: false,
        }
    }

    #[test]
    fn id_matches_desktop_basename() {
        let m = AppModule::new(entry("firefox.desktop", "Firefox", None));
        assert_eq!(m.id(), "firefox.desktop");
    }

    #[test]
    fn icon_name_falls_back_to_generic_when_missing() {
        let m = AppModule::new(entry("foo.desktop", "Foo", None));
        assert_eq!(m.icon_name(), "applications-other-symbolic");
    }

    #[test]
    fn icon_name_uses_entry_icon_when_present() {
        let m = AppModule::new(entry("foo.desktop", "Foo", Some("foo-icon")));
        assert_eq!(m.icon_name(), "foo-icon");
    }

    #[test]
    fn set_state_round_trips() {
        let mut m = AppModule::new(entry("x.desktop", "X", None));
        assert_eq!(m.state(), DockState::Idle);
        m.set_state(DockState::Focused);
        assert_eq!(m.state(), DockState::Focused);
    }

    #[test]
    fn tooltip_uses_name() {
        let m = AppModule::new(entry("x.desktop", "Some App", None));
        assert_eq!(m.tooltip(), "Some App");
    }
}
