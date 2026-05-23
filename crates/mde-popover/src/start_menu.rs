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

use iced::widget::{
    button, column, container, mouse_area, row, scrollable, svg, text, text_input, Space,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Shadow, Task, Theme};

use crate::watermark::{
    current_pending_count, spawn_pkexec_dnf_upgrade, WatermarkState,
};

/// v4.0.1 BUG-13 — Carbon glyph bytes for the pinned tiles. Baked
/// here rather than depending on `mde-panel`'s `panel_icons` module
/// so the popover crate stays free of upstream-binary deps.
const FILES_SVG: &[u8] =
    include_bytes!("../../../assets/icons/carbon/files.svg");
const WORKBENCH_SVG: &[u8] =
    include_bytes!("../../../assets/icons/carbon/workbench.svg");

/// v4.0.1 — accent for the "N updates pending" chip in the footer
/// system-identity strip. Matches the operator-locked indigo (Q2).
const UPDATE_CHIP_ACCENT: Color = Color {
    r: 0.357,
    g: 0.416,
    b: 0.961,
    a: 1.0,
};
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
    /// v4.0.1 — operator clicked the "N updates pending" chip in
    /// the system-identity footer. Routes to `spawn_pkexec_dnf_upgrade`
    /// (was the watermark popover's click handler before the v4.0.1
    /// watermark-into-start-menu move).
    DnfUpgradeClicked,
}

pub struct App {
    all: Vec<AppEntry>,
    query: String,
    /// v4.0.1 — system-identity snapshot loaded once at popover
    /// spawn. The pending-update count is re-read from the cache
    /// file on every view() so a `dnf upgrade` that completes
    /// during the popover's lifetime reflects on the next render
    /// (cheap — a 3-byte file read).
    system: WatermarkState,
}

