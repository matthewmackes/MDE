//! Start-menu popover — Win10-style app launcher.
//!
//! Anchored to the bottom-left corner of the primary output via
//! wlr-layer-shell-v1, above the panel's exclusive zone. The user
//! clicks the panel's `M` button → `mde-panel` execs
//! `mde-popover start-menu` → this binary opens a 480×560 layer-shell
//! window with a text-input search bar and a scrollable list of apps.
//! Clicking an app launches its Exec string and exits. Escape exits
//! without launching.

use std::path::{Path, PathBuf};
use std::process::Command;

use iced::widget::{button, column, container, scrollable, text, text_input, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;
use mde_applet_start_menu::{parse_desktop_file, search as search_apps, AppEntry};

/// Window dimensions. 480 px wide × 560 px tall matches the
/// Win10 start-menu proportions and fits comfortably above a
/// 40 px panel on every output we ship for (>= 768 px tall).
const WIDTH: u32 = 480;
const HEIGHT: u32 = 560;

/// Accent — same Carbon `interactive-04` / PatternFly blue-400 the
/// panel uses, kept in sync by visual inspection (a shared theme
/// crate lands at Phase E3.1 follow-up).
const ACCENT: Color = Color {
    r: 0.169,
    g: 0.604,
    b: 0.953,
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

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    SearchChanged(String),
    Launch(String),
    Exit,
}

pub struct App {
    all: Vec<AppEntry>,
    query: String,
}

impl iced_layershell::Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Task<Message>) {
        let all = load_all_entries();
        tracing::info!(count = all.len(), "loaded .desktop entries");
        (
            Self {
                all,
                query: String::new(),
            },
            text_input::focus("start-menu-search"),
        )
    }

    fn namespace(&self) -> String {
        "mde-popover-start-menu".to_string()
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::SearchChanged(q) => {
                self.query = q;
                Task::none()
            }
            Message::Launch(exec) => {
                launch_exec(&exec);
                std::process::exit(0);
            }
            Message::Exit => {
                std::process::exit(0);
            }
            _ => Task::none(),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        // Header — search box.
        let search = text_input("Search apps…", &self.query)
            .id("start-menu-search")
            .on_input(Message::SearchChanged)
            .padding(Padding {
                top: 10.0,
                right: 12.0,
                bottom: 10.0,
                left: 12.0,
            })
            .size(15)
            .style(search_input_style);

        // Filtered list.
        let q = self.query.trim();
        let entries: Vec<&AppEntry> = if q.is_empty() {
            self.all
                .iter()
                .filter(|e| !e.hidden && !e.name.is_empty() && !e.exec.is_empty())
                .collect()
        } else {
            search_apps(&self.all, q)
                .into_iter()
                .filter(|e| !e.exec.is_empty())
                .collect()
        };
        let mut sorted: Vec<&AppEntry> = entries;
        sorted.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        let mut list = column![].spacing(2);
        for entry in sorted.iter().take(200) {
            let label = column![
                text(entry.name.clone()).size(14).color(FG_TEXT),
                text(if entry.comment.is_empty() {
                    String::new()
                } else {
                    entry.comment.clone()
                })
                .size(11)
                .color(FG_MUTED),
            ]
            .spacing(2);
            let exec = entry.exec.clone();
            let row_btn = button(label)
                .width(Length::Fill)
                .padding(Padding {
                    top: 6.0,
                    right: 10.0,
                    bottom: 6.0,
                    left: 10.0,
                })
                .style(row_button_style)
                .on_press(Message::Launch(exec));
            list = list.push(row_btn);
        }

        let scroll = scrollable(list).height(Length::Fill);

        let header = container(text("Applications").size(11).color(FG_MUTED))
            .padding(Padding {
                top: 8.0,
                right: 12.0,
                bottom: 0.0,
                left: 12.0,
            });

        let footer = container(text("Press Esc to close").size(10).color(FG_MUTED))
            .padding(Padding {
                top: 4.0,
                right: 12.0,
                bottom: 8.0,
                left: 12.0,
            });

        let body = column![
            search,
            Space::with_height(Length::Fixed(4.0)),
            header,
            scroll,
            footer,
        ]
        .padding(Padding {
            top: 10.0,
            right: 8.0,
            bottom: 4.0,
            left: 8.0,
        });

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(popover_surface)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::keyboard::on_key_press(|key, _| {
            use iced::keyboard::{key::Named, Key};
            if matches!(key, Key::Named(Named::Escape)) {
                Some(Message::Exit)
            } else {
                None
            }
        })
    }
}

