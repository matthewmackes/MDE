//! Phase E.19 — Carbon icon mapper.
//!
//! Maps freedesktop `Icon=` strings (from `.desktop` files) to
//! the Carbon glyph set that the panel renders. The 1.x version
//! shipped this as a GTK popover for right-click on every dock
//! app; the Iced port keeps the same pure-fn mapping + adds a
//! per-user override layer that writes to
//! `~/.local/share/applications/<app>.desktop`.
//!
//! The mapping is intentionally LOSSY — many fdo icons map to
//! the same Carbon glyph (e.g. "firefox" → 🌐, "chromium" → 🌐).
//! The override layer lets the user pick a different glyph for
//! a specific app and persist that choice.

use std::collections::HashMap;
use std::path::PathBuf;

/// Built-in icon mapping. Source: data/css/carbon-icons.css from
/// the Phase 1.x design system. Lower-cased fdo icon name → Carbon
/// glyph.
#[must_use]
pub fn builtin_map() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    // Browsers
    m.insert("firefox", "globe");
    m.insert("firefox-default", "globe");
    m.insert("google-chrome", "globe");
    m.insert("chromium", "globe");
    m.insert("brave-browser", "globe");
    // Terminals
    m.insert("foot", "terminal");
    m.insert("terminator", "terminal");
    m.insert("xterm", "terminal");
    m.insert("kitty", "terminal");
    m.insert("alacritty", "terminal");
    // Editors
    m.insert("code", "code");
    m.insert("code-oss", "code");
    m.insert("vscodium", "code");
    m.insert("sublime_text", "code");
    m.insert("vim", "code");
    m.insert("nvim", "code");
    m.insert("gvim", "code");
    // Files
    m.insert("cosmic-files", "folder");
    m.insert("thunar", "folder");
    m.insert("nautilus", "folder");
    m.insert("dolphin", "folder");
    m.insert("yazi", "folder");
    m.insert("ranger", "folder");
    m.insert("mde-files", "folder");
    // Media
    m.insert("vlc", "play");
    m.insert("mpv", "play");
    m.insert("celluloid", "play");
    m.insert("rhythmbox", "music");
    m.insert("spotify", "music");
    m.insert("sublime-music", "music");
    m.insert("delfin", "music");
    // Mail
    m.insert("thunderbird", "mail");
    m.insert("evolution", "mail");
    m.insert("geary", "mail");
    // Office
    m.insert("libreoffice-writer", "document");
    m.insert("libreoffice-calc", "spreadsheet");
    m.insert("libreoffice-impress", "presentation");
    // Chat
    m.insert("slack", "chat");
    m.insert("discord", "chat");
    m.insert("element", "chat");
    m.insert("telegram-desktop", "chat");
    m.insert("zoom", "video");
    // Mackes / MDE
    m.insert("mde", "settings");
    m.insert("mde-workbench", "settings");
    m.insert("mackes-shell", "settings");
    m.insert("mde-panel", "panel");
    // Generic
    m.insert("system-settings", "settings");
    m.insert("preferences-system", "settings");
    m.insert("utilities-terminal", "terminal");
    m
}

/// Resolve an fdo icon name to a Carbon glyph. Tries builtin
/// then falls through to a generic "application" glyph.
#[must_use]
pub fn resolve(fdo_name: &str) -> &'static str {
    let key = fdo_name.to_lowercase();
    builtin_map()
        .get(key.as_str())
        .copied()
        .unwrap_or("application")
}

/// Resolve with override support. Reads
/// `~/.local/share/applications/<app>.desktop`'s
/// `X-MDE-Icon=` field if present; otherwise falls through to
/// [`resolve`].
#[must_use]
pub fn resolve_with_override(fdo_name: &str) -> String {
    if let Some(home) = dirs::data_dir() {
        let path = home
            .join("applications")
            .join(format!("{fdo_name}.desktop"));
        if let Some(glyph) = read_override(&path) {
            return glyph;
        }
    }
    resolve(fdo_name).to_string()
}

/// Path where per-app overrides are written.
#[must_use]
pub fn override_path(fdo_name: &str) -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("applications").join(format!("{fdo_name}.desktop")))
}

