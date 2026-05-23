//! Phase E.20 — bottom-edge transient toast popups.
//!
//! Toasts are short-lived overlay messages — "copied!", "saved",
//! "linked to peer lab-01". They appear above the bottom-bar
//! panel surface for a fixed duration, then fade.
//!
//! 2026 design language:
//! - Centered on the bottom edge, 24px above the panel.
//! - Pill shape: 12px corner radius, 8px vertical / 16px horizontal
//!   padding, hairline border in `mackes_accent` at 22% alpha.
//! - 2 second visible duration, 220ms fade-in + 320ms fade-out.
//! - Stack vertically when multiple toasts queue, newest on top.
//! - Drop the longest-visible toast if the stack exceeds 3.

use std::time::{Duration, Instant};

/// Default visible duration (excluding fade in/out).
pub const DEFAULT_VISIBLE_MS: u64 = 2000;
/// Stack ceiling — drop the oldest when this is hit.
pub const STACK_LIMIT: usize = 3;

/// Severity styles the renderer can pick up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastKind {
    #[default]
    Info,
    Success,
    Warn,
    Error,
}

/// One toast in the stack.
#[derive(Debug, Clone)]
pub struct Toast {
    pub kind: ToastKind,
    pub body: String,
    pub created_at: Instant,
    pub visible_for: Duration,
}

impl Toast {
    /// Create with the default duration.
    #[must_use]
    pub fn info<S: Into<String>>(body: S) -> Self {
        Self::with(
            ToastKind::Info,
            body,
            Duration::from_millis(DEFAULT_VISIBLE_MS),
        )
    }

    #[must_use]
    pub fn success<S: Into<String>>(body: S) -> Self {
        Self::with(
            ToastKind::Success,
            body,
            Duration::from_millis(DEFAULT_VISIBLE_MS),
        )
    }

    #[must_use]
    pub fn warn<S: Into<String>>(body: S) -> Self {
        Self::with(
            ToastKind::Warn,
            body,
            Duration::from_millis(DEFAULT_VISIBLE_MS),
        )
    }

    #[must_use]
    pub fn error<S: Into<String>>(body: S) -> Self {
        Self::with(
            ToastKind::Error,
            body,
            Duration::from_millis(DEFAULT_VISIBLE_MS),
        )
    }

    fn with<S: Into<String>>(kind: ToastKind, body: S, visible_for: Duration) -> Self {
        Self {
            kind,
            body: body.into(),
            created_at: Instant::now(),
            visible_for,
        }
    }

    /// True once the toast's visible window has elapsed.
    #[must_use]
    pub fn is_expired_at(&self, now: Instant) -> bool {
        now.duration_since(self.created_at) >= self.visible_for
    }
}

/// The toast stack — bounded queue with FIFO eviction.
#[derive(Debug, Clone, Default)]
pub struct ToastStack {
    inner: Vec<Toast>,
}

impl ToastStack {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new toast onto the stack. Evicts the oldest if the
    /// stack would exceed `STACK_LIMIT`.
    pub fn push(&mut self, toast: Toast) {
        self.inner.push(toast);
        if self.inner.len() > STACK_LIMIT {
            self.inner.remove(0);
        }
    }

    /// Remove expired toasts. Caller invokes this on each tick.
    pub fn retain_unexpired(&mut self, now: Instant) {
        self.inner.retain(|t| !t.is_expired_at(now));
    }

    /// Current visible count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// True when no toasts are visible.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Iterate from oldest (bottom of stack) to newest (top).
    pub fn iter(&self) -> impl Iterator<Item = &Toast> {
        self.inner.iter()
    }
}

// ──────────────────────────────────────────────────────────────
// v3.0.3 — long-running toast render surface
// ──────────────────────────────────────────────────────────────

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use iced::widget::{column, container, text};
use iced::{
    Background, Border, Color, Element, Length, Padding, Shadow, Task, Theme,
};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;
use serde::{Deserialize, Serialize};

/// Where toast emit-events get appended. One JSON line per event.
/// The render surface tails this file every `POLL_INTERVAL_MS` and
/// pushes new entries into its `ToastStack`. Emit sites just append
/// to the file — they don't need to know whether the surface is
/// running (no-op when it isn't).
#[must_use]
pub fn toast_queue_path() -> PathBuf {
    dirs::cache_dir()
        .map(|d| d.join("mde/toasts.jsonl"))
        .unwrap_or_else(|| PathBuf::from("/tmp/mde-toasts.jsonl"))
}

const POLL_INTERVAL_MS: u64 = 200;

