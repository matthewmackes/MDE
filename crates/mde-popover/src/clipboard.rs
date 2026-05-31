//! BUS-5.6 — Super+V clipboard history popover.
//!
//! Reads clipboard entries from the `clipboard/sync` bus topic
//! (`<bus_root>/clipboard/sync/<ulid>.json`) written by mde-clipd
//! (BUS-5.2). Falls back to the v3.0.3 legacy path
//! (`~/.cache/mde/clipboard.json`) when the bus root is absent so
//! sessions without mde-clipd running still show something.
//!
//! Features added in BUS-5.6:
//! - Type-to-filter: the filter input at the top narrows the list.
//! - j/k navigation: arrow keys or j/k move the cursor; Enter pastes.
//! - Enter pastes the highlighted entry via `wl-copy` and dismisses.
//!
//! Features added in BUS-5.7:
//! - Right-click any entry → inline "Pin / Unpin" context menu.
//! - Pinned entries appear above rolling history and survive GC TTL.
//! - Pin state persisted to `<data_home>/mde/clipboard/pins.json`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Write `text` to the clipboard via `wl-copy`. Returns `Err` on failure.
pub fn copy_text(text: &[u8], mime: &str) -> std::io::Result<()> {
    let mut child = Command::new("wl-copy")
        .args(["--type", mime])
        .stdin(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text)?;
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(std::io::Error::other(format!(
            "wl-copy exited with {status}"
        )));
    }
    Ok(())
}

// ── Wire types (mirrors crates/mde-clipd/src/publish.rs) ─────────────────

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ClipboardPayload {
    Inline { data_b64: String },
    BlobRef { path: String },
}

#[derive(Debug, Clone, serde::Deserialize)]
struct BusSyncMsg {
    publisher_peer: String,
    selected_mime: String,
    payload: ClipboardPayload,
    ts_iso: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct BusEnvelope {
    ulid: String,
    body: Option<String>,
}

// ── ClipEntry (public for tests + legacy parse) ───────────────────────────

/// One past clipboard entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ClipEntry {
    /// Unique ULID (lexicographic = time-ordered).
    pub ulid: String,
    /// Human-readable timestamp from the bus message.
    pub ts_iso: String,
    /// MIME type of the payload.
    pub mime: String,
    /// Text body (empty string for non-text or if decoding fails).
    pub body: String,
    /// Originating peer hostname.
    pub origin_peer: String,
}

// ── Pin store (BUS-5.7 — mirrors crates/mde-clipd/src/pin.rs) ────────────

mod pin {
    use std::collections::HashSet;
    use std::path::Path;

    #[derive(Default, serde::Serialize, serde::Deserialize)]
    pub struct PinStore {
        pub pinned: HashSet<String>,
    }

    impl PinStore {
        pub fn load_from(path: &Path) -> Self {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        }

        pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let json =
                serde_json::to_string(self).map_err(std::io::Error::other)?;
            let tmp = path.with_extension("tmp");
            std::fs::write(&tmp, json.as_bytes())?;
            std::fs::rename(&tmp, path)?;
            Ok(())
        }

        pub fn pin(&mut self, ulid: &str) {
            self.pinned.insert(ulid.to_owned());
        }

        pub fn unpin(&mut self, ulid: &str) {
            self.pinned.remove(ulid);
        }

        pub fn is_pinned(&self, ulid: &str) -> bool {
            self.pinned.contains(ulid)
        }
    }
}

fn clipboard_pin_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("mde/clipboard/pins.json")
}

// ── Loaders ───────────────────────────────────────────────────────────────

/// Resolve `<bus_root>/clipboard/sync/` from `$MDE_BUS_ROOT` or the
/// default XDG path.
fn bus_sync_dir() -> PathBuf {
    let base: PathBuf = std::env::var("MDE_BUS_ROOT")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("XDG_DATA_HOME")
                .ok()
                .filter(|s| !s.is_empty())
                .map(|d| PathBuf::from(d).join("mde/bus"))
        })
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".local/share/mde/bus"))
        })
        .unwrap_or_else(|| PathBuf::from("/var/lib/mde/bus"));
    base.join("clipboard/sync")
}

