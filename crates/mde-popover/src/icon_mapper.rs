//! v3.0.3 Phase E.19 wiring — icon-mapper popover.
//!
//! Surfaces a grid of candidate Material Symbols glyph names for the
//! target app's freedesktop Icon= (or the app's sway `app_id`
//! when the .desktop doesn't supply one). Clicking a candidate
//! writes `~/.local/share/applications/<app>.desktop`'s
//! `X-MDE-Icon=` line via `mde_panel::icon_mapper::
//! write_override`; the panel's next render pass picks the
//! new glyph up.
//!
//! Spawn contract (env-var based — same shape as the WM-3
//! window-actions popover):
//!
//!   MDE_ICON_MAPPER_APP_ID   the `app_id` (sway) or the
//!                            .desktop basename (panel) the
//!                            override should be written for.
//!                            Empty value → the popover
//!                            renders but every button is a
//!                            no-op (defensive).
//!
//! Anchor: bottom-left (sits above the dock corner where the
//! invoking cell renders). The fullscreen backdrop dismiss
//! pattern from v3.0.4 keeps the click-outside semantics.

use std::process::Command;

use iced::widget::{button, column, container, mouse_area, row, scrollable, text, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Subscription, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

const WIDTH: u32 = 320;
const HEIGHT: u32 = 360;

const ACCENT: Color = Color {
    r: 0.357,
    g: 0.416,
    b: 0.961,
    a: 1.0,
};
const FG_TEXT: Color = Color {
    r: 0.957,
    g: 0.957,
    b: 0.957,
    a: 1.0,
};
const FG_MUTED: Color = Color {
    r: 0.659,
    g: 0.659,
    b: 0.659,
    a: 1.0,
};
const SURFACE_BG: Color = Color {
    r: 0.055,
    g: 0.055,
    b: 0.063,
    a: 0.97,
};

/// Curated short list of Material Symbols glyph names the picker
/// surfaces. Mirrors the `builtin_map()` codomain in
/// `mde_panel::icon_mapper` — when the operator hits a glyph
/// here, it's one of the names the resolver will produce on
/// next launch. Adding a new name here costs nothing; the
/// override write is glyph-name-agnostic.
const CANDIDATE_GLYPHS: &[&str] = &[
    "application",
    "globe",
    "terminal",
    "code",
    "folder",
    "play",
    "music",
    "mail",
    "document",
    "spreadsheet",
    "presentation",
    "chat",
    "video",
    "settings",
    "panel",
];

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    /// User clicked a glyph — write the override + exit.
    PickGlyph(String),
    /// Esc / backdrop click — exit without writing.
    Exit,
}

pub struct App {
    app_id: String,
    current: String,
    last_error: Option<String>,
}

fn namespace() -> String {
    "mde-popover-icon-mapper".to_string()
}

fn update(state: &mut App, msg: Message) -> Task<Message> {
    match msg {
        Message::PickGlyph(glyph) => {
            if state.app_id.is_empty() {
                state.last_error = Some("no app_id supplied".to_string());
                return Task::none();
            }
            match write_override_for(&state.app_id, &glyph) {
                Ok(()) => std::process::exit(0),
                Err(e) => {
                    state.last_error = Some(e);
                    Task::none()
                }
            }
        }
        Message::Exit => std::process::exit(0),
        _ => Task::none(),
    }
}

fn view(state: &App) -> Element<'_, Message> {
    let header_label = if state.app_id.is_empty() {
        "Customize Icon".to_string()
    } else {
        format!("Customize {} icon", state.app_id)
    };
    let header = row![
        text(header_label).size(13).color(FG_TEXT),
        Space::new().width(Length::Fill),
        crate::dismiss::close_button(Message::Exit),
    ]
    .align_y(iced::Alignment::Center);

    let current_line = text(format!("Currently: {}", state.current))
        .size(10)
        .color(FG_MUTED);

    let mut grid = column![].spacing(6);
    let mut current_row = row![].spacing(6).align_y(iced::Alignment::Center);
    let mut col_count = 0usize;
    for glyph in CANDIDATE_GLYPHS {
        let is_current = *glyph == state.current;
        current_row = current_row.push(glyph_button(glyph, is_current));
        col_count += 1;
        if col_count == 3 {
            grid = grid.push(current_row);
            current_row = row![].spacing(6).align_y(iced::Alignment::Center);
            col_count = 0;
        }
    }
    if col_count > 0 {
        grid = grid.push(current_row);
    }

    let mut body_col = column![
        header,
        Space::new().height(Length::Fixed(4.0)),
        current_line,
        Space::new().height(Length::Fixed(12.0)),
        scrollable(grid).height(Length::Fill),
    ];
    if let Some(err) = &state.last_error {
        body_col = body_col.push(Space::new().height(Length::Fixed(6.0)));
        body_col = body_col.push(text(err.clone()).size(10).color(Color {
            r: 0.95,
            g: 0.40,
            b: 0.40,
            a: 1.0,
        }));
    }
    body_col = body_col.push(Space::new().height(Length::Fixed(6.0)));
    body_col = body_col.push(
        text("Esc closes · click outside dismisses")
            .size(9)
            .color(FG_MUTED),
    );

    let card: Element<'_, Message> = container(
        body_col.padding(Padding {
            top: 14.0,
            right: 14.0,
            bottom: 10.0,
            left: 14.0,
        }),
    )
    .width(Length::Fixed(WIDTH as f32))
    .height(Length::Fixed(HEIGHT as f32))
    .style(popover_surface)
    .into();

    let dismiss = || {
        mouse_area(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(Message::Exit)
    };
    let bottom_strip = row![
        container(card).padding(Padding {
            top: 0.0,
            right: 0.0,
            bottom: 48.0,
            left: 4.0,
        }),
        dismiss(),
    ]
    .height(Length::Fixed((HEIGHT + 48) as f32));
    container(column![dismiss(), bottom_strip])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 0.0.into(),
            },
            shadow: Shadow::default(),
            text_color: None,
            snap: false,
        })
        .into()
}

