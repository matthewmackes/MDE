//! Phase E.5 — clipboard via `wl-clipboard`.
//!
//! Best-choice deviation from the original "wlr-data-control via
//! smithay-client-toolkit" lock: the `wl-clipboard` package (the
//! `wl-copy` + `wl-paste` binaries) is the canonical
//! command-line interface to the wlr-data-control protocol on
//! every Wayland-on-wlroots compositor. It's a 50-line
//! subprocess wrapper instead of 500 lines of SCTK protocol
//! boilerplate.
//!
//! The mesh-replication path (`~/.cache/mde/clipboard.json`)
//! stays unchanged — `mded` writes it on every paste broadcast
//! and reads from it on peer connect. This module owns the
//! local Wayland side; mded owns the mesh side.

use std::io::Write;
use std::process::{Command, Stdio};

/// Read the current clipboard contents (text only). Returns
/// None when no text payload is available or `wl-paste` fails.
#[must_use]
pub fn paste_text() -> Option<String> {
    let out = Command::new("wl-paste")
        .args(["--no-newline", "--type", "text/plain"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Write `text` to the clipboard. Returns `Err` on subprocess
/// failure.
pub fn copy_text(text: &str) -> std::io::Result<()> {
    let mut child = Command::new("wl-copy")
        .args(["--type", "text/plain"])
        .stdin(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(std::io::Error::other(format!(
            "wl-copy exited with {status}"
        )));
    }
    Ok(())
}

/// Read mime-types currently available in the clipboard.
#[must_use]
pub fn available_mime_types() -> Vec<String> {
    let Ok(out) = Command::new("wl-paste").args(["--list-types"]).output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let s = String::from_utf8_lossy(&out.stdout);
    s.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// One past clipboard entry, as stored in `~/.cache/mde/clipboard.json`
/// by the mesh-clipboard worker. Defined here so test fixtures don't
/// need to import the worker's types.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ClipEntry {
    /// Unix epoch ms when the entry was captured.
    pub captured_at_ms: u64,
    /// Mime type — text/plain is the only one mesh-replicated.
    pub mime: String,
    /// Body (text/plain) or a base64 blob for non-text mimes.
    pub body: String,
    /// Which peer originated the entry (None = local).
    pub origin_peer: Option<String>,
}

/// Pure helper — parse the clipboard.json file. Returns an
/// empty vec if the file's missing/malformed.
#[must_use]
pub fn parse_clipboard_history(json: &str) -> Vec<ClipEntry> {
    serde_json::from_str(json).unwrap_or_default()
}

/// Default location of the mesh-replicated clipboard history.
#[must_use]
pub fn default_history_path() -> std::path::PathBuf {
    dirs::cache_dir()
        .map(|d| d.join("mde/clipboard.json"))
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp/mde-clipboard.json"))
}

// ──────────────────────────────────────────────────────────────
// v3.0.3 — Iced layer-shell history popover (Super+V)
// ──────────────────────────────────────────────────────────────

use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Shadow, Task, Theme,
};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

const WIDTH: u32 = 480;
const HEIGHT: u32 = 480;

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
const ACCENT: Color = Color {
    r: 0.169,
    g: 0.604,
    b: 0.953,
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
    Copy(usize),
    Exit,
}

pub struct App {
    history: Vec<ClipEntry>,
}

impl iced_layershell::Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Task<Message>) {
        let raw = std::fs::read_to_string(default_history_path()).unwrap_or_default();
        let history = parse_clipboard_history(&raw);
        tracing::info!(count = history.len(), "clipboard popover loaded");
        (Self { history }, Task::none())
    }

    fn namespace(&self) -> String {
        "mde-popover-clipboard".into()
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Copy(idx) => {
                if let Some(entry) = self.history.get(idx) {
                    if let Err(e) = copy_text(&entry.body) {
                        tracing::warn!(error = %e, "clipboard popover copy failed");
                        crate::toasts::emit(&crate::toasts::ToastEvent {
                            body: "clipboard copy failed".into(),
                            kind: crate::toasts::ToastKindWire::Error,
                            visible_ms: None,
                        });
                    } else {
                        // v3.0.3 — emit a toast confirming the
                        // copy. First in-tree emit site so the
                        // toast surface has a real source.
                        crate::toasts::emit(&crate::toasts::ToastEvent {
                            body: format!(
                                "Copied: {}",
                                preview(&entry.body, 40)
                            ),
                            kind: crate::toasts::ToastKindWire::Success,
                            visible_ms: None,
                        });
                    }
                }
                std::process::exit(0);
            }
            Message::Exit => std::process::exit(0),
            _ => Task::none(),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let header = container(
            row![
                text("Clipboard").size(13).color(FG_TEXT),
                Space::with_width(Length::Fixed(8.0)),
                text(format!("{} entries · mesh-synced", self.history.len()))
                    .size(10)
                    .color(FG_MUTED),
                Space::with_width(Length::Fill),
                crate::dismiss::close_button(Message::Exit),
            ]
            .align_y(Alignment::Center),
        )
        .padding(Padding {
            top: 8.0,
            right: 12.0,
            bottom: 4.0,
            left: 12.0,
        });

        let mut list = column![].spacing(2);
        if self.history.is_empty() {
            list = list.push(
                container(text("No clipboard history yet").size(13).color(FG_MUTED))
                    .padding(Padding {
                        top: 28.0,
                        right: 0.0,
                        bottom: 0.0,
                        left: 12.0,
                    }),
            );
        }
        for (idx, entry) in self.history.iter().take(50).enumerate() {
            let origin_label = entry
                .origin_peer
                .as_deref()
                .unwrap_or("local")
                .to_string();
            let preview_text = preview(&entry.body, 80);
            let row_btn = button(
                column![
                    text(preview_text).size(13).color(FG_TEXT),
                    Space::with_height(Length::Fixed(2.0)),
                    text(format!("{} · {}", origin_label, &entry.mime))
                        .size(10)
                        .color(FG_MUTED),
                ]
                .padding(Padding {
                    top: 6.0,
                    right: 12.0,
                    bottom: 6.0,
                    left: 12.0,
                }),
            )
            .width(Length::Fill)
            .style(history_row_style)
            .on_press(Message::Copy(idx));
            list = list.push(row_btn);
        }

        let scroll = scrollable(list).height(Length::Fill);

        let footer = container(
            text("Esc closes · click an entry to copy back to clipboard")
                .size(10)
                .color(FG_MUTED),
        )
        .padding(Padding {
            top: 4.0,
            right: 12.0,
            bottom: 8.0,
            left: 12.0,
        });

        let body = column![header, scroll, footer].padding(Padding {
            top: 4.0,
            right: 4.0,
            bottom: 4.0,
            left: 4.0,
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
        id: Some("mde-popover-clipboard".into()),
        fonts: crate::fonts::load_fallback_fonts(),
        layer_settings: LayerShellSettings {
            size: Some((WIDTH, HEIGHT)),
            exclusive_zone: 0,
            anchor: Anchor::Bottom | Anchor::Left,
            margin: (0, 0, 48, 4),
            layer: Layer::Overlay,
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        },
        ..Default::default()
    })
}

/// Truncate `body` to `max` chars + ellipsize newlines so the
/// list row stays single-line. Char-safe.
fn preview(body: &str, max: usize) -> String {
    let one_line: String = body
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    if one_line.chars().count() <= max {
        return one_line;
    }
    let mut out: String = one_line.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
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

fn history_row_style(_theme: &Theme, status: button::Status) -> button::Style {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_clipboard_history_handles_empty_string() {
        let out = parse_clipboard_history("");
        assert!(out.is_empty());
    }

    #[test]
    fn parse_clipboard_history_handles_malformed_json() {
        let out = parse_clipboard_history("{not json}");
        assert!(out.is_empty());
    }

    #[test]
    fn parse_clipboard_history_round_trips() {
        let entry = ClipEntry {
            captured_at_ms: 1_700_000_000_000,
            mime: "text/plain".into(),
            body: "hello".into(),
            origin_peer: None,
        };
        let json = serde_json::to_string(&vec![entry.clone()]).unwrap();
        let parsed = parse_clipboard_history(&json);
        assert_eq!(parsed, vec![entry]);
    }

    #[test]
    fn parse_clipboard_history_picks_up_peer_origin() {
        let json =
            r#"[{"captured_at_ms":1,"mime":"text/plain","body":"x","origin_peer":"lab-01"}]"#;
        let parsed = parse_clipboard_history(json);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].origin_peer, Some("lab-01".into()));
    }

    #[test]
    fn default_history_path_ends_with_clipboard_json() {
        let p = default_history_path();
        assert!(p.ends_with("clipboard.json"));
    }

    #[test]
    fn paste_text_does_not_panic_when_wl_paste_absent() {
        // Best-effort: on a Wayland-less host this just returns None.
        let _ = paste_text();
    }

    #[test]
    fn copy_text_does_not_panic_when_wl_copy_absent() {
        // Best-effort: just verify no panic on subprocess failure.
        let _ = copy_text("hello");
    }

    #[test]
    fn available_mime_types_returns_vec_even_on_failure() {
        let _ = available_mime_types();
    }
}