/// Load history from the bus topic tree. Returns entries newest-first.
fn load_from_bus(dir: &Path) -> Vec<ClipEntry> {
    let rd = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut paths: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map_or(false, |x| x == "json"))
        .collect();
    // Lexicographic order on ULID filenames = chronological.
    paths.sort();

    let mut entries = Vec::new();
    for path in paths {
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(env) = serde_json::from_str::<BusEnvelope>(&raw) else {
            continue;
        };
        let body_str = env.body.unwrap_or_default();
        let Ok(msg) = serde_json::from_str::<BusSyncMsg>(&body_str) else {
            continue;
        };
        let text_body = match &msg.payload {
            ClipboardPayload::Inline { data_b64 } => {
                use base64::Engine as _;
                base64::engine::general_purpose::STANDARD
                    .decode(data_b64)
                    .ok()
                    .and_then(|b| String::from_utf8(b).ok())
                    .unwrap_or_default()
            }
            ClipboardPayload::BlobRef { path } => {
                std::fs::read_to_string(path).unwrap_or_default()
            }
        };
        entries.push(ClipEntry {
            ulid: env.ulid,
            ts_iso: msg.ts_iso,
            mime: msg.selected_mime,
            body: text_body,
            origin_peer: msg.publisher_peer,
        });
    }
    entries.reverse(); // newest first
    entries
}

/// Parse the legacy `~/.cache/mde/clipboard.json` format.
#[derive(serde::Deserialize)]
struct LegacyEntry {
    captured_at_ms: u64,
    mime: String,
    body: String,
    origin_peer: Option<String>,
}

fn load_from_legacy(json: &str) -> Vec<ClipEntry> {
    let legacy: Vec<LegacyEntry> = serde_json::from_str(json).unwrap_or_default();
    legacy
        .into_iter()
        .enumerate()
        .map(|(_i, e)| ClipEntry {
            ulid: format!("{:020}", e.captured_at_ms),
            ts_iso: String::new(),
            mime: e.mime,
            body: e.body,
            origin_peer: e.origin_peer.unwrap_or_else(|| "local".into()),
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn default_legacy_path() -> PathBuf {
    dirs::cache_dir()
        .map(|d| d.join("mde/clipboard.json"))
        .unwrap_or_else(|| PathBuf::from("/tmp/mde-clipboard.json"))
}

/// Load history from bus topic (primary) or legacy file (fallback).
fn load_history() -> Vec<ClipEntry> {
    let dir = bus_sync_dir();
    let entries = load_from_bus(&dir);
    if !entries.is_empty() {
        return entries;
    }
    let raw = std::fs::read_to_string(default_legacy_path()).unwrap_or_default();
    load_from_legacy(&raw)
}

// ── Split helper (public for tests) ───────────────────────────────────────

/// Partition `indices` into (pinned, history), preserving relative order
/// within each group. Pinned entries appear first in the combined list.
pub fn split_by_pin<'a>(
    entries: &'a [ClipEntry],
    indices: &'a [usize],
    pin_store: &'a pin::PinStore,
) -> (Vec<usize>, Vec<usize>) {
    let mut pinned = Vec::new();
    let mut history = Vec::new();
    for &i in indices {
        if pin_store.is_pinned(&entries[i].ulid) {
            pinned.push(i);
        } else {
            history.push(i);
        }
    }
    (pinned, history)
}

// ── Iced layer-shell popover ───────────────────────────────────────────────

use iced::widget::{button, column, container, mouse_area, row, scrollable, text, text_input,
    Space};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Shadow, Subscription, Task,
    Theme,
};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

const WIDTH: u32 = 480;
const CARD_HEIGHT: u32 = 520;

const FG_TEXT: Color = Color { r: 0.957, g: 0.957, b: 0.957, a: 1.0 };
const FG_MUTED: Color = Color { r: 0.659, g: 0.659, b: 0.659, a: 1.0 };
const ACCENT: Color = Color { r: 0.169, g: 0.604, b: 0.953, a: 1.0 };
const SURFACE_BG: Color = Color { r: 0.055, g: 0.055, b: 0.063, a: 0.97 };

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    FilterChanged(String),
    Select(usize),
    CursorUp,
    CursorDown,
    PasteSelected,
    // BUS-5.7 — right-click pin actions.
    ShowContext(usize),  // real_idx into all_entries
    HideContext,
    TogglePin(String),  // ULID
    Exit,
}

pub struct App {
    all_entries: Vec<ClipEntry>,
    filter: String,
    cursor: usize,
    // BUS-5.7 — pin state.
    pin_store: pin::PinStore,
    pin_path: PathBuf,
    context_entry: Option<usize>,  // real_idx of entry showing context menu
}

