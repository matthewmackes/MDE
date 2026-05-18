// API surface is consumed by Phase 1.5+; suppress dead-code warnings
// while the module ships ahead of its callers.
#![allow(dead_code)]

//! Mackes-Carbon icon loader.
//!
//! Given a freedesktop icon name (e.g. `"folder-symbolic"`, `"home"`,
//! `"system-search-symbolic"`) returns a `gdk_pixbuf::Pixbuf` from the
//! installed Mackes-Carbon theme at the requested pixel size. Caches the
//! parsed pixbuf per (`name`, `size`) so subsequent loads of the same
//! glyph are free.
//!
//! Per Q14 of the design lock, every panel/dock icon resolves through
//! this loader — no app-specific PNGs, no third-party theme.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

use gdk_pixbuf::Pixbuf;

/// Root of the installed Mackes-Carbon icon theme.
const THEME_ROOT: &str = "/usr/share/icons/Mackes-Carbon/scalable";

/// Freedesktop categories the build script lays icons out under. Search
/// order matches the spec's typical lookup order so the most semantically
/// correct hit wins.
const CATEGORIES: &[&str] = &[
    "actions",
    "status",
    "devices",
    "places",
    "emblems",
    "categories",
    "mimetypes",
    "apps",
];

thread_local! {
    /// Per-thread (`name`, `size`) → Pixbuf cache. The panel runs single-
    /// threaded (GTK main loop) so a thread-local is sufficient and avoids
    /// a Mutex on the hot path.
    static CACHE: RefCell<HashMap<(String, i32), Pixbuf>> = RefCell::new(HashMap::new());
}

/// Curated map of app `.desktop` Icon= values → Mackes-Carbon symbolic
/// names per Q14 (monochrome Carbon for every dock item). When a dock
/// `AppModule` asks for its icon, we route through `resolve()` first so
/// well-known apps wear the Carbon look even if their `.desktop` shipped
/// a brand-colored Icon=.
const APP_TO_CARBON: &[(&str, &str)] = &[
    // Browsers
    ("firefox", "earth"),
    ("firefox-nightly", "earth"),
    ("google-chrome", "earth"),
    ("chromium", "earth"),
    ("brave-browser", "earth"),
    // Mail / chat
    ("thunderbird", "email"),
    ("evolution", "email"),
    ("element-desktop", "chat"),
    ("slack", "chat"),
    ("discord", "chat"),
    // Files / utilities
    ("thunar", "folder--open"),
    ("nautilus", "folder--open"),
    ("file-roller", "archive"),
    ("gnome-calculator", "calculator"),
    ("gnome-system-monitor", "analytics"),
    ("htop", "analytics"),
    // Terminals / dev
    ("xfce4-terminal", "terminal"),
    ("gnome-terminal", "terminal"),
    ("alacritty", "terminal"),
    ("kitty", "terminal"),
    ("code", "code"),
    ("code-oss", "code"),
    ("idea", "code"),
    ("clion", "code"),
    // Media
    ("vlc", "play--filled-alt"),
    ("mpv", "play--filled-alt"),
    ("sublime-music", "music"),
    ("rhythmbox", "music"),
    ("delfin", "video"),
    // Office
    ("libreoffice-writer", "document"),
    ("libreoffice-calc", "table"),
    ("libreoffice-impress", "presentation-file"),
    ("libreoffice-startcenter", "document"),
    // Graphics
    ("gimp", "image"),
    ("krita", "image"),
    ("inkscape", "image"),
    ("blender", "cube"),
    // Mackes (our own)
    ("mackes-shell", "settings--adjust"),
    ("mackes-clipboard", "paste"),
];

/// Look up a `.desktop` Icon= value in the app→Carbon table. Returns
/// the Carbon symbolic name on hit, or the original (so `load()` can
/// still try the freedesktop hierarchy) on miss.
#[must_use]
pub fn resolve(desktop_icon: &str) -> &str {
    let key = desktop_icon
        .rsplit('/')
        .next()
        .and_then(|n| n.split('.').next())
        .unwrap_or(desktop_icon);
    for (k, v) in APP_TO_CARBON {
        if k.eq_ignore_ascii_case(key) {
            return v;
        }
    }
    desktop_icon
}

/// Look up an icon by freedesktop name and return a `Pixbuf` sized to
/// `size_px`. Returns `None` only if the file is genuinely missing or
/// fails to parse — never panics.
#[must_use]
pub fn load(name: &str, size_px: i32) -> Option<Pixbuf> {
    let key = (name.to_owned(), size_px);
    if let Some(hit) = CACHE.with(|c| c.borrow().get(&key).cloned()) {
        return Some(hit);
    }

    let path = locate_svg(name)?;
    let pb = Pixbuf::from_file_at_scale(&path, size_px, size_px, true).ok()?;
    CACHE.with(|c| {
        c.borrow_mut().insert(key, pb.clone());
    });
    Some(pb)
}

/// Resolve `name` to an SVG path under `THEME_ROOT`. Tries the literal
/// name first, then the `-symbolic` variant, then the bare-name (stripped
/// suffix) variant. Walks the category list in `CATEGORIES` order.
fn locate_svg(name: &str) -> Option<PathBuf> {
    let candidates = candidate_basenames(name);
    for category in CATEGORIES {
        for cand in &candidates {
            let p = PathBuf::from(THEME_ROOT).join(category).join(cand);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

fn candidate_basenames(name: &str) -> Vec<String> {
    let mut v = Vec::with_capacity(3);
    v.push(format!("{name}.svg"));
    if name.ends_with("-symbolic") {
        // foo-symbolic → also try foo.svg as a fallback
        let bare = name.trim_end_matches("-symbolic");
        v.push(format!("{bare}.svg"));
    } else {
        // foo → also try foo-symbolic.svg in case the theme only carries
        // the symbolic variant for this name.
        v.push(format!("{name}-symbolic.svg"));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_basenames_for_plain_name() {
        let v = candidate_basenames("folder");
        assert_eq!(v, ["folder.svg", "folder-symbolic.svg"]);
    }

    #[test]
    fn candidate_basenames_for_symbolic_suffix() {
        let v = candidate_basenames("folder-symbolic");
        assert_eq!(v, ["folder-symbolic.svg", "folder.svg"]);
    }

    #[test]
    fn locate_returns_none_for_missing() {
        // This will only "miss correctly" when the theme isn't installed
        // in the test environment. Either way the function shouldn't
        // panic — that's the assertion.
        let _ = locate_svg("definitely-not-a-real-icon-name");
    }

    #[test]
    fn resolve_maps_known_app() {
        assert_eq!(resolve("firefox"), "earth");
        assert_eq!(resolve("thunar"), "folder--open");
        assert_eq!(resolve("FIREFOX"), "earth"); // case-insensitive
    }

    #[test]
    fn resolve_strips_path_and_extension() {
        assert_eq!(resolve("/usr/share/icons/firefox.png"), "earth");
        assert_eq!(resolve("firefox.svg"), "earth");
    }

    #[test]
    fn resolve_falls_through_on_miss() {
        assert_eq!(
            resolve("definitely-not-mapped-symbolic"),
            "definitely-not-mapped-symbolic"
        );
    }
}
