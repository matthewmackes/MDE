//! Wallpaper layer-shell applet — paints the desktop
//! background.
//!
//! Phase E1.2.12: today this applet shells out to
//! `swaybg -i <path>` against the wallpaper key from the
//! settings store. The proper layer-shell-via-Iced path
//! lands when Phase E.2 (Iced layer-shell anchor) ships;
//! until then `swaybg` is the lowest-risk way to surface
//! the user's wallpaper choice without a custom Wayland
//! client.

#![forbid(unsafe_code)]

use std::path::PathBuf;

use mde_applet_api::{AppletId, AppletSlot, HostMessage};

/// Build the static applet manifest the host registers at
/// startup. Slot = Overlay because the wallpaper painter renders
/// on the wlr-layer-shell `background` layer rather than embedded
/// in a top-bar slot.
#[must_use]
pub fn manifest() -> mde_applet_api::AppletManifest {
    mde_applet_api::AppletManifest {
        id: AppletId::from_static("bg"),
        binary: "mde-applet-bg".into(),
        slot: AppletSlot::Overlay,
        summary: "Wallpaper layer-shell painter (swaybg today; Iced layer-shell at Phase E.2)"
            .into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

/// MDE wallpaper settings key — the workbench wallpaper
/// panel writes the chosen image path here.
pub const WALLPAPER_KEY: &str = "wallpaper.path";

/// Resolve the wallpaper file path the applet should
/// display. Priority:
///   1. `$MDE_WALLPAPER_PATH` env var (test override).
///   2. The `wallpaper.path` value from
///      `~/.cache/mde/wallpaper.json` (the sidecar the
///      MDE Settings store writes).
///   3. A built-in fallback at
///      `/usr/share/mde/branding/standard-wallpaper.png`.
#[must_use]
pub fn resolve_wallpaper_path() -> PathBuf {
    if let Ok(p) = std::env::var("MDE_WALLPAPER_PATH") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    if let Some(p) = read_sidecar_path() {
        return p;
    }
    PathBuf::from("/usr/share/mde/branding/standard-wallpaper.png")
}

fn read_sidecar_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let sidecar = PathBuf::from(home).join(".cache/mde/wallpaper.json");
    let raw = std::fs::read_to_string(sidecar).ok()?;
    let parsed = parse_wallpaper_sidecar(&raw)?;
    Some(PathBuf::from(parsed))
}

/// Pure parser for the wallpaper sidecar JSON shape:
/// `{"path": "/path/to/image.png"}`. Returns the path
/// string on success.
#[must_use]
pub fn parse_wallpaper_sidecar(raw: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    let obj = v.as_object()?;
    let path = obj.get("path")?.as_str()?;
    if path.is_empty() {
        return None;
    }
    Some(path.to_string())
}

/// Build the swaybg argv. Today the applet always uses
/// `--mode fill` so portrait photos crop sensibly on
/// landscape outputs. The wallpaper-panel's mode picker
/// (CB-1.6 follow-on) will write a `wallpaper.mode` key
/// that the applet reads + threads into here.
#[must_use]
pub fn build_swaybg_argv(path: &std::path::Path) -> Vec<String> {
    vec![
        "swaybg".into(),
        "--image".into(),
        path.to_string_lossy().into_owned(),
        "--mode".into(),
        "fill".into(),
    ]
}

/// Process a host control message and return `true` when the
/// applet should keep running. Only [`HostMessage::Shutdown`]
/// stops the event loop; every other variant is a host-side
/// hint the renderer reacts to elsewhere.
#[must_use]
pub fn handle_host(msg: &HostMessage) -> bool {
    !matches!(msg, HostMessage::Shutdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_lands_in_overlay_slot() {
        let m = manifest();
        assert_eq!(m.id.as_str(), "bg");
        assert_eq!(m.slot, AppletSlot::Overlay);
    }

    #[test]
    fn wallpaper_key_lock() {
        assert_eq!(WALLPAPER_KEY, "wallpaper.path");
    }

    #[test]
    fn parse_wallpaper_sidecar_extracts_path() {
        let raw = r#"{"path": "/home/u/Pictures/wp.jpg"}"#;
        assert_eq!(
            parse_wallpaper_sidecar(raw),
            Some("/home/u/Pictures/wp.jpg".to_string())
        );
    }

    #[test]
    fn parse_wallpaper_sidecar_rejects_empty_path() {
        let raw = r#"{"path": ""}"#;
        assert!(parse_wallpaper_sidecar(raw).is_none());
    }

    #[test]
    fn parse_wallpaper_sidecar_rejects_non_object() {
        assert!(parse_wallpaper_sidecar("[]").is_none());
        assert!(parse_wallpaper_sidecar("garbage").is_none());
    }

    #[test]
    fn build_swaybg_argv_includes_mode_fill() {
        let argv = build_swaybg_argv(std::path::Path::new("/tmp/wp.png"));
        assert_eq!(argv[0], "swaybg");
        assert!(argv.iter().any(|a| a == "--image"));
        assert!(argv.iter().any(|a| a == "/tmp/wp.png"));
        assert!(argv.iter().any(|a| a == "--mode"));
        assert!(argv.iter().any(|a| a == "fill"));
    }

    #[test]
    fn resolve_wallpaper_path_uses_env_override() {
        let key = "MDE_WALLPAPER_PATH";
        std::env::set_var(key, "/tmp/test-wp.png");
        let path = resolve_wallpaper_path();
        std::env::remove_var(key);
        assert_eq!(path, PathBuf::from("/tmp/test-wp.png"));
    }

    #[test]
    fn handle_host_short_circuits_shutdown() {
        assert!(!handle_host(&HostMessage::Shutdown));
    }
}