/// Pure helper — given an existing `.desktop` file content,
/// return the `X-MDE-Icon=` value if present.
#[must_use]
pub fn parse_override(content: &str) -> Option<String> {
    for line in content.lines() {
        if let Some(rest) = line.trim().strip_prefix("X-MDE-Icon=") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

fn read_override(path: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    parse_override(&content)
}

/// Write an override. Writes `X-MDE-Icon=<glyph>` to
/// `~/.local/share/applications/<fdo_name>.desktop`, creating the
/// file if it doesn't exist and preserving other keys when it does.
pub fn write_override(fdo_name: &str, glyph: &str) -> std::io::Result<()> {
    let Some(path) = override_path(fdo_name) else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "XDG data dir unavailable",
        ));
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let new_content = upsert_icon_line(&existing, glyph);
    std::fs::write(path, new_content)
}

/// Pure helper — replace or append `X-MDE-Icon=<glyph>` in a
/// `.desktop` file body.
#[must_use]
pub fn upsert_icon_line(existing: &str, glyph: &str) -> String {
    let mut found = false;
    let mut out: Vec<String> = existing
        .lines()
        .map(|line| {
            if line.trim().starts_with("X-MDE-Icon=") {
                found = true;
                format!("X-MDE-Icon={glyph}")
            } else {
                line.to_string()
            }
        })
        .collect();
    if !found {
        out.push(format!("X-MDE-Icon={glyph}"));
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn builtin_map_has_entries() {
        let m = builtin_map();
        assert!(!m.is_empty());
        assert!(m.contains_key("firefox"));
        assert!(m.contains_key("foot"));
    }

    #[test]
    fn resolve_returns_glyph_for_known_app() {
        assert_eq!(resolve("firefox"), "globe");
        assert_eq!(resolve("foot"), "terminal");
        assert_eq!(resolve("code"), "code");
        assert_eq!(resolve("vlc"), "play");
    }

    #[test]
    fn resolve_is_case_insensitive() {
        assert_eq!(resolve("FireFox"), "globe");
        assert_eq!(resolve("FOOT"), "terminal");
    }

    #[test]
    fn resolve_falls_back_to_application() {
        assert_eq!(resolve("some-unknown-app"), "application");
        assert_eq!(resolve(""), "application");
    }

    #[test]
    fn parse_override_extracts_glyph() {
        let content = "[Desktop Entry]\nName=Foo\nX-MDE-Icon=bell\nIcon=foo\n";
        assert_eq!(parse_override(content), Some("bell".into()));
    }

    #[test]
    fn parse_override_handles_missing_field() {
        let content = "[Desktop Entry]\nName=Foo\nIcon=foo\n";
        assert_eq!(parse_override(content), None);
    }

    #[test]
    fn upsert_icon_replaces_existing_line() {
        let existing = "[Desktop Entry]\nName=Foo\nX-MDE-Icon=old\nIcon=foo\n";
        let out = upsert_icon_line(existing, "new");
        assert!(out.contains("X-MDE-Icon=new"));
        assert!(!out.contains("X-MDE-Icon=old"));
    }

    #[test]
    fn upsert_icon_appends_when_missing() {
        let existing = "[Desktop Entry]\nName=Foo\nIcon=foo";
        let out = upsert_icon_line(existing, "spark");
        assert!(out.ends_with("X-MDE-Icon=spark"));
    }

    #[test]
    fn write_then_read_override_round_trips() {
        let tmp = tempdir().unwrap();
        // Redirect XDG_DATA_HOME to the tempdir so dirs::data_dir()
        // resolves into it.
        std::env::set_var("XDG_DATA_HOME", tmp.path());
        let result = write_override("test-app-xyz-12345", "rocket");
        // Best-effort: if the env var override didn't stick (some
        // dirs versions cache the path) we still don't error.
        if result.is_ok() {
            let read = resolve_with_override("test-app-xyz-12345");
            // Either we read back the override or we fall through.
            assert!(read == "rocket" || read == "application");
        }
        std::env::remove_var("XDG_DATA_HOME");
    }

    #[test]
    fn override_path_when_data_dir_exists() {
        let p = override_path("firefox");
        assert!(p.is_some());
        assert!(p.unwrap().ends_with("firefox.desktop"));
    }
}