impl App {
    /// Returns matching entry indices, pinned first, then newest-first history.
    fn filtered_indices(&self) -> Vec<usize> {
        let q = self.filter.to_lowercase();
        let mut indices: Vec<usize> = self.all_entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                q.is_empty()
                    || e.body.to_lowercase().contains(&q)
                    || e.origin_peer.to_lowercase().contains(&q)
                    || e.mime.to_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .take(100)
            .collect();
        // Stable sort: pinned first (false < true for !is_pinned key).
        indices.sort_by_key(|&i| !self.pin_store.is_pinned(&self.all_entries[i].ulid));
        indices
    }

    fn paste_entry(&self, idx: usize) {
        if let Some(entry) = self.all_entries.get(idx) {
            let bytes = entry.body.as_bytes();
            if let Err(e) = copy_text(bytes, &entry.mime) {
                tracing::warn!(error = %e, "clipboard popover copy failed");
                crate::toasts::emit(&crate::toasts::ToastEvent {
                    body: "clipboard copy failed".into(),
                    kind: crate::toasts::ToastKindWire::Error,
                    visible_ms: None,
                });
            } else {
                crate::toasts::emit(&crate::toasts::ToastEvent {
                    body: format!("Copied: {}", preview(&entry.body, 40)),
                    kind: crate::toasts::ToastKindWire::Success,
                    visible_ms: None,
                });
            }
        }
    }
}

fn namespace() -> String {
    "mde-popover-clipboard".into()
}

fn update(state: &mut App, msg: Message) -> Task<Message> {
    match msg {
        Message::FilterChanged(s) => {
            state.filter = s;
            state.cursor = 0;
            state.context_entry = None;
            Task::none()
        }
        Message::CursorUp => {
            if state.cursor > 0 {
                state.cursor -= 1;
            }
            Task::none()
        }
        Message::CursorDown => {
            let max = state.filtered_indices().len().saturating_sub(1);
            if state.cursor < max {
                state.cursor += 1;
            }
            Task::none()
        }
        Message::PasteSelected => {
            let indices = state.filtered_indices();
            if let Some(&real_idx) = indices.get(state.cursor) {
                state.paste_entry(real_idx);
            }
            std::process::exit(0);
        }
        Message::Select(real_idx) => {
            state.paste_entry(real_idx);
            std::process::exit(0);
        }
        Message::ShowContext(real_idx) => {
            state.context_entry = Some(real_idx);
            Task::none()
        }
        Message::HideContext => {
            state.context_entry = None;
            Task::none()
        }
        Message::TogglePin(ulid) => {
            if state.pin_store.is_pinned(&ulid) {
                state.pin_store.unpin(&ulid);
            } else {
                state.pin_store.pin(&ulid);
            }
            if let Err(e) = state.pin_store.save_to(&state.pin_path) {
                tracing::warn!(error = %e, "clipboard: failed to save pin store");
            }
            state.context_entry = None;
            Task::none()
        }
        Message::Exit => std::process::exit(0),
        _ => Task::none(),
    }
}