fn subscription(_state: &App) -> Subscription<Message> {
    use iced::event;
    event::listen_with(|event, status, _window| {
        use iced::keyboard;
        match event {
            iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. })
                if status == event::Status::Ignored =>
            {
                use iced::keyboard::{key::Named, Key};
                if matches!(key, Key::Named(Named::Escape)) {
                    Some(Message::Exit)
                } else {
                    None
                }
            }
            _ => None,
        }
    })
}

pub fn run() -> iced_layershell::Result {
    iced_layershell::application(
        || {
            let app_id = std::env::var("MDE_ICON_MAPPER_APP_ID").unwrap_or_default();
            let current = resolve_current(&app_id);
            tracing::info!(app_id = %app_id, current = %current, "icon-mapper popover open");
            App {
                app_id,
                current,
                last_error: None,
            }
        },
        namespace,
        update,
        view,
    )
    .theme(|_: &App| Theme::Dark)
    .subscription(subscription)
    .settings(Settings {
        id: Some("mde-popover-icon-mapper".to_string()),
        fonts: crate::fonts::load_fallback_fonts(),
        layer_settings: LayerShellSettings {
            size: None,
            exclusive_zone: -1,
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            margin: (0, 0, 0, 0),
            layer: Layer::Overlay,
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        },
        ..Default::default()
    })
    .run()
}

fn glyph_button(glyph: &str, current: bool) -> Element<'static, Message> {
    let label = text(glyph.to_string()).size(11).color(FG_TEXT);
    let g = glyph.to_string();
    button(label)
        .padding(Padding {
            top: 10.0,
            right: 10.0,
            bottom: 10.0,
            left: 10.0,
        })
        .width(Length::Fixed(88.0))
        .on_press(Message::PickGlyph(g))
        .style(move |_t: &Theme, status: iced::widget::button::Status| {
            let alpha = match status {
                iced::widget::button::Status::Hovered => 0.32,
                iced::widget::button::Status::Pressed => 0.50,
                _ if current => 0.22,
                _ => 0.06,
            };
            iced::widget::button::Style {
                background: Some(Background::Color(Color {
                    r: ACCENT.r,
                    g: ACCENT.g,
                    b: ACCENT.b,
                    a: alpha,
                })),
                text_color: FG_TEXT,
                border: Border {
                    color: Color {
                        r: ACCENT.r,
                        g: ACCENT.g,
                        b: ACCENT.b,
                        a: if current { 0.85 } else { 0.30 },
                    },
                    width: if current { 1.5 } else { 1.0 },
                    radius: 5.0.into(),
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .into()
}

fn popover_surface(_t: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE_BG)),
        border: Border {
            color: Color {
                r: 0.957,
                g: 0.957,
                b: 0.957,
                a: 0.10,
            },
            width: 1.0,
            radius: 8.0.into(),
        },
        text_color: Some(FG_TEXT),
        shadow: Shadow::default(),
        snap: false,
    }
}

/// Best-effort current-glyph readout. Spawns `mde-panel
/// --resolve-icon <app_id>` and falls back to the static
/// builtin table when the subprocess isn't installed (dev
/// builds) or fails. The function is a pure read — never
/// writes anywhere.
fn resolve_current(app_id: &str) -> String {
    if app_id.is_empty() {
        return "application".to_string();
    }
    // Try the panel binary first.
    if let Ok(out) = Command::new("mde-panel")
        .args(["--resolve-icon", app_id])
        .output()
    {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return s;
            }
        }
    }
    // Fall back to a tiny inline table mirroring the
    // mde_panel::icon_mapper::builtin_map entries that this
    // popover's CANDIDATE_GLYPHS shows. Keeps the popover
    // useful even when mde-panel isn't on PATH.
    inline_fallback_resolve(&app_id.to_lowercase())
}

