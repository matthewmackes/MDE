//! Portal-37 theme pump — propagates MDE-Dark + Intel One Mono to
//! GTK3 / GTK4 / Qt6 apps on every login.
//!
//! mde-session runs this once before exec'ing the compositor, so by
//! the time autostarted GTK / Qt apps come up, their settings files
//! already point at the MDE-Dark theme and Intel One Mono font.
//!
//! The pump is idempotent — it rewrites the settings files every
//! login because user-side theme switchers (lookbook tool, qt6ct
//! GUI) may have stomped them between sessions. Operators who want
//! to opt out can drop `~/.config/mde/theme-pump.disabled` (any
//! contents; presence is the signal).

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;

const MDE_GTK_THEME: &str = "MDE-Dark";
const MDE_FONT: &str = "Intel One Mono 10";
const MDE_ICON_THEME: &str = "Mackes-Carbon";
const QT6CT_COLOR_NAME: &str = "MDE-Dark";

/// Apply the MDE theme + font settings to every config file the
/// pump owns. Returns the list of paths that were rewritten; an
/// empty vec means everything was already current (idempotent).
///
/// Failures on individual files log a warning but never propagate —
/// the session-start path must never block on theme-pump errors.
pub fn apply() -> Vec<PathBuf> {
    let Some(cfg) = dirs::config_dir() else {
        tracing::warn!("theme-pump: no XDG config dir — skipping");
        return Vec::new();
    };
    if cfg.join("mde").join("theme-pump.disabled").exists() {
        tracing::info!("theme-pump: disabled by user opt-out marker");
        return Vec::new();
    }

    let mut changed = Vec::new();
    for (path, body) in build_targets(&cfg) {
        match write_if_changed(&path, &body) {
            Ok(true) => {
                tracing::info!(path = %path.display(), "theme-pump: rewrote");
                changed.push(path);
            }
            Ok(false) => {}
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "theme-pump: write failed");
            }
        }
    }
    changed
}

fn build_targets(cfg: &std::path::Path) -> Vec<(PathBuf, String)> {
    vec![
        (cfg.join("gtk-3.0").join("settings.ini"), gtk_settings_ini()),
        (cfg.join("gtk-4.0").join("settings.ini"), gtk_settings_ini()),
        (cfg.join("qt6ct").join("qt6ct.conf"), qt6ct_conf()),
    ]
}

fn gtk_settings_ini() -> String {
    format!(
        "[Settings]\n\
         gtk-theme-name={MDE_GTK_THEME}\n\
         gtk-icon-theme-name={MDE_ICON_THEME}\n\
         gtk-font-name={MDE_FONT}\n\
         gtk-application-prefer-dark-theme=1\n\
         gtk-cursor-theme-name=Adwaita\n\
         gtk-decoration-layout=:close\n\
         gtk-enable-animations=1\n",
    )
}

fn qt6ct_conf() -> String {
    format!(
        "[Appearance]\n\
         color_scheme_path=/usr/share/qt6ct/colors/{QT6CT_COLOR_NAME}.conf\n\
         custom_palette=true\n\
         icon_theme={MDE_ICON_THEME}\n\
         standard_dialogs=default\n\
         style=Fusion\n\
         \n\
         [Fonts]\n\
         general=\"{MDE_FONT},,-1,5,400,0,0,0,0,0,0,0,0,0,0,1,Intel One Mono\"\n\
         fixed=\"{MDE_FONT},,-1,5,400,0,0,0,0,0,0,0,0,0,0,1,Intel One Mono\"\n\
         \n\
         [Interface]\n\
         activate_item_on_single_click=1\n\
         menus_have_icons=true\n\
         show_shortcuts_in_context_menus=true\n",
    )
}

fn write_if_changed(path: &std::path::Path, body: &str) -> std::io::Result<bool> {
    if let Ok(existing) = fs::read_to_string(path) {
        if existing == body {
            return Ok(false);
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::File::create(path)?;
    f.write_all(body.as_bytes())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn gtk_settings_carries_mde_theme_and_font() {
        let body = gtk_settings_ini();
        assert!(body.contains("gtk-theme-name=MDE-Dark"));
        assert!(body.contains("gtk-font-name=Intel One Mono 10"));
        assert!(body.contains("gtk-application-prefer-dark-theme=1"));
    }

    #[test]
    fn qt6ct_conf_points_at_mde_color_scheme() {
        let body = qt6ct_conf();
        assert!(body.contains("color_scheme_path=/usr/share/qt6ct/colors/MDE-Dark.conf"));
        assert!(body.contains("style=Fusion"));
        assert!(body.contains("Intel One Mono"));
    }

    #[test]
    fn write_if_changed_writes_on_first_call() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("subdir").join("foo.ini");
        let wrote = write_if_changed(&path, "hello").unwrap();
        assert!(wrote);
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn write_if_changed_skips_when_identical() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("foo.ini");
        fs::write(&path, "hello").unwrap();
        let wrote = write_if_changed(&path, "hello").unwrap();
        assert!(!wrote);
    }

    #[test]
    fn write_if_changed_rewrites_when_different() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("foo.ini");
        fs::write(&path, "old").unwrap();
        let wrote = write_if_changed(&path, "new").unwrap();
        assert!(wrote);
        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
    }

    #[test]
    fn build_targets_includes_three_files() {
        let cfg = std::path::Path::new("/tmp/fake-cfg");
        let targets = build_targets(cfg);
        assert_eq!(targets.len(), 3);
        let paths: Vec<_> = targets.iter().map(|(p, _)| p.to_string_lossy().to_string()).collect();
        assert!(paths.iter().any(|p| p.ends_with("gtk-3.0/settings.ini")));
        assert!(paths.iter().any(|p| p.ends_with("gtk-4.0/settings.ini")));
        assert!(paths.iter().any(|p| p.ends_with("qt6ct/qt6ct.conf")));
    }
}