pub fn run() -> iced_layershell::Result {
    <App as iced_layershell::Application>::run(Settings {
        id: Some("mde-popover-start-menu".to_string()),
        fonts: crate::fonts::load_fallback_fonts(),
        layer_settings: LayerShellSettings {
            size: Some((WIDTH, HEIGHT)),
            // Don't reserve space — popovers float above content.
            exclusive_zone: 0,
            // Anchor bottom + left so the popover hugs the bottom-
            // left corner of the output (matches the panel's M
            // button position). Margin pushes it up above the panel.
            anchor: Anchor::Bottom | Anchor::Left,
            // Margin: 8 px above the panel (which is 40 px tall),
            // 4 px from the left edge.
            margin: (0, 0, 48, 4),
            // Overlay layer so the popover floats above any tiled
            // window and over the panel itself if rendering happens
            // to land there.
            layer: Layer::Overlay,
            // OnDemand keyboard so the search text-input can grab
            // focus + receive key events.
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        },
        ..Default::default()
    })
}

/// Walk every `applications/` directory on `$XDG_DATA_DIRS` (plus
/// the user's `~/.local/share/applications`) and parse every
/// `.desktop` file into an `AppEntry`. Duplicates from `lib.rs` of
/// `mde-applet-start-menu` because that lives in main.rs as a
/// private helper.
fn load_all_entries() -> Vec<AppEntry> {
    let mut out = Vec::new();
    for dir in application_dirs() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("desktop") {
                continue;
            }
            let Some(base) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_string)
            else {
                continue;
            };
            let Ok(raw) = std::fs::read_to_string(&path) else {
                continue;
            };
            out.push(parse_desktop_file(&base, &raw));
        }
    }
    out
}

fn application_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        out.push(Path::new(&home).join(".local/share/applications"));
    }
    let xdg = std::env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    for component in xdg.split(':') {
        if component.is_empty() {
            continue;
        }
        out.push(Path::new(component).join("applications"));
    }
    out
}

/// Launch a `.desktop` Exec string. Strips XDG field codes (%U, %F,
/// %i, %c, %k) per the spec and spawns the result detached. We use
/// `sh -c` because Exec strings may contain shell metacharacters
/// (quoted args, ~, env expansions).
fn launch_exec(exec: &str) {
    let stripped = strip_field_codes(exec);
    tracing::info!(exec = %stripped, "launching");
    let _ = Command::new("sh")
        .args(["-c", &stripped])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// Strip XDG `.desktop` Exec field codes — `%U`, `%F`, `%i`, `%c`,
/// `%k`, and any future single-letter code. Replaces each with a
/// space so argv-tokenizing shells don't merge surrounding words.
fn strip_field_codes(exec: &str) -> String {
    let mut out = String::with_capacity(exec.len());
    let mut chars = exec.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            if chars.next().is_some() {
                out.push(' ');
            }
        } else {
            out.push(c);
        }
    }
    out.trim().to_string()
}

fn popover_surface(_theme: &Theme) -> container::Style {
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
    }
}

fn row_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(Background::Color(Color {
            r: ACCENT.r,
            g: ACCENT.g,
            b: ACCENT.b,
            a: 0.14,
        })),
        button::Status::Pressed => Some(Background::Color(Color {
            r: ACCENT.r,
            g: ACCENT.g,
            b: ACCENT.b,
            a: 0.22,
        })),
        _ => None,
    };
    button::Style {
        background: bg,
        text_color: FG_TEXT,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
    }
}

fn search_input_style(_theme: &Theme, _status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: Background::Color(Color {
            r: 0.106,
            g: 0.106,
            b: 0.114,
            a: 1.0,
        }),
        border: Border {
            color: Color {
                r: 0.957,
                g: 0.957,
                b: 0.957,
                a: 0.08,
            },
            width: 1.0,
            radius: 6.0.into(),
        },
        icon: FG_MUTED,
        placeholder: FG_MUTED,
        value: FG_TEXT,
        selection: ACCENT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_field_codes_handles_known_codes() {
        assert_eq!(strip_field_codes("firefox %U"), "firefox");
        // %F is replaced with a single space; surrounding spaces are
        // preserved, so "gedit %F file.txt" → "gedit   file.txt"
        // (3 spaces internal). `sh -c` collapses runs of whitespace
        // when tokenizing argv, so the extra space is harmless.
        assert_eq!(strip_field_codes("gedit %F file.txt"), "gedit   file.txt");
        assert_eq!(strip_field_codes("plain"), "plain");
    }

    #[test]
    fn dimensions_pinned_for_visual_consistency() {
        assert_eq!(WIDTH, 480);
        assert_eq!(HEIGHT, 560);
    }
}