/// Pure helper — used both as the live fallback and as a
/// test fixture so we don't shell out in tests.
#[must_use]
pub fn inline_fallback_resolve(app_id_lc: &str) -> String {
    let g = match app_id_lc {
        "firefox" | "google-chrome" | "chromium" | "brave-browser" => "globe",
        "foot" | "terminator" | "kitty" | "alacritty" | "xterm" => "terminal",
        "code" | "code-oss" | "vscodium" | "vim" | "nvim" | "gvim" => "code",
        "thunar" | "nautilus" | "dolphin" | "yazi" | "ranger" | "mde-files"
        | "cosmic-files" => "folder",
        "vlc" | "mpv" | "celluloid" => "play",
        "rhythmbox" | "spotify" | "sublime-music" | "delfin" => "music",
        "thunderbird" | "evolution" | "geary" => "mail",
        "libreoffice-writer" => "document",
        "libreoffice-calc" => "spreadsheet",
        "libreoffice-impress" => "presentation",
        "slack" | "discord" | "element" | "telegram-desktop" => "chat",
        "zoom" => "video",
        "mde" | "mde-workbench" | "mackes-shell" | "system-settings"
        | "preferences-system" => "settings",
        "mde-panel" => "panel",
        _ => "application",
    };
    g.to_string()
}

/// Write the X-MDE-Icon= override. Returns an Err(String)
/// instead of propagating the panic so the popover can
/// surface the failure in its red status row rather than
/// crashing.
fn write_override_for(app_id: &str, glyph: &str) -> Result<(), String> {
    if app_id.is_empty() {
        return Err("no app_id".to_string());
    }
    if glyph.is_empty() {
        return Err("no glyph".to_string());
    }
    let path = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| "HOME not set".to_string())?
        .join(".local/share/applications")
        .join(format!("{app_id}.desktop"));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let updated = upsert_icon_line(&existing, glyph);
    std::fs::write(&path, updated).map_err(|e| format!("write: {e}"))?;
    tracing::info!(app = %app_id, glyph = %glyph, path = %path.display(), "icon override written");
    Ok(())
}

/// Insert or replace `X-MDE-Icon=<glyph>` in an existing
/// `.desktop` file content. If a `[Desktop Entry]` header is
/// absent, prepends one. Mirrors `mde_panel::icon_mapper::
/// upsert_icon_line` so test assertions stay shape-stable
/// across the two consumers.
#[must_use]
pub fn upsert_icon_line(existing: &str, glyph: &str) -> String {
    let replacement = format!("X-MDE-Icon={glyph}");
    let mut lines: Vec<String> = existing.lines().map(String::from).collect();
    let mut replaced = false;
    for line in lines.iter_mut() {
        if line.trim().starts_with("X-MDE-Icon=") {
            *line = replacement.clone();
            replaced = true;
            break;
        }
    }
    if !replaced {
        if lines.iter().any(|l| l.trim() == "[Desktop Entry]") {
            // Append after the header but before the next
            // section.
            let mut out = Vec::with_capacity(lines.len() + 1);
            let mut inserted = false;
            for line in lines {
                out.push(line.clone());
                if !inserted && line.trim() == "[Desktop Entry]" {
                    out.push(replacement.clone());
                    inserted = true;
                }
            }
            lines = out;
        } else {
            // No header — fresh file. Build a minimal valid
            // .desktop.
            lines = vec!["[Desktop Entry]".to_string(), replacement];
        }
    }
    let mut joined = lines.join("\n");
    if !joined.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_glyphs_distinct() {
        use std::collections::HashSet;
        let set: HashSet<&&str> = CANDIDATE_GLYPHS.iter().collect();
        assert_eq!(set.len(), CANDIDATE_GLYPHS.len());
    }

    #[test]
    fn inline_fallback_resolves_known_apps() {
        assert_eq!(inline_fallback_resolve("firefox"), "globe");
        assert_eq!(inline_fallback_resolve("foot"), "terminal");
        assert_eq!(inline_fallback_resolve("code"), "code");
        assert_eq!(inline_fallback_resolve("mde-files"), "folder");
    }

    #[test]
    fn inline_fallback_resolves_unknown_to_application() {
        assert_eq!(inline_fallback_resolve("never-seen-app"), "application");
        assert_eq!(inline_fallback_resolve(""), "application");
    }

    #[test]
    fn upsert_icon_line_appends_when_missing() {
        let raw = "[Desktop Entry]\nName=Firefox\nExec=firefox\n";
        let out = upsert_icon_line(raw, "globe");
        assert!(out.contains("X-MDE-Icon=globe"));
        assert!(out.contains("Name=Firefox"));
    }

    #[test]
    fn upsert_icon_line_replaces_existing() {
        let raw = "[Desktop Entry]\nX-MDE-Icon=old\nName=Firefox\n";
        let out = upsert_icon_line(raw, "globe");
        assert!(out.contains("X-MDE-Icon=globe"));
        assert!(!out.contains("X-MDE-Icon=old"));
    }

    #[test]
    fn upsert_icon_line_handles_empty_input() {
        let out = upsert_icon_line("", "globe");
        assert!(out.contains("[Desktop Entry]"));
        assert!(out.contains("X-MDE-Icon=globe"));
    }
}