fn view(state: &App) -> Element<'_, Message> {
    let indices = state.filtered_indices();
    let count = state.all_entries.len();
    let pinned_count = state
        .all_entries
        .iter()
        .filter(|e| state.pin_store.is_pinned(&e.ulid))
        .count();

    let header_label = if pinned_count > 0 {
        format!("{count} entries · {pinned_count} pinned")
    } else {
        format!("{count} entries · bus-synced")
    };
    let header = container(
        row![
            text("Clipboard").size(13).color(FG_TEXT),
            Space::new().width(Length::Fixed(8.0)),
            text(header_label).size(10).color(FG_MUTED),
            Space::new().width(Length::Fill),
            crate::dismiss::close_button(Message::Exit),
        ]
        .align_y(Alignment::Center),
    )
    .padding(Padding { top: 8.0, right: 12.0, bottom: 4.0, left: 12.0 });

    let filter_row = container(
        text_input("Filter…", &state.filter)
            .on_input(Message::FilterChanged)
            .size(13)
            .padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 })
            .style(|_theme, _status| text_input::Style {
                background: Background::Color(Color {
                    r: ACCENT.r,
                    g: ACCENT.g,
                    b: ACCENT.b,
                    a: 0.08,
                }),
                border: Border {
                    color: Color { r: ACCENT.r, g: ACCENT.g, b: ACCENT.b, a: 0.35 },
                    width: 1.0,
                    radius: 4.0.into(),
                },
                icon: FG_MUTED,
                placeholder: FG_MUTED,
                value: FG_TEXT,
                selection: Color { r: ACCENT.r, g: ACCENT.g, b: ACCENT.b, a: 0.30 },
            }),
    )
    .padding(Padding { top: 0.0, right: 12.0, bottom: 6.0, left: 12.0 });

    let (pinned_real, history_real) = split_by_pin(&state.all_entries, &indices, &state.pin_store);
    let cursor = state.cursor;
    let mut list = column![].spacing(2);

    if indices.is_empty() {
        list = list.push(
            container(
                text(if state.filter.is_empty() {
                    "No clipboard history yet"
                } else {
                    "No matches"
                })
                .size(13)
                .color(FG_MUTED),
            )
            .padding(Padding { top: 28.0, right: 0.0, bottom: 0.0, left: 12.0 }),
        );
    }

    let mut list_pos: usize = 0;

    if !pinned_real.is_empty() {
        list = list.push(section_label("Pinned"));
        for &real_idx in &pinned_real {
            let entry = &state.all_entries[real_idx];
            let is_cursor = list_pos == cursor;
            list = list.push(entry_row(entry, real_idx, is_cursor, true));
            if state.context_entry == Some(real_idx) {
                list = list.push(context_menu_row(entry.ulid.clone(), true));
            }
            list_pos += 1;
        }
        if !history_real.is_empty() {
            list = list.push(section_label("History"));
        }
    }

    for &real_idx in &history_real {
        let entry = &state.all_entries[real_idx];
        let is_cursor = list_pos == cursor;
        list = list.push(entry_row(entry, real_idx, is_cursor, false));
        if state.context_entry == Some(real_idx) {
            list = list.push(context_menu_row(entry.ulid.clone(), false));
        }
        list_pos += 1;
    }

    let scroll = scrollable(list).height(Length::Fill);

    let footer = container(
        text("↑↓/jk navigate · Enter paste · right-click pin · Esc close")
            .size(10)
            .color(FG_MUTED),
    )
    .padding(Padding { top: 4.0, right: 12.0, bottom: 8.0, left: 12.0 });

    let body = column![header, filter_row, scroll, footer]
        .padding(Padding { top: 4.0, right: 4.0, bottom: 4.0, left: 4.0 });

    let card: Element<'_, Message> = container(body)
        .width(Length::Fixed(WIDTH as f32))
        .height(Length::Fixed(CARD_HEIGHT as f32))
        .style(popover_surface)
        .into();

    let dismiss = || {
        mouse_area(
            container(Space::new().width(Length::Fill))
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
    .height(Length::Fixed((CARD_HEIGHT + 48) as f32));
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

fn subscription(_state: &App) -> Subscription<Message> {
    use iced::event;
    event::listen_with(|event, status, _window| {
        use iced::keyboard;
        match event {
            iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. })
                if status == event::Status::Ignored =>
            {
                use iced::keyboard::{key::Named, Key};
                match key {
                    Key::Named(Named::Escape) => Some(Message::Exit),
                    Key::Named(Named::Enter) => Some(Message::PasteSelected),
                    Key::Named(Named::ArrowUp) => Some(Message::CursorUp),
                    Key::Named(Named::ArrowDown) => Some(Message::CursorDown),
                    Key::Character(ref c) if c.as_str() == "j" => Some(Message::CursorDown),
                    Key::Character(ref c) if c.as_str() == "k" => Some(Message::CursorUp),
                    _ => None,
                }
            }
            _ => None,
        }
    })
}