/// Wire shape for one toast event. `body` is required; everything
/// else defaults if absent so a minimal `{"body":"…"}` write
/// works.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToastEvent {
    pub body: String,
    #[serde(default)]
    pub kind: ToastKindWire,
    /// Visible duration in ms. Defaults to DEFAULT_VISIBLE_MS.
    #[serde(default)]
    pub visible_ms: Option<u64>,
}

/// Serde-friendly mirror of `ToastKind`. We define a separate type
/// so the public ToastKind doesn't have to derive Serde (which
/// would lock down its rename behavior in unrelated callers).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ToastKindWire {
    #[default]
    Info,
    Success,
    Warn,
    Error,
}

impl From<ToastKindWire> for ToastKind {
    fn from(w: ToastKindWire) -> Self {
        match w {
            ToastKindWire::Info => ToastKind::Info,
            ToastKindWire::Success => ToastKind::Success,
            ToastKindWire::Warn => ToastKind::Warn,
            ToastKindWire::Error => ToastKind::Error,
        }
    }
}

/// Append one toast event to the queue file. Used by every emit
/// site in the panel + popover crates. Best-effort: errors are
/// swallowed with a debug log because a missing queue file just
/// means the toast surface isn't running (toasts get silently
/// dropped, which is the right failure mode for transient UI).
pub fn emit(event: &ToastEvent) {
    let path = toast_queue_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let line = match serde_json::to_string(event) {
        Ok(s) => format!("{s}\n"),
        Err(e) => {
            tracing::debug!(error = %e, "toast emit: serialize failed");
            return;
        }
    };
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = f.write_all(line.as_bytes());
    }
}

const FG_TEXT: Color = Color {
    r: 0.957,
    g: 0.957,
    b: 0.957,
    a: 1.0,
};
const SURFACE_BG: Color = Color {
    r: 0.055,
    g: 0.055,
    b: 0.063,
    a: 0.95,
};

fn pill_accent(kind: ToastKind) -> Color {
    match kind {
        ToastKind::Info => Color {
            r: 0.169,
            g: 0.604,
            b: 0.953,
            a: 1.0,
        },
        ToastKind::Success => Color {
            r: 0.224,
            g: 0.741,
            b: 0.388,
            a: 1.0,
        },
        ToastKind::Warn => Color {
            r: 0.949,
            g: 0.694,
            b: 0.247,
            a: 1.0,
        },
        ToastKind::Error => Color {
            r: 0.98,
            g: 0.31,
            b: 0.34,
            a: 1.0,
        },
    }
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Tick,
}

pub struct App {
    stack: Arc<Mutex<ToastStack>>,
    /// Byte offset of the last successful queue-file read so we
    /// only parse new lines on subsequent polls.
    last_offset: Arc<Mutex<u64>>,
}

impl iced_layershell::Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Task<Message>) {
        let app = Self {
            stack: Arc::new(Mutex::new(ToastStack::new())),
            last_offset: Arc::new(Mutex::new(0)),
        };
        // Seed: skip any pre-existing entries (we only show toasts
        // that fire after the surface started). Set offset to the
        // current file size.
        if let Ok(meta) = std::fs::metadata(toast_queue_path()) {
            if let Ok(mut o) = app.last_offset.lock() {
                *o = meta.len();
            }
        }
        (app, Task::none())
    }

    fn namespace(&self) -> String {
        "mde-popover-toasts".into()
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Tick => {
                self.poll_queue();
                if let Ok(mut s) = self.stack.lock() {
                    s.retain_unexpired(std::time::Instant::now());
                }
            }
            _ => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let snapshot: Vec<Toast> = match self.stack.lock() {
            Ok(g) => g.iter().cloned().collect(),
            Err(_) => Vec::new(),
        };
        if snapshot.is_empty() {
            return container(text(""))
                .width(Length::Fixed(1.0))
                .height(Length::Fixed(1.0))
                .into();
        }
        let mut col = column![].spacing(8);
        // Iterate oldest-to-newest; render newest at the top.
        for toast in snapshot.iter().rev() {
            col = col.push(toast_pill(toast));
        }
        container(col)
            .padding(Padding {
                top: 0.0,
                right: 0.0,
                bottom: 8.0,
                left: 0.0,
            })
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(POLL_INTERVAL_MS))
            .map(|_| Message::Tick)
    }
}