impl iced_layershell::Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Task<Message>) {
        // v4.0.1 BUG-2 defensive perf fix: pre-sort `all` once
        // here, in lowercase-by-name order. The previous render
        // path called `Vec::sort_by` on every redraw — on systems
        // with hundreds of .desktop entries (a stock Fedora
        // workstation has ~250) the per-frame N log N cost
        // accumulated under scroll wheel input bursts and the
        // operator reported the popover locking. Sorting once at
        // load + dropping the per-render `sort_by` keeps the
        // view function O(N) (filter only).
        let mut all = load_all_entries();
        all.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        tracing::info!(count = all.len(), "loaded .desktop entries");
        let system = WatermarkState::load();
        (
            Self {
                all,
                query: String::new(),
                system,
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
            Message::DnfUpgradeClicked => {
                tracing::info!("start-menu footer click → pkexec dnf upgrade");
                spawn_pkexec_dnf_upgrade();
                Task::none()
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

        // Filtered list. `self.all` is already sorted at load time
        // (BUG-2 defensive perf fix), so view() is O(N) filter only.
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

        let mut list = column![].spacing(2);
        for entry in entries.iter().take(200) {
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

        // v4.0.1 BUG-12 — pinned tiles row. Static (non-scrolling)
        // shortcuts for the file manager (mde-files) and the
        // workbench (mde-workbench), shown above the scrollable
        // .desktop apps list. Win10 start-menu pattern: the
        // operator's most-used surfaces are one click away without
        // needing to type or scroll.
        // v4.0.1 BUG-13: tiles now render the Carbon `folder` and
        // `tools` glyphs above the label rather than label-only.
        let pinned_tile = |svg_bytes: &'static [u8], label: &'static str, exec: &'static str| {
            let glyph = svg(svg::Handle::from_memory(svg_bytes))
                .width(Length::Fixed(28.0))
                .height(Length::Fixed(28.0))
                .style(|_theme: &Theme, _status: svg::Status| svg::Style {
                    color: Some(FG_TEXT),
                });
            button(
                column![
                    glyph,
                    text(label).size(13).color(FG_TEXT),
                ]
                .align_x(Alignment::Center)
                .spacing(4),
            )
            .padding(Padding {
                top: 12.0,
                right: 12.0,
                bottom: 12.0,
                left: 12.0,
            })
            .width(Length::FillPortion(1))
            .style(row_button_style)
            .on_press(Message::Launch(exec.into()))
        };
        let pinned_row = container(
            row![
                pinned_tile(FILES_SVG, "Files", "mde-files"),
                Space::with_width(Length::Fixed(8.0)),
                pinned_tile(WORKBENCH_SVG, "Workbench", "mde-workbench"),
            ]
            .align_y(Alignment::Center),
        )
        .padding(Padding {
            top: 4.0,
            right: 12.0,
            bottom: 8.0,
            left: 12.0,
        });

        // v3.0.3 — header row with the section label on the left
        // and a visible close button on the right so the popover
        // can always be dismissed by mouse (Esc still works via
        // the subscription handler below). Bug fix: previously
        // the only close path was Esc, and with OnDemand keyboard
        // interactivity Esc didn't always reach the surface.
        let header = container(
            row![
                text("Applications").size(11).color(FG_MUTED),
                Space::with_width(Length::Fill),
                crate::dismiss::close_button(Message::Exit),
            ]
            .align_y(Alignment::Center),
        )
        .padding(Padding {
            top: 8.0,
            right: 12.0,
            bottom: 0.0,
            left: 12.0,
        });

        // v4.0.1 — Win10-style system-identity strip. Replaces the
        // retired watermark popover. Left side shows the always-on
        // "MDE X.Y.Z · Fedora N · host" line (per
        // `WatermarkState::identity_line`); right side shows a
        // clickable "N updates pending" chip when the cached dnf
        // count is > 0 (chip fires `Message::DnfUpgradeClicked` →
        // `spawn_pkexec_dnf_upgrade`, same action the old watermark
        // had). Count is re-read from the cache on every view so an
        // upgrade that completed during the popover lifetime
        // reflects immediately.
        let pending = current_pending_count();
        let identity = text(self.system.identity_line()).size(10).color(FG_MUTED);
        let identity_row: Element<'_, Message> = if pending == 0 {
            row![identity, Space::with_width(Length::Fill)]
                .align_y(Alignment::Center)
                .into()
        } else {
            let chip = button(
                text(format!("{pending} updates pending"))
                    .size(10)
                    .color(Color::WHITE),
            )
            .padding(Padding {
                top: 3.0,
                right: 8.0,
                bottom: 3.0,
                left: 8.0,
            })
            .style(update_chip_style)
            .on_press(Message::DnfUpgradeClicked);
            row![identity, Space::with_width(Length::Fill), chip]
                .align_y(Alignment::Center)
                .spacing(8)
                .into()
        };
        let identity_strip = container(identity_row).padding(Padding {
            top: 6.0,
            right: 12.0,
            bottom: 0.0,
            left: 12.0,
        });

        let footer = container(
            text("Esc closes · click outside the M to re-toggle").size(10).color(FG_MUTED),
        )
        .padding(Padding {
            top: 4.0,
            right: 12.0,
            bottom: 8.0,
            left: 12.0,
        });

        let body = column![
            search,
            Space::with_height(Length::Fixed(4.0)),
            pinned_row,
            header,
            scroll,
            identity_strip,
            footer,
        ]
        .padding(Padding {
            top: 10.0,
            right: 8.0,
            bottom: 4.0,
            left: 8.0,
        });

        let card: iced::Element<'_, Message> = container(body)
            .width(Length::Fixed(WIDTH as f32))
            .height(Length::Fixed(HEIGHT as f32))
            .style(popover_surface)
            .into();

        // v3.0.4 (2026-05-23) — backdrop surround. Card pinned
        // bottom-left (4 px from left edge, 48 px above the
        // panel), every other pixel = mouse_area firing Exit
        // on click.
        let dismiss = || {
            mouse_area(
                container(Space::with_width(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .on_press(Message::Exit)
        };
        let bottom_strip = row![
            container(card).padding(iced::Padding {
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
                background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
                border: iced::Border {
                    color: iced::Color::TRANSPARENT,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                shadow: iced::Shadow::default(),
                text_color: None,
            })
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
            // v3.0.4 (2026-05-23) — fullscreen surface so the
            // outer mouse_area covering the rest of the screen
            // catches click-outside-to-dismiss events. The
            // visible card stays at WIDTH×HEIGHT pinned
            // bottom-left by the view's column+row layout.
            size: None,
            exclusive_zone: -1,
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            margin: (0, 0, 0, 0),
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

/// v4.0.1 — "N updates pending" chip in the start-menu footer.
/// Indigo Q2 accent with a subtle darken on press; full opacity at
/// rest so the count reads as an actionable affordance.
fn update_chip_style(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Background::Color(Color {
            r: UPDATE_CHIP_ACCENT.r * 1.08,
            g: UPDATE_CHIP_ACCENT.g * 1.08,
            b: UPDATE_CHIP_ACCENT.b * 1.08,
            a: 1.0,
        }),
        button::Status::Pressed => Background::Color(Color {
            r: UPDATE_CHIP_ACCENT.r * 0.85,
            g: UPDATE_CHIP_ACCENT.g * 0.85,
            b: UPDATE_CHIP_ACCENT.b * 0.85,
            a: 1.0,
        }),
        _ => Background::Color(UPDATE_CHIP_ACCENT),
    };
    button::Style {
        background: Some(bg),
        text_color: Color::WHITE,
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