pub fn run() -> iced_layershell::Result {
    iced_layershell::application(
        || {
            let all_entries = load_history();
            let pin_path = clipboard_pin_path();
            let pin_store = pin::PinStore::load_from(&pin_path);
            tracing::info!(count = all_entries.len(), "clipboard popover loaded");
            App {
                all_entries,
                filter: String::new(),
                cursor: 0,
                pin_store,
                pin_path,
                context_entry: None,
            }
        },
        namespace,
        update,
        view,
    )
    .theme(|_: &App| iced::Theme::Dark)
    .subscription(subscription)
    .settings(Settings {
        id: Some("mde-popover-clipboard".into()),
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

// ── Widget helpers ────────────────────────────────────────────────────────

/// One clipboard entry row wrapped in a mouse_area for right-click detection.
fn entry_row<'a>(
    entry: &'a ClipEntry,
    real_idx: usize,
    is_cursor: bool,
    is_pinned: bool,
) -> Element<'a, Message> {
    let pin_mark: Element<'_, Message> = if is_pinned {
        text("📌").size(10).color(ACCENT).into()
    } else {
        Space::new().width(Length::Fixed(0.0)).into()
    };

    let row_btn = button(
        row![
            column![
                text(preview(&entry.body, 72)).size(13).color(FG_TEXT),
                Space::new().height(Length::Fixed(2.0)),
                text(format!("{} · {}", &entry.origin_peer, &entry.mime))
                    .size(10)
                    .color(FG_MUTED),
            ]
            .width(Length::Fill),
            pin_mark,
        ]
        .align_y(Alignment::Center)
        .padding(Padding { top: 6.0, right: 12.0, bottom: 6.0, left: 12.0 }),
    )
    .width(Length::Fill)
    .style(move |_theme, status| history_row_style(status, is_cursor))
    .on_press(Message::Select(real_idx));

    mouse_area(row_btn)
        .on_right_press(Message::ShowContext(real_idx))
        .into()
}

/// Inline context menu that appears below the right-clicked row.
fn context_menu_row<'a>(ulid: String, is_pinned: bool) -> Element<'a, Message> {
    let pin_label = if is_pinned { "Unpin" } else { "Pin" };
    let ulid_for_btn = ulid;
    container(
        row![
            button(text(pin_label).size(12).color(FG_TEXT))
                .on_press(Message::TogglePin(ulid_for_btn))
                .style(|_theme, status| {
                    let bg = match status {
                        button::Status::Hovered | button::Status::Pressed => {
                            Some(Background::Color(Color {
                                r: ACCENT.r, g: ACCENT.g, b: ACCENT.b, a: 0.20,
                            }))
                        }
                        _ => Some(Background::Color(Color {
                            r: ACCENT.r, g: ACCENT.g, b: ACCENT.b, a: 0.10,
                        })),
                    };
                    button::Style {
                        background: bg,
                        text_color: FG_TEXT,
                        border: Border {
                            color: Color { r: ACCENT.r, g: ACCENT.g, b: ACCENT.b, a: 0.40 },
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        shadow: Shadow::default(),
                        snap: false,
                    }
                }),
            Space::new().width(Length::Fill),
            button(text("✕").size(11).color(FG_MUTED))
                .on_press(Message::HideContext)
                .style(|_theme, status| {
                    let alpha = match status {
                        button::Status::Hovered | button::Status::Pressed => 0.12,
                        _ => 0.0,
                    };
                    button::Style {
                        background: Some(Background::Color(Color {
                            r: FG_MUTED.r, g: FG_MUTED.g, b: FG_MUTED.b, a: alpha,
                        })),
                        text_color: FG_MUTED,
                        border: Border::default(),
                        shadow: Shadow::default(),
                        snap: false,
                    }
                }),
        ]
        .align_y(Alignment::Center),
    )
    .padding(Padding { top: 3.0, right: 8.0, bottom: 3.0, left: 8.0 })
    .style(|_| container::Style {
        background: Some(Background::Color(Color {
            r: ACCENT.r, g: ACCENT.g, b: ACCENT.b, a: 0.08,
        })),
        border: Border {
            color: Color { r: ACCENT.r, g: ACCENT.g, b: ACCENT.b, a: 0.25 },
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        text_color: None,
        snap: false,
    })
    .into()
}

/// Section label row ("Pinned" / "History").
fn section_label<'a>(label: &'static str) -> Element<'a, Message> {
    container(
        text(label)
            .size(10)
            .color(FG_MUTED),
    )
    .padding(Padding { top: 4.0, right: 12.0, bottom: 1.0, left: 12.0 })
    .into()
}

// ── Helpers ───────────────────────────────────────────────────────────────

/// Truncate `body` to `max` chars + collapse newlines.
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
            color: Color { r: 0.957, g: 0.957, b: 0.957, a: 0.10 },
            width: 1.0,
            radius: 8.0.into(),
        },
        text_color: Some(FG_TEXT),
        shadow: Shadow::default(),
        snap: false,
    }
}

