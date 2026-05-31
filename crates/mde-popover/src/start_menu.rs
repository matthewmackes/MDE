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
use std::time::Instant;

use mde_theme::motion::list::{STAGGER_CAP, STAGGER_REVEAL_MS, STAGGER_STEP_MS};

use iced::widget::{
    button, column, container, mouse_area, row, scrollable, svg, text, text_input, Space,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Shadow, Subscription, Task, Theme};

use crate::watermark::{
    current_pending_count, spawn_pkexec_dnf_upgrade, WatermarkState,
};

/// v4.0.1 BUG-13 — pinned-tile glyph bytes (still served from the
/// `assets/icons/carbon/` directory until the Material Symbols
/// asset rename ships in EPIC-UI-MATERIAL.svg-swap). Baked here
/// rather than depending on `mde-panel`'s `panel_icons` module so
/// the popover crate stays free of upstream-binary deps.
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

/// ANIM-6.a — total entrance window: last-staggered-row delay + reveal.
/// Matches the ANIM-3.b.2 notifications pattern: cap at STAGGER_CAP rows,
/// 20 ms per-step, 120 ms reveal window → 260 ms total.
const MAX_ENTRANCE_MS: u64 =
    (STAGGER_CAP as u64 - 1) * STAGGER_STEP_MS as u64 + STAGGER_REVEAL_MS as u64;

/// ANIM-6.a — stagger alpha for an app-list row at the given elapsed ms.
/// Row indices beyond `STAGGER_CAP` are clamped so long lists don't crawl
/// (Q15 long-list policy). Uses sqrt easing (ease-out) so the reveal feels
/// snappy at the start and settles naturally.
fn stagger_alpha(row_index: usize, opened_ms: u64) -> f32 {
    let delay =
        row_index.min(STAGGER_CAP.saturating_sub(1)) as u64 * STAGGER_STEP_MS as u64;
    let elapsed = opened_ms.saturating_sub(delay);
    let t = (elapsed as f32 / STAGGER_REVEAL_MS as f32).clamp(0.0, 1.0);
    t.sqrt()
}

/// Accent — same Material blue 60 / PatternFly blue-400 the panel
/// uses, kept in sync by visual inspection (a shared theme crate
/// lands at Phase E3.1 follow-up).
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
    /// ANIM-6.a — drives entrance stagger frames at 16 ms intervals
    /// during the first MAX_ENTRANCE_MS ms after launch.
    AnimTick,
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
    /// ANIM-6.a — when the launcher was spawned; drives the entrance
    /// stagger. Read-only after initialization.
    opened_at: Instant,
}

fn namespace() -> String {
    "mde-popover-start-menu".to_string()
}

fn update(state: &mut App, msg: Message) -> Task<Message> {
    match msg {
        Message::SearchChanged(q) => {
            state.query = q;
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
        Message::AnimTick => Task::none(),
        _ => Task::none(),
    }
}

fn view(state: &App) -> Element<'_, Message> {
    // ANIM-6.a — elapsed ms since launch; drives per-row stagger.
    // After MAX_ENTRANCE_MS all rows are at alpha 1.0 and the tick
    // subscription has already stopped, so this read is free then.
    let opened_ms = state.opened_at.elapsed().as_millis() as u64;
    // row_idx is incremented for each visible app entry so long lists
    // stagger only the first STAGGER_CAP rows (Q15 long-list policy).
    let mut row_idx: usize = 0;

    // Header — search box.
    let search = text_input("Search apps…", &state.query)
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

    // Filtered list. `state.all` is already sorted at load time
    // (BUG-2 defensive perf fix), so view() is O(N) filter only.
    let q = state.query.trim();
    let entries: Vec<&AppEntry> = if q.is_empty() {
        state.all
            .iter()
            .filter(|e| !e.hidden && !e.name.is_empty() && !e.exec.is_empty())
            .collect()
    } else {
        search_apps(&state.all, q)
            .into_iter()
            .filter(|e| !e.exec.is_empty())
            .collect()
    };

    let mut list = column![].spacing(2);
    for entry in entries.iter().take(200) {
        // ANIM-6.a — stagger: each app entry fades in after a
        // per-row delay capped at STAGGER_CAP rows (Q15 policy).
        let alpha = stagger_alpha(row_idx, opened_ms);
        row_idx += 1;
        let label = column![
            text(entry.name.clone())
                .size(14)
                .color(Color { a: FG_TEXT.a * alpha, ..FG_TEXT }),
            text(if entry.comment.is_empty() {
                String::new()
            } else {
                entry.comment.clone()
            })
            .size(11)
            .color(Color { a: FG_MUTED.a * alpha, ..FG_MUTED }),
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
            .style(move |_theme: &Theme, status: button::Status| {
                let bg = match status {
                    button::Status::Hovered => Some(Background::Color(Color {
                        r: ACCENT.r,
                        g: ACCENT.g,
                        b: ACCENT.b,
                        a: 0.14 * alpha,
                    })),
                    button::Status::Pressed => Some(Background::Color(Color {
                        r: ACCENT.r,
                        g: ACCENT.g,
                        b: ACCENT.b,
                        a: 0.22 * alpha,
                    })),
                    _ => None,
                };
                button::Style {
                    background: bg,
                    text_color: Color { a: FG_TEXT.a * alpha, ..FG_TEXT },
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: 4.0.into(),
                    },
                    shadow: Shadow::default(),
                    snap: false,
                }
            })
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
    // v4.0.1 BUG-13: tiles now render the Material `folder` and
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
            Space::new().width(Length::Fixed(8.0)),
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
            Space::new().width(Length::Fill),
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
    let identity = text(state.system.identity_line()).size(10).color(FG_MUTED);
    let identity_row: Element<'_, Message> = if pending == 0 {
        row![identity, Space::new().width(Length::Fill)]
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
        row![identity, Space::new().width(Length::Fill), chip]
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
        Space::new().height(Length::Fixed(4.0)),
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
            container(Space::new())
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
            snap: false,
        })
        .into()
}