impl App {
    /// Read any new bytes from the queue file since the last poll.
    /// Parses each new line as a `ToastEvent` and pushes onto the
    /// shared `ToastStack` (which evicts at STACK_LIMIT=3).
    fn poll_queue(&mut self) {
        use std::io::{Read, Seek, SeekFrom};
        let path = toast_queue_path();
        let mut file = match std::fs::File::open(&path) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut offset = match self.last_offset.lock() {
            Ok(g) => *g,
            Err(_) => return,
        };
        if file.seek(SeekFrom::Start(offset)).is_err() {
            return;
        }
        let mut buf = String::new();
        if file.read_to_string(&mut buf).is_err() {
            return;
        }
        // Advance offset by however many bytes we read.
        offset += buf.len() as u64;
        if let Ok(mut o) = self.last_offset.lock() {
            *o = offset;
        }
        for line in buf.lines() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<ToastEvent>(line) {
                Ok(event) => {
                    let visible = event
                        .visible_ms
                        .map(std::time::Duration::from_millis)
                        .unwrap_or_else(|| {
                            std::time::Duration::from_millis(DEFAULT_VISIBLE_MS)
                        });
                    let toast = Toast::with(event.kind.into(), event.body, visible);
                    if let Ok(mut s) = self.stack.lock() {
                        s.push(toast);
                    }
                }
                Err(e) => {
                    tracing::debug!(error = %e, line, "toast queue: parse failed");
                }
            }
        }
    }
}

pub fn run() -> iced_layershell::Result {
    <App as iced_layershell::Application>::run(Settings {
        id: Some("mde-popover-toasts".into()),
        fonts: crate::fonts::load_fallback_fonts(),
        layer_settings: LayerShellSettings {
            // Top layer so toasts sit above the panel + over any
            // normal window. Bottom-center anchor: 48px above the
            // panel's 40px exclusive zone.
            layer: Layer::Top,
            anchor: Anchor::Bottom,
            margin: (0, 0, 48, 0),
            keyboard_interactivity: KeyboardInteractivity::None,
            exclusive_zone: 0,
            size: None,
            ..Default::default()
        },
        ..Default::default()
    })
}

fn toast_pill(toast: &Toast) -> Element<'static, Message> {
    let accent = pill_accent(toast.kind);
    let body = toast.body.clone();
    container(text(body).size(13).color(FG_TEXT))
        .padding(Padding {
            top: 8.0,
            right: 16.0,
            bottom: 8.0,
            left: 16.0,
        })
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(SURFACE_BG)),
            border: Border {
                color: Color { a: 0.30, ..accent },
                width: 1.0,
                radius: 12.0.into(),
            },
            text_color: Some(FG_TEXT),
            shadow: Shadow::default(),
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_constructor_has_info_kind() {
        let t = Toast::info("hello");
        assert_eq!(t.kind, ToastKind::Info);
        assert_eq!(t.body, "hello");
    }

    #[test]
    fn variant_constructors_set_correct_kind() {
        assert_eq!(Toast::success("ok").kind, ToastKind::Success);
        assert_eq!(Toast::warn("uh").kind, ToastKind::Warn);
        assert_eq!(Toast::error("no").kind, ToastKind::Error);
    }

    #[test]
    fn is_expired_after_visible_window() {
        let mut t = Toast::info("body");
        t.visible_for = Duration::from_millis(100);
        let later = t.created_at + Duration::from_millis(150);
        assert!(t.is_expired_at(later));
    }

    #[test]
    fn is_not_expired_before_visible_window() {
        let mut t = Toast::info("body");
        t.visible_for = Duration::from_millis(2000);
        let earlier = t.created_at + Duration::from_millis(500);
        assert!(!t.is_expired_at(earlier));
    }

    #[test]
    fn stack_starts_empty() {
        let stack = ToastStack::new();
        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn stack_push_adds_a_toast() {
        let mut stack = ToastStack::new();
        stack.push(Toast::info("a"));
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn stack_evicts_oldest_when_over_limit() {
        let mut stack = ToastStack::new();
        stack.push(Toast::info("oldest"));
        stack.push(Toast::info("middle"));
        stack.push(Toast::info("newest-1"));
        stack.push(Toast::info("newest-2")); // 4th — exceeds STACK_LIMIT=3
        assert_eq!(stack.len(), STACK_LIMIT);
        // The "oldest" was evicted.
        let bodies: Vec<&str> = stack.iter().map(|t| t.body.as_str()).collect();
        assert!(!bodies.contains(&"oldest"));
        assert_eq!(bodies, vec!["middle", "newest-1", "newest-2"]);
    }

    #[test]
    fn retain_unexpired_drops_expired_toasts() {
        let mut stack = ToastStack::new();
        let mut t = Toast::info("expired");
        t.visible_for = Duration::from_millis(10);
        stack.push(t);
        let later = Instant::now() + Duration::from_millis(500);
        stack.retain_unexpired(later);
        assert!(stack.is_empty());
    }

    #[test]
    fn default_visible_window_is_2000ms() {
        assert_eq!(DEFAULT_VISIBLE_MS, 2000);
    }

    #[test]
    fn stack_limit_is_3() {
        assert_eq!(STACK_LIMIT, 3);
    }
}