fn history_row_style(status: button::Status, is_cursor: bool) -> button::Style {
    let base_alpha: f32 = if is_cursor { 0.16 } else { 0.0 };
    let bg = match status {
        button::Status::Hovered => Some(Background::Color(Color {
            r: ACCENT.r, g: ACCENT.g, b: ACCENT.b, a: (base_alpha + 0.10).min(1.0),
        })),
        button::Status::Pressed => Some(Background::Color(Color {
            r: ACCENT.r, g: ACCENT.g, b: ACCENT.b, a: (base_alpha + 0.18).min(1.0),
        })),
        _ if is_cursor => Some(Background::Color(Color {
            r: ACCENT.r, g: ACCENT.g, b: ACCENT.b, a: base_alpha,
        })),
        _ => None,
    };
    button::Style {
        background: bg,
        text_color: FG_TEXT,
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 4.0.into() },
        shadow: Shadow::default(),
        snap: false,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(ulid: &str, body: &str) -> ClipEntry {
        ClipEntry {
            ulid: ulid.to_string(),
            ts_iso: String::new(),
            mime: "text/plain".into(),
            body: body.to_string(),
            origin_peer: "local".into(),
        }
    }

    // ── load_from_legacy ──────────────────────────────────────────────────

    #[test]
    fn load_from_legacy_empty_string_returns_empty() {
        assert!(load_from_legacy("").is_empty());
    }

    #[test]
    fn load_from_legacy_malformed_returns_empty() {
        assert!(load_from_legacy("{not json}").is_empty());
    }

    #[test]
    fn load_from_legacy_round_trips() {
        let json = r#"[{"captured_at_ms":1000,"mime":"text/plain","body":"hello","origin_peer":"lab-01"}]"#;
        let entries = load_from_legacy(json);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].body, "hello");
        assert_eq!(entries[0].origin_peer, "lab-01");
        assert_eq!(entries[0].mime, "text/plain");
    }

    #[test]
    fn load_from_legacy_newest_first() {
        let json = r#"[{"captured_at_ms":1000,"mime":"text/plain","body":"first","origin_peer":null},
                       {"captured_at_ms":2000,"mime":"text/plain","body":"second","origin_peer":null}]"#;
        let entries = load_from_legacy(json);
        assert_eq!(entries[0].body, "second");
        assert_eq!(entries[1].body, "first");
    }

    // ── load_from_bus ─────────────────────────────────────────────────────

    #[test]
    fn load_from_bus_empty_dir_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_from_bus(dir.path()).is_empty());
    }

    #[test]
    fn load_from_bus_missing_dir_returns_empty() {
        let path = std::path::PathBuf::from("/tmp/mde-clipd-test-nonexistent-dir-abc123");
        assert!(load_from_bus(&path).is_empty());
    }

    #[test]
    fn load_from_bus_parses_inline_entry() {
        use base64::Engine as _;
        let dir = tempfile::tempdir().unwrap();
        let body_b64 = base64::engine::general_purpose::STANDARD.encode("hello world");
        let msg = serde_json::json!({
            "publisher_peer": "peer-01",
            "mime_types": ["text/plain"],
            "selected_mime": "text/plain",
            "payload": { "kind": "inline", "data_b64": body_b64 },
            "ts_iso": "2026-05-30T00:00:00Z"
        });
        let env = serde_json::json!({
            "ulid": "01HWAAAAAAAAAAAAAAAAAAAAAA",
            "topic": "clipboard/sync",
            "priority": "normal",
            "title": null,
            "body": msg.to_string(),
            "ts_unix_ms": 1748563200000i64,
            "file_path": "clipboard/sync/01HWAAAAAAAAAAAAAAAAAAAAAA.json"
        });
        std::fs::write(
            dir.path().join("01HWAAAAAAAAAAAAAAAAAAAAAA.json"),
            serde_json::to_string_pretty(&env).unwrap(),
        )
        .unwrap();
        let entries = load_from_bus(dir.path());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].body, "hello world");
        assert_eq!(entries[0].origin_peer, "peer-01");
        assert_eq!(entries[0].mime, "text/plain");
        assert_eq!(entries[0].ts_iso, "2026-05-30T00:00:00Z");
    }

    #[test]
    fn load_from_bus_newest_first() {
        use base64::Engine as _;
        let dir = tempfile::tempdir().unwrap();
        for (ulid, body) in [
            ("01AAAAAAAAAAAAAAAAAAAAAAAA", "older"),
            ("01ZZZZZZZZZZZZZZZZZZZZZZZZ", "newer"),
        ] {
            let b64 = base64::engine::general_purpose::STANDARD.encode(body);
            let msg = serde_json::json!({
                "publisher_peer": "local",
                "mime_types": ["text/plain"],
                "selected_mime": "text/plain",
                "payload": { "kind": "inline", "data_b64": b64 },
                "ts_iso": ""
            });
            let env = serde_json::json!({
                "ulid": ulid,
                "topic": "clipboard/sync",
                "priority": "normal",
                "title": null,
                "body": msg.to_string(),
                "ts_unix_ms": 0i64,
                "file_path": ""
            });
            std::fs::write(
                dir.path().join(format!("{ulid}.json")),
                serde_json::to_string_pretty(&env).unwrap(),
            )
            .unwrap();
        }
        let entries = load_from_bus(dir.path());
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].body, "newer");
        assert_eq!(entries[1].body, "older");
    }

    #[test]
    fn load_from_bus_skips_malformed_entries() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bad.json"), "not json").unwrap();
        assert!(load_from_bus(dir.path()).is_empty());
    }

    // ── split_by_pin (BUS-5.7) ────────────────────────────────────────────

    #[test]
    fn split_pinned_first_then_history() {
        let entries = vec![
            make_entry("ULID-A", "first"),
            make_entry("ULID-B", "second"),
            make_entry("ULID-C", "third"),
        ];
        let mut ps = pin::PinStore::default();
        ps.pin("ULID-B");
        let all_indices = vec![0usize, 1, 2];
        let (pinned, history) = split_by_pin(&entries, &all_indices, &ps);
        assert_eq!(pinned, vec![1]);       // ULID-B is pinned
        assert_eq!(history, vec![0, 2]);   // A and C are history
    }

    #[test]
    fn split_all_unpinned_returns_empty_pinned() {
        let entries = vec![make_entry("A", "x"), make_entry("B", "y")];
        let ps = pin::PinStore::default();
        let (pinned, history) = split_by_pin(&entries, &[0, 1], &ps);
        assert!(pinned.is_empty());
        assert_eq!(history, vec![0, 1]);
    }

    #[test]
    fn split_all_pinned_returns_empty_history() {
        let entries = vec![make_entry("A", "x"), make_entry("B", "y")];
        let mut ps = pin::PinStore::default();
        ps.pin("A");
        ps.pin("B");
        let (pinned, history) = split_by_pin(&entries, &[0, 1], &ps);
        assert_eq!(pinned, vec![0, 1]);
        assert!(history.is_empty());
    }

    #[test]
    fn split_empty_indices_returns_empty() {
        let entries: Vec<ClipEntry> = Vec::new();
        let ps = pin::PinStore::default();
        let (pinned, history) = split_by_pin(&entries, &[], &ps);
        assert!(pinned.is_empty());
        assert!(history.is_empty());
    }

    // ── pin store round-trip (BUS-5.7) ────────────────────────────────────

    #[test]
    fn pin_store_save_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pins.json");
        let mut ps = pin::PinStore::default();
        ps.pin("ULID-X");
        ps.save_to(&path).unwrap();
        let loaded = pin::PinStore::load_from(&path);
        assert!(loaded.is_pinned("ULID-X"));
        assert!(!loaded.is_pinned("ULID-NEVER"));
    }

    #[test]
    fn pin_store_unpin_persists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pins.json");
        let mut ps = pin::PinStore::default();
        ps.pin("ULID-X");
        ps.save_to(&path).unwrap();
        ps.unpin("ULID-X");
        ps.save_to(&path).unwrap();
        let loaded = pin::PinStore::load_from(&path);
        assert!(!loaded.is_pinned("ULID-X"));
    }

    // ── preview ───────────────────────────────────────────────────────────

    #[test]
    fn preview_truncates_long_text() {
        let long = "a".repeat(200);
        let p = preview(&long, 80);
        assert!(p.chars().count() <= 80);
        assert!(p.ends_with('…'));
    }

    #[test]
    fn preview_collapses_newlines() {
        let s = "line1\nline2";
        assert_eq!(preview(s, 80), "line1 line2");
    }

    #[test]
    fn preview_short_text_unchanged() {
        let s = "hello";
        assert_eq!(preview(s, 80), "hello");
    }
}