fn subscription(state: &App) -> Subscription<Message> {
    use iced::event;
    let keyboard = event::listen_with(|event, status, _window| {
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
    });
    // ANIM-6.a — drive entrance stagger frames during the first
    // MAX_ENTRANCE_MS ms. Self-disabling: once the window closes the
    // tick is dropped, avoiding pointless repaints at rest.
    let opened_ms = state.opened_at.elapsed().as_millis() as u64;
    if opened_ms <= MAX_ENTRANCE_MS {
        Subscription::batch([
            keyboard,
            iced::time::every(std::time::Duration::from_millis(16))
                .map(|_| Message::AnimTick),
        ])
    } else {
        keyboard
    }
}

pub fn run() -> iced_layershell::Result {
    iced_layershell::application(
        || {
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
                App {
                    all,
                    query: String::new(),
                    system,
                    opened_at: Instant::now(),
                },
                iced::widget::operation::focus("start-menu-search"),
            )
        },
        namespace,
        update,
        view,
    )
    .theme(|_: &App| iced::Theme::Dark)
    .subscription(subscription)
    .settings(Settings {
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
    .run()
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
        snap: false,
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
        snap: false,
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
        snap: false,
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

    // ── ANIM-6.a: stagger_alpha unit tests ────────────────────────────────────

    #[test]
    fn stagger_alpha_at_zero_ms_is_transparent() {
        // Row 0 has no delay but reveal hasn't started yet.
        assert!((stagger_alpha(0, 0) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn stagger_alpha_row0_fully_opaque_after_reveal_window() {
        // Row 0 delay = 0 ms; after STAGGER_REVEAL_MS elapsed → alpha = 1.
        assert!((stagger_alpha(0, STAGGER_REVEAL_MS as u64) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn stagger_alpha_row1_still_transparent_at_row0_reveal() {
        // Row 1's delay = STAGGER_STEP_MS; at exactly that point elapsed=0 → alpha=0.
        let ms = STAGGER_STEP_MS as u64;
        assert!((stagger_alpha(1, ms) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn stagger_alpha_caps_beyond_stagger_cap() {
        // Row STAGGER_CAP and row STAGGER_CAP+5 have the same delay.
        let ms = MAX_ENTRANCE_MS;
        let at_cap = stagger_alpha(STAGGER_CAP, ms);
        let beyond = stagger_alpha(STAGGER_CAP + 5, ms);
        assert!((at_cap - beyond).abs() < 1e-5);
    }

    #[test]
    fn max_entrance_ms_matches_token_arithmetic() {
        let expected =
            (STAGGER_CAP as u64 - 1) * STAGGER_STEP_MS as u64 + STAGGER_REVEAL_MS as u64;
        assert_eq!(MAX_ENTRANCE_MS, expected);
    }

    #[test]
    fn stagger_alpha_all_rows_opaque_at_max_entrance() {
        for idx in 0..STAGGER_CAP + 2 {
            let a = stagger_alpha(idx, MAX_ENTRANCE_MS);
            assert!(
                (a - 1.0).abs() < 1e-5,
                "row {idx} alpha={a} not 1.0 at MAX_ENTRANCE_MS"
            );
        }
    }
}
