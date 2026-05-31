//! Notifications popover — recent notifications list.
//!
//! Anchored bottom-right of the primary output above the panel.
//! Reads `~/.cache/mackes/notifications.json` (the same cache the
//! notification-bell applet polls) and renders the rows grouped by
//! peer, with phone-origin rows badged via the locked glyph.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use iced::widget::{column, container, mouse_area, row, scrollable, text, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Subscription, Task, Theme};
use mde_theme::motion::list::{STAGGER_CAP, STAGGER_REVEAL_MS, STAGGER_STEP_MS};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;
use mde_applet_notifications::{
    group_and_sort, group_by_app, is_phone_origin, notifications_cache_path,
    parse_notifications, visible, NotificationRow,
};

const WIDTH: u32 = 480;
const HEIGHT: u32 = 600;

// BUS-2.3 — priority accent colors (Carbon red/orange/blue, matching BUS-2.2).
const BUS_URGENT_COLOR: Color = Color { r: 0.91, g: 0.30, b: 0.36, a: 1.0 };
const BUS_HIGH_COLOR: Color   = Color { r: 0.93, g: 0.55, b: 0.21, a: 1.0 };
const BUS_DEFAULT_COLOR: Color = Color { r: 0.20, g: 0.69, b: 1.00, a: 1.0 };

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
const FG_FAINT: Color = Color {
    r: 0.45,
    g: 0.45,
    b: 0.45,
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

// ──────────────────────────────────────────────────────────────
// ANIM-3.b.2 — entrance stagger + dismiss-fade animation
// ──────────────────────────────────────────────────────────────

/// Bus-ack dismiss-fade duration (ms). Row fades to 0 then
/// moves to the Acked section.
const DISMISS_ANIM_MS: u64 = 200;

/// Total entrance-animation window (ms): last stagger delay + reveal.
/// After this many ms from open, the entrance animation is complete.
const MAX_ENTRANCE_MS: u64 =
    (STAGGER_CAP as u64 - 1) * STAGGER_STEP_MS as u64 + STAGGER_REVEAL_MS as u64;

/// Per-row entrance alpha at `opened_ms` since the popover opened.
/// Uses the same capped-8 stagger + ease-out-sqrt pattern as ANIM-4.
fn stagger_alpha(row_index: usize, opened_ms: u64) -> f32 {
    let delay =
        row_index.min(STAGGER_CAP.saturating_sub(1)) as u64 * STAGGER_STEP_MS as u64;
    let elapsed = opened_ms.saturating_sub(delay);
    let t = (elapsed as f32 / STAGGER_REVEAL_MS as f32).clamp(0.0, 1.0);
    t.sqrt()
}

/// Dismiss alpha for a Bus row whose ack fired at `start`.
/// Linear fade from 1.0 → 0.0 over `DISMISS_ANIM_MS`.
fn dismiss_alpha(start: Instant) -> f32 {
    let elapsed = start.elapsed().as_millis() as u64;
    let t = (elapsed as f32 / DISMISS_ANIM_MS as f32).clamp(0.0, 1.0);
    1.0 - t
}

// ──────────────────────────────────────────────────────────────
// BUS-2.3 — Bus message integration (pure data layer)
// ──────────────────────────────────────────────────────────────

/// A Bus message loaded from the GFS file tree for popover display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusPopoverMessage {
    pub ulid: String,
    pub topic: String,
    pub priority: String,
    pub title: String,
    pub body: String,
}

/// Resolve `$XDG_DATA_HOME/mde/bus` (or `~/.local/share/mde/bus`).
#[must_use]
pub fn bus_data_root() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("mde").join("bus"))
}

/// Parse one Bus JSON file from the store.
/// Returns `None` for `min`-priority or malformed files.
/// Mirrors `parse_breadcrumb_file` from `mde-portal/src/workspace.rs`.
#[must_use]
pub fn parse_bus_message(path: &Path, ulid: &str, topic: &str) -> Option<BusPopoverMessage> {
    let raw = std::fs::read_to_string(path).ok()?;
    let outer: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let priority = outer
        .get("priority")
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string();
    if priority == "min" {
        return None;
    }
    let title = outer
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let body = outer
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Some(BusPopoverMessage { ulid: ulid.to_string(), topic: topic.to_string(), priority, title, body })
}

/// Walk `dir` recursively, collecting `(topic, ulid, path)` for each
/// ULID-named `.json` file. Topic is the relative path of the parent
/// directory from `bus_root` with `/` separators.
fn collect_bus_files(dir: &Path, bus_root: &Path, out: &mut Vec<(String, String, PathBuf)>) {
    let Ok(iter) = std::fs::read_dir(dir) else { return };
    for entry in iter.flatten() {
        let path = entry.path();
        let name_os = entry.file_name();
        let name = name_os.to_str().unwrap_or("");
        if path.is_dir() {
            if !name.starts_with('.') {
                collect_bus_files(&path, bus_root, out);
            }
        } else if name.ends_with(".json") {
            let parent = path.parent().unwrap_or(bus_root);
            let rel = parent
                .strip_prefix(bus_root)
                .map(|p| p.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/"))
                .unwrap_or_default();
            // Skip top-level (index.sqlite sibling) and audit/ paths.
            if rel.is_empty() || rel.starts_with("audit") {
                continue;
            }
            let ulid = name.trim_end_matches(".json").to_string();
            out.push((rel, ulid, path));
        }
    }
}

/// Load all displayable Bus messages from `bus_root`, skipping
/// `min`-priority, audit topics, and malformed files.
#[must_use]
pub fn load_bus_messages(bus_root: &Path) -> Vec<BusPopoverMessage> {
    let mut triples: Vec<(String, String, PathBuf)> = Vec::new();
    collect_bus_files(bus_root, bus_root, &mut triples);
    triples
        .into_iter()
        .filter_map(|(topic, ulid, path)| parse_bus_message(&path, &ulid, &topic))
        .collect()
}

/// Partition `messages` (excluding `acked` ULIDs) into
/// `(urgent, high, default)` buckets, newest-first within each.
#[must_use]
pub fn bucket_by_priority<'a>(
    messages: &'a [BusPopoverMessage],
    acked: &std::collections::HashSet<String>,
) -> (Vec<&'a BusPopoverMessage>, Vec<&'a BusPopoverMessage>, Vec<&'a BusPopoverMessage>) {
    let active: Vec<&BusPopoverMessage> =
        messages.iter().filter(|m| !acked.contains(&m.ulid)).collect();
    let mut urgent: Vec<&BusPopoverMessage> =
        active.iter().copied().filter(|m| m.priority == "urgent").collect();
    let mut high: Vec<&BusPopoverMessage> =
        active.iter().copied().filter(|m| m.priority == "high").collect();
    let mut default: Vec<&BusPopoverMessage> = active
        .iter()
        .copied()
        .filter(|m| m.priority != "urgent" && m.priority != "high")
        .collect();
    for bucket in [&mut urgent, &mut high, &mut default] {
        bucket.sort_by(|a, b| b.ulid.cmp(&a.ulid));
    }
    (urgent, high, default)
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Exit,
    /// BUG-8.a (2026-05-23) — clear the cache file then exit.
    ClearAll,
    /// ANIM-3.b.2 — drives entrance stagger + dismiss-fade frames.
    AnimTick,
    /// BUG-8.b (2026-05-23) — toggle the mute state for a peer
    /// group. Writes the new state to
    /// `~/.config/mde/notification-mutes.toml` and refreshes the
    /// in-memory groups list so muted peers disappear
    /// immediately.
    ToggleMute(String),
    /// BUG-8.c (2026-05-23) — flip between peer-grouped and
    /// app-grouped layouts.
    ToggleGroupMode,
    /// BUG-8.c — collapse / expand a single group bucket. Key
    /// is the bucket label (peer name or app_id).
    ToggleCollapse(String),
    /// BUS-2.3 — move a Bus message ULID to the acked-list.
    AckBusMessage(String),
}

/// BUG-8.c — group layout selector. Default is `Peer` so existing
/// users see the previously-shipped layout without surprise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupMode {
    Peer,
    App,
}

pub struct App {
    groups: Vec<(String, Vec<NotificationRow>)>,
    /// BUG-8.b — set of peer names currently muted. Backed by
    /// `~/.config/mde/notification-mutes.toml`. When a peer is
    /// in this set, its group is filtered out of `groups`
    /// before render.
    muted_peers: std::collections::HashSet<String>,
    /// BUG-8.c — active group-by selector. Survives a single
    /// popover open; not persisted across sessions (the user
    /// re-clicks "By app" each time they want it, matching
    /// nm-connection-editor-style transient view options).
    group_mode: GroupMode,
    /// BUG-8.c — bucket keys the user has collapsed. Persists
    /// for the popover's lifetime. Click the header to flip.
    collapsed: std::collections::HashSet<String>,
    /// BUS-2.3 — Bus messages loaded from the GFS file tree at open.
    bus_messages: Vec<BusPopoverMessage>,
    /// BUS-2.3 — ULIDs the operator has acked; filtered from the
    /// active buckets and shown in the acked-list section instead.
    bus_acked: std::collections::HashSet<String>,
    /// ANIM-3.b.2 — when the popover was created; drives entrance stagger.
    opened_at: Instant,
    /// ANIM-3.b.2 — Bus ULIDs currently in dismiss-fade; maps ulid →
    /// ack-start time. Row fades out, then moves to `bus_acked` once
    /// `DISMISS_ANIM_MS` has elapsed.
    dismissing: HashMap<String, Instant>,
}

fn namespace() -> String {
    "mde-popover-notifications".to_string()
}

fn update(state: &mut App, msg: Message) -> Task<Message> {
    match msg {
        Message::Exit => std::process::exit(0),
        Message::ClearAll => {
            // BUG-8.a — empty the cache file (atomic via
            // write to "") so the next open of any source
            // re-reads zero notifications. Then exit so
            // the operator sees the cleared state on next
            // open.
            let path = notifications_cache_path();
            let _ = std::fs::write(&path, "");
            std::process::exit(0);
        }
        Message::ToggleMute(peer) => {
            // BUG-8.b — flip the mute state for `peer`,
            // persist to ~/.config/mde/notification-mutes.toml,
            // and refresh the in-memory groups so the peer's
            // rows disappear (or reappear) immediately.
            if state.muted_peers.contains(&peer) {
                state.muted_peers.remove(&peer);
            } else {
                state.muted_peers.insert(peer);
            }
            let _ = save_muted_peers(&state.muted_peers);
            state.groups = load_groups_for(state.group_mode, &state.muted_peers);
            Task::none()
        }
        Message::ToggleGroupMode => {
            state.group_mode = match state.group_mode {
                GroupMode::Peer => GroupMode::App,
                GroupMode::App => GroupMode::Peer,
            };
            // Reset collapses on mode flip — the bucket
            // keys mean different things across modes.
            state.collapsed.clear();
            state.groups = load_groups_for(state.group_mode, &state.muted_peers);
            Task::none()
        }
        Message::ToggleCollapse(key) => {
            if state.collapsed.contains(&key) {
                state.collapsed.remove(&key);
            } else {
                state.collapsed.insert(key);
            }
            Task::none()
        }
        Message::AckBusMessage(ulid) => {
            // ANIM-3.b.2 — start dismiss fade instead of instant ack.
            // The row fades out; AnimTick moves it to bus_acked once done.
            state.dismissing.insert(ulid, Instant::now());
            Task::none()
        }
        Message::AnimTick => {
            // Advance dismiss animations: completed ones move to bus_acked.
            let done: Vec<String> = state
                .dismissing
                .iter()
                .filter(|(_, start)| start.elapsed().as_millis() as u64 >= DISMISS_ANIM_MS)
                .map(|(k, _)| k.clone())
                .collect();
            for ulid in done {
                state.dismissing.remove(&ulid);
                state.bus_acked.insert(ulid);
            }
            Task::none()
        }
        _ => Task::none(),
    }
}

fn view(state: &App) -> Element<'_, Message> {
    let header = text("Notifications").size(14).color(FG_TEXT);
    let total_rows: usize = state.groups.iter().map(|(_, r)| r.len()).sum();
    let subhead = text(format!("{total_rows} total"))
        .size(11)
        .color(FG_MUTED);

    // ANIM-3.b.2 — shared animation state for this frame.
    let opened_ms = state.opened_at.elapsed().as_millis() as u64;
    let mut row_idx: usize = 0;

    let mut list = column![].spacing(10);
    if state.groups.is_empty() {
        list = list.push(
            container(text("No notifications").size(13).color(FG_MUTED))
                .padding(Padding {
                    top: 28.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: 0.0,
                }),
        );
    }
    for (group_name, rows) in &state.groups {
        let label_text = if group_name.is_empty() {
            "Local".to_string()
        } else {
            group_name.clone()
        };
        // BUG-8.c — collapsed flag drives the chevron glyph
        // + body visibility.
        let is_collapsed = state.collapsed.contains(group_name);
        let chevron = if is_collapsed { "▶" } else { "▼" };
        let header_label = format!("{chevron}  {label_text}  ({})", rows.len());
        let collapse_key = group_name.clone();
        let header_btn: Element<'_, Message> = iced::widget::Button::new(
            text(header_label).size(11).color(FG_TEXT),
        )
        .padding(Padding {
            top: 2.0,
            right: 8.0,
            bottom: 2.0,
            left: 8.0,
        })
        .style(|_t: &Theme, status: iced::widget::button::Status| {
            let bg = match status {
                iced::widget::button::Status::Hovered => Color {
                    r: 0.14,
                    g: 0.14,
                    b: 0.16,
                    a: 1.0,
                },
                _ => Color::TRANSPARENT,
            };
            iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                text_color: FG_TEXT,
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 3.0.into(),
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .on_press(Message::ToggleCollapse(collapse_key))
        .into();

        // BUG-8.b — per-peer Mute button only makes sense in
        // peer-grouped mode; hide it in app-grouped mode
        // (muting "firefox" doesn't have the same wire
        // semantics — that would be a future BUG-8.d).
        let mute_btn: Element<'_, Message> = if state.group_mode == GroupMode::Peer {
            let peer_for_mute = group_name.clone();
            iced::widget::Button::new(text("Mute").size(10).color(FG_MUTED))
                .padding(Padding {
                    top: 2.0,
                    right: 8.0,
                    bottom: 2.0,
                    left: 8.0,
                })
                .style(|_t: &Theme, status: iced::widget::button::Status| {
                    let bg = match status {
                        iced::widget::button::Status::Hovered => Color {
                            r: 0.18,
                            g: 0.18,
                            b: 0.20,
                            a: 1.0,
                        },
                        _ => Color::TRANSPARENT,
                    };
                    iced::widget::button::Style {
                        background: Some(Background::Color(bg)),
                        text_color: FG_MUTED,
                        border: Border {
                            color: Color {
                                a: 0.12,
                                ..Color::WHITE
                            },
                            width: 1.0,
                            radius: 3.0.into(),
                        },
                        shadow: Shadow::default(),
                        snap: false,
                    }
                })
                .on_press(Message::ToggleMute(peer_for_mute))
                .into()
        } else {
            Space::new().into()
        };

        let group_header = row![
            header_btn,
            Space::new().width(Length::Fill),
            mute_btn,
        ]
        .align_y(iced::Alignment::Center);

        let mut group_column = column![group_header].spacing(4);
        if !is_collapsed {
            for row_data in rows.iter().take(40) {
                let alpha = stagger_alpha(row_idx, opened_ms);
                row_idx += 1;
                group_column = group_column.push(render_row(row_data, alpha));
            }
        }
        list = list.push(group_column);
    }

    // BUS-2.3 — Bus Messages section (priority-bucketed, below FDO).
    {
        let (urgent, high, default) = bucket_by_priority(&state.bus_messages, &state.bus_acked);
        let acked_msgs: Vec<&BusPopoverMessage> = state
            .bus_messages
            .iter()
            .filter(|m| state.bus_acked.contains(&m.ulid))
            .collect();
        let has_bus = !urgent.is_empty() || !high.is_empty() || !default.is_empty() || !acked_msgs.is_empty();
        if !state.bus_messages.is_empty() || has_bus {
            // Section divider + header
            list = list.push(Space::new().height(Length::Fixed(8.0)));
            list = list.push(
                container(Space::new().height(Length::Fixed(1.0)))
                    .width(Length::Fill)
                    .style(|_: &Theme| container::Style {
                        background: Some(iced::Background::Color(Color { r: 1.0, g: 1.0, b: 1.0, a: 0.08 })),
                        ..Default::default()
                    }),
            );
            list = list.push(Space::new().height(Length::Fixed(6.0)));
            let bus_active_total = urgent.len() + high.len() + default.len();
            list = list.push(
                text(format!("Bus Messages  ({bus_active_total} active)"))
                    .size(11)
                    .color(FG_FAINT),
            );
            // Urgent bucket
            if !urgent.is_empty() {
                list = list.push(
                    text(format!("⚠ Urgent  ({})", urgent.len()))
                        .size(10)
                        .color(BUS_URGENT_COLOR),
                );
                for msg in urgent.iter().take(20) {
                    let alpha = if let Some(&start) = state.dismissing.get(&msg.ulid) {
                        dismiss_alpha(start)
                    } else {
                        stagger_alpha(row_idx, opened_ms)
                    };
                    row_idx += 1;
                    list = list.push(render_bus_row(msg, alpha));
                }
            }
            // High bucket
            if !high.is_empty() {
                list = list.push(
                    text(format!("! High  ({})", high.len()))
                        .size(10)
                        .color(BUS_HIGH_COLOR),
                );
                for msg in high.iter().take(20) {
                    let alpha = if let Some(&start) = state.dismissing.get(&msg.ulid) {
                        dismiss_alpha(start)
                    } else {
                        stagger_alpha(row_idx, opened_ms)
                    };
                    row_idx += 1;
                    list = list.push(render_bus_row(msg, alpha));
                }
            }
            // Default bucket
            if !default.is_empty() {
                list = list.push(
                    text(format!("Default  ({})", default.len()))
                        .size(10)
                        .color(BUS_DEFAULT_COLOR),
                );
                for msg in default.iter().take(20) {
                    let alpha = if let Some(&start) = state.dismissing.get(&msg.ulid) {
                        dismiss_alpha(start)
                    } else {
                        stagger_alpha(row_idx, opened_ms)
                    };
                    row_idx += 1;
                    list = list.push(render_bus_row(msg, alpha));
                }
            }
            // Empty state
            if bus_active_total == 0 && acked_msgs.is_empty() {
                list = list.push(
                    container(text("No bus messages").size(13).color(FG_MUTED))
                        .padding(Padding { top: 6.0, right: 0.0, bottom: 0.0, left: 0.0 }),
                );
            }
            // Acked-list (if any)
            if !acked_msgs.is_empty() {
                list = list.push(
                    text(format!("✓ Acked  ({})", acked_msgs.len()))
                        .size(10)
                        .color(FG_FAINT),
                );
                for msg in acked_msgs.iter().take(10) {
                    list = list.push(
                        container(
                            text(format!(
                                "✓  {}",
                                if msg.title.is_empty() { &msg.topic } else { &msg.title }
                            ))
                            .size(11)
                            .color(FG_FAINT),
                        )
                        .padding(Padding { top: 3.0, right: 8.0, bottom: 3.0, left: 8.0 }),
                    );
                }
            }
        }
    }

    if !state.muted_peers.is_empty() {
        let muted_list: Vec<&str> = state.muted_peers.iter().map(|s| s.as_str()).collect();
        list = list.push(
            container(
                text(format!("Muted: {}", muted_list.join(", ")))
                    .size(10)
                    .color(FG_FAINT),
            )
            .padding(Padding {
                top: 8.0,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            }),
        );
    }

    let scroll = scrollable(list).height(Length::Fill);

    // BUG-8.a — "Clear all" button (rendered only when
    // ≥1 notification exists). Click empties the cache
    // file + exits.
    let clear_btn: Element<'_, Message> = if total_rows > 0 {
        iced::widget::Button::new(text("Clear all").size(11).color(FG_TEXT))
            .padding(Padding {
                top: 3.0,
                right: 10.0,
                bottom: 3.0,
                left: 10.0,
            })
            .style(|_t: &Theme, status: iced::widget::button::Status| {
                let bg = match status {
                    iced::widget::button::Status::Hovered => Color {
                        r: 0.18,
                        g: 0.18,
                        b: 0.20,
                        a: 1.0,
                    },
                    _ => Color::TRANSPARENT,
                };
                iced::widget::button::Style {
                    background: Some(Background::Color(bg)),
                    text_color: FG_TEXT,
                    border: Border {
                        color: Color {
                            a: 0.15,
                            ..Color::WHITE
                        },
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    shadow: Shadow::default(),
                    snap: false,
                }
            })
            .on_press(Message::ClearAll)
            .into()
    } else {
        Space::new().into()
    };

    // BUG-8.c — group-mode toggle. Label reflects the
    // mode the click will switch TO (so "By app" means
    // currently grouped by peer; clicking flips to app).
    let mode_label = match state.group_mode {
        GroupMode::Peer => "By app",
        GroupMode::App => "By peer",
    };
    let mode_btn: Element<'_, Message> =
        iced::widget::Button::new(text(mode_label).size(11).color(FG_TEXT))
            .padding(Padding {
                top: 3.0,
                right: 10.0,
                bottom: 3.0,
                left: 10.0,
            })
            .style(|_t: &Theme, status: iced::widget::button::Status| {
                let bg = match status {
                    iced::widget::button::Status::Hovered => Color {
                        r: 0.18,
                        g: 0.18,
                        b: 0.20,
                        a: 1.0,
                    },
                    _ => Color::TRANSPARENT,
                };
                iced::widget::button::Style {
                    background: Some(Background::Color(bg)),
                    text_color: FG_TEXT,
                    border: Border {
                        color: Color {
                            a: 0.15,
                            ..Color::WHITE
                        },
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    shadow: Shadow::default(),
                    snap: false,
                }
            })
            .on_press(Message::ToggleGroupMode)
            .into();

    let body = column![
        row![
            header,
            Space::new().width(Length::Fill),
            subhead,
            Space::new().width(Length::Fixed(8.0)),
            mode_btn,
            Space::new().width(Length::Fixed(6.0)),
            clear_btn,
            Space::new().width(Length::Fixed(8.0)),
            // v3.0.3 — always-visible close button (Esc still
            // works via subscription below).
            crate::dismiss::close_button(Message::Exit),
        ]
        .align_y(iced::Alignment::Center),
        Space::new().height(Length::Fixed(8.0)),
        scroll,
        Space::new().height(Length::Fixed(4.0)),
        text("Esc closes · Clear all empties the cache")
            .size(10)
            .color(FG_MUTED),
    ]
    .padding(Padding {
        top: 14.0,
        right: 14.0,
        bottom: 8.0,
        left: 14.0,
    });

    let card: Element<'_, Message> = container(body)
        .width(Length::Fixed(WIDTH as f32))
        .height(Length::Fixed(HEIGHT as f32))
        .style(popover_surface)
        .into();

    // v3.0.4 — backdrop dismiss; bottom-right card.
    let dismiss = || {
        mouse_area(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(Message::Exit)
    };
    let bottom_strip = row![
        dismiss(),
        container(card).padding(Padding {
            top: 0.0,
            right: 4.0,
            bottom: 48.0,
            left: 0.0,
        }),
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
    // ANIM-3.b.2 — tick while entrance stagger or dismiss fades are active.
    let entrance_ms = state.opened_at.elapsed().as_millis() as u64;
    let animating = entrance_ms <= MAX_ENTRANCE_MS || !state.dismissing.is_empty();
    if animating {
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
            let muted_peers = load_muted_peers();
            let group_mode = GroupMode::Peer;
            let groups = load_groups_for(group_mode, &muted_peers);
            let bus_messages = bus_data_root()
                .map(|root| load_bus_messages(&root))
                .unwrap_or_default();
            tracing::info!(
                group_count = groups.len(),
                muted = muted_peers.len(),
                bus_messages = bus_messages.len(),
                "notifications popover open"
            );
            App {
                groups,
                muted_peers,
                group_mode,
                collapsed: std::collections::HashSet::new(),
                bus_messages,
                bus_acked: std::collections::HashSet::new(),
                opened_at: Instant::now(),
                dismissing: HashMap::new(),
            }
        },
        namespace,
        update,
        view,
    )
    .theme(|_: &App| iced::Theme::Dark)
    .subscription(subscription)
    .settings(Settings {
        id: Some("mde-popover-notifications".to_string()),
        fonts: crate::fonts::load_fallback_fonts(),
        layer_settings: LayerShellSettings {
            // v3.0.4 — fullscreen for backdrop dismiss.
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

/// BUS-2.3 / ANIM-3.b.2 — render one Bus message row with an "Ack"
/// button on the right. `alpha` = entrance stagger or dismiss-fade
/// value (0.0–1.0); all colors are scaled so the row fades in/out.
fn render_bus_row(msg: &BusPopoverMessage, alpha: f32) -> Element<'_, Message> {
    let priority_color = match msg.priority.as_str() {
        "urgent" => BUS_URGENT_COLOR,
        "high"   => BUS_HIGH_COLOR,
        _        => BUS_DEFAULT_COLOR,
    };
    let topic_label = text(format!("[{}]", msg.topic))
        .size(10)
        .color(Color { a: priority_color.a * alpha, ..priority_color });
    let title_label = text(if msg.title.is_empty() { msg.topic.as_str() } else { msg.title.as_str() })
        .size(12)
        .color(Color { a: FG_TEXT.a * alpha, ..FG_TEXT });
    let body_label = if msg.body.is_empty() {
        text("").size(11).color(Color { a: FG_MUTED.a * alpha, ..FG_MUTED })
    } else {
        text(msg.body.chars().take(100).collect::<String>())
            .size(11)
            .color(Color { a: FG_MUTED.a * alpha, ..FG_MUTED })
    };
    let ack_ulid = msg.ulid.clone();
    // Hide the Ack button while the row is fading out (alpha < full).
    let ack_btn: Element<'_, Message> = if alpha >= 0.99 {
        iced::widget::Button::new(
            text("Ack").size(10).color(FG_MUTED),
        )
        .padding(Padding { top: 2.0, right: 6.0, bottom: 2.0, left: 6.0 })
        .style(|_t: &Theme, status: iced::widget::button::Status| {
            let bg = match status {
                iced::widget::button::Status::Hovered => Color { r: 0.18, g: 0.18, b: 0.20, a: 1.0 },
                _ => Color::TRANSPARENT,
            };
            iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                text_color: FG_MUTED,
                border: Border {
                    color: Color { a: 0.12, ..Color::WHITE },
                    width: 1.0,
                    radius: 3.0.into(),
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .on_press(Message::AckBusMessage(ack_ulid))
        .into()
    } else {
        Space::new().into()
    };

    let text_col = column![topic_label, title_label, body_label].spacing(1);
    let content_row = row![
        text_col,
        Space::new().width(Length::Fill),
        ack_btn,
    ]
    .align_y(iced::Alignment::Center)
    .spacing(6);

    container(content_row)
        .padding(Padding { top: 5.0, right: 8.0, bottom: 5.0, left: 8.0 })
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(Color {
                r: 0.106,
                g: 0.106,
                b: 0.114,
                a: alpha,
            })),
            border: Border {
                color: Color {
                    r: ACCENT.r,
                    g: ACCENT.g,
                    b: ACCENT.b,
                    a: 0.05 * alpha,
                },
                width: 1.0,
                radius: 6.0.into(),
            },
            text_color: Some(Color { a: FG_TEXT.a * alpha, ..FG_TEXT }),
            shadow: Shadow::default(),
            snap: false,
        })
        .width(Length::Fill)
        .into()
}

/// ANIM-3.b.2 — `alpha` is the entrance stagger value (0.0→1.0).
fn render_row(row_data: &NotificationRow, alpha: f32) -> Element<'_, Message> {
    let title_prefix = if is_phone_origin(row_data) {
        "📱 ".to_string()
    } else if !row_data.read {
        "• ".to_string()
    } else {
        "  ".to_string()
    };
    let text_base = if row_data.read { FG_MUTED } else { FG_TEXT };
    let title = text(format!("{title_prefix}{}", row_data.title))
        .size(13)
        .color(Color { a: text_base.a * alpha, ..text_base });
    let body = if row_data.body.is_empty() {
        text("").size(11).color(Color { a: FG_MUTED.a * alpha, ..FG_MUTED })
    } else {
        text(row_data.body.chars().take(120).collect::<String>())
            .size(11)
            .color(Color { a: FG_MUTED.a * alpha, ..FG_MUTED })
    };
    container(column![title, body].spacing(2))
        .padding(Padding {
            top: 6.0,
            right: 10.0,
            bottom: 6.0,
            left: 10.0,
        })
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(Color {
                r: 0.106,
                g: 0.106,
                b: 0.114,
                a: alpha,
            })),
            border: Border {
                color: Color {
                    r: ACCENT.r,
                    g: ACCENT.g,
                    b: ACCENT.b,
                    a: 0.05 * alpha,
                },
                width: 1.0,
                radius: 6.0.into(),
            },
            text_color: Some(Color { a: FG_TEXT.a * alpha, ..FG_TEXT }),
            shadow: Shadow::default(),
            snap: false,
        })
        .width(Length::Fill)
        .into()
}

/// BUG-8.c — load + group rows for the current `GroupMode`,
/// filtering muted peers in peer-mode (mute is a peer concept;
/// app-mode users want the full firehose so they can still see
/// chatty apps even from muted peers).
fn load_groups_for(
    mode: GroupMode,
    muted_peers: &std::collections::HashSet<String>,
) -> Vec<(String, Vec<NotificationRow>)> {
    let path: PathBuf = notifications_cache_path();
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = parse_notifications(&raw);
    let visible_rows = visible(rows);
    match mode {
        GroupMode::Peer => group_and_sort(visible_rows)
            .into_iter()
            .filter(|(peer, _)| !muted_peers.contains(peer))
            .collect(),
        GroupMode::App => group_by_app(visible_rows),
    }
}

/// BUG-8.b — resolve the mute file path. Returns the canonical
/// `~/.config/mde/notification-mutes.toml`; falls back to
/// `$XDG_CONFIG_HOME/mde/...` if HOME isn't set.
fn mutes_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("mde").join("notification-mutes.toml"))
}

/// BUG-8.b — pure parser for the mute file. Returns the set of
/// peer names whose `[muted]."<peer>" = true` row is present.
#[must_use]
pub fn parse_mutes(raw: &str) -> std::collections::HashSet<String> {
    let value: toml::Value = match toml::from_str(raw) {
        Ok(v) => v,
        Err(_) => return Default::default(),
    };
    let mut out = std::collections::HashSet::new();
    if let Some(tbl) = value.get("muted").and_then(|v| v.as_table()) {
        for (peer, on) in tbl {
            if on.as_bool() == Some(true) {
                out.insert(peer.clone());
            }
        }
    }
    out
}

/// BUG-8.b — serialise the muted-peers set to TOML.
#[must_use]
pub fn serialize_mutes(muted: &std::collections::HashSet<String>) -> String {
    let mut peers: Vec<&String> = muted.iter().collect();
    peers.sort();
    let mut out = String::from("# mde-popover-notifications mute state — BUG-8.b\n");
    out.push_str("[muted]\n");
    for p in peers {
        let escaped = p.replace('\\', "\\\\").replace('"', "\\\"");
        out.push_str(&format!("\"{escaped}\" = true\n"));
    }
    out
}

fn load_muted_peers() -> std::collections::HashSet<String> {
    let Some(path) = mutes_path() else {
        return Default::default();
    };
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Default::default(),
    };
    parse_mutes(&raw)
}

fn save_muted_peers(
    muted: &std::collections::HashSet<String>,
) -> std::io::Result<()> {
    let Some(path) = mutes_path() else {
        return Err(std::io::Error::other("no $HOME"));
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&path, serialize_mutes(muted))
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_pinned_for_visual_consistency() {
        assert_eq!(WIDTH, 480);
        assert_eq!(HEIGHT, 600);
    }

    // ── ANIM-3.b.2 animation helper tests ─────────────────────────

    #[test]
    fn stagger_alpha_at_zero_ms_is_transparent() {
        // Row 0 has no delay; at t=0 the fade hasn't started yet.
        let a = stagger_alpha(0, 0);
        assert!(a < f32::EPSILON, "row 0 at t=0 should be fully transparent, got {a}");
    }

    #[test]
    fn stagger_alpha_after_full_reveal_is_opaque() {
        // Row 0 needs 0 ms delay + STAGGER_REVEAL_MS to become fully opaque.
        let a = stagger_alpha(0, STAGGER_REVEAL_MS as u64 + 1);
        assert!((a - 1.0).abs() < 0.01, "row 0 fully revealed should be ~1.0, got {a}");
    }

    #[test]
    fn stagger_alpha_row_1_has_delay() {
        // Row 1 has a delay of 1 * STAGGER_STEP_MS.
        let delay = STAGGER_STEP_MS as u64;
        // At exactly the delay point, elapsed = 0 → still transparent.
        let a = stagger_alpha(1, delay);
        assert!(a < f32::EPSILON, "row 1 at its delay onset should be transparent, got {a}");
        // After delay + full reveal, row 1 is opaque.
        let a2 = stagger_alpha(1, delay + STAGGER_REVEAL_MS as u64 + 1);
        assert!((a2 - 1.0).abs() < 0.01, "row 1 fully revealed, got {a2}");
    }

    #[test]
    fn stagger_alpha_caps_at_stagger_cap() {
        // Row STAGGER_CAP and row STAGGER_CAP*2 must give identical alpha
        // (cap clamps the delay).
        let opened_ms = 500;
        let a_cap = stagger_alpha(STAGGER_CAP, opened_ms);
        let a_double = stagger_alpha(STAGGER_CAP * 2, opened_ms);
        assert!(
            (a_cap - a_double).abs() < f32::EPSILON,
            "rows beyond cap should have identical alpha"
        );
    }

    #[test]
    fn max_entrance_ms_matches_token_math() {
        // Verify the constant matches the hand-computed value.
        let expected =
            (STAGGER_CAP as u64 - 1) * STAGGER_STEP_MS as u64 + STAGGER_REVEAL_MS as u64;
        assert_eq!(MAX_ENTRANCE_MS, expected);
    }

    #[test]
    fn dismiss_alpha_starts_near_one() {
        // A freshly-started dismiss should be close to 1.0.
        let start = Instant::now();
        let a = dismiss_alpha(start);
        assert!(a > 0.90, "fresh dismiss should be nearly opaque, got {a}");
    }

    #[test]
    fn dismiss_alpha_clamps_to_zero_after_duration() {
        use std::time::Duration;
        // Simulate a start time well in the past (> DISMISS_ANIM_MS ago).
        let past = Instant::now() - Duration::from_millis(DISMISS_ANIM_MS + 50);
        let a = dismiss_alpha(past);
        assert!(a < f32::EPSILON, "completed dismiss should be 0.0, got {a}");
    }


    #[test]
    fn parse_mutes_decodes_known_shape() {
        let raw = r#"
            [muted]
            "pine.mesh" = true
            "birch.mesh" = true
            "oak.mesh" = false
        "#;
        let muted = parse_mutes(raw);
        assert_eq!(muted.len(), 2);
        assert!(muted.contains("pine.mesh"));
        assert!(muted.contains("birch.mesh"));
        assert!(!muted.contains("oak.mesh"));
    }

    #[test]
    fn parse_mutes_returns_empty_for_garbage() {
        assert!(parse_mutes("not toml").is_empty());
    }

    #[test]
    fn serialize_mutes_round_trips_through_parse() {
        let mut m: std::collections::HashSet<String> = Default::default();
        m.insert("pine.mesh".into());
        m.insert("birch.mesh".into());
        let raw = serialize_mutes(&m);
        let back = parse_mutes(&raw);
        assert_eq!(back, m);
    }

    #[test]
    fn serialize_mutes_handles_peers_with_quotes_in_name() {
        let mut m: std::collections::HashSet<String> = Default::default();
        m.insert(r#"odd"name"#.to_string());
        let raw = serialize_mutes(&m);
        let back = parse_mutes(&raw);
        assert_eq!(back, m);
    }

    // ── BUS-2.3 tests ─────────────────────────────────────────────

    #[test]
    fn parse_bus_message_decodes_high_priority_file() {
        let dir = tempfile::tempdir().unwrap();
        let ulid = "01JABCDEFGHJKMNPQRST";
        let json = r#"{"ulid":"01JABCDEFGHJKMNPQRST","topic":"fleet/announce","priority":"high","title":"Test title","body":"Test body","ts_unix_ms":1700000000000,"file_path":"fleet/announce/01JABCDEFGHJKMNPQRST.json"}"#;
        let path = dir.path().join(format!("{ulid}.json"));
        std::fs::write(&path, json).unwrap();
        let msg = parse_bus_message(&path, ulid, "fleet/announce").unwrap();
        assert_eq!(msg.ulid, ulid);
        assert_eq!(msg.priority, "high");
        assert_eq!(msg.title, "Test title");
        assert_eq!(msg.body, "Test body");
        assert_eq!(msg.topic, "fleet/announce");
    }

    #[test]
    fn parse_bus_message_filters_min_priority() {
        let dir = tempfile::tempdir().unwrap();
        let ulid = "01JBBBBBBBBBBBBBBBBB";
        let json = r#"{"ulid":"01JBBBBBBBBBBBBBBBBB","topic":"debug/info","priority":"min","title":"x","body":"y","ts_unix_ms":0,"file_path":""}"#;
        let path = dir.path().join(format!("{ulid}.json"));
        std::fs::write(&path, json).unwrap();
        assert!(parse_bus_message(&path, ulid, "debug/info").is_none());
    }

    #[test]
    fn parse_bus_message_returns_none_for_malformed_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("01JCCC.json");
        std::fs::write(&path, "not json at all").unwrap();
        assert!(parse_bus_message(&path, "01JCCC", "fleet/announce").is_none());
    }

    fn make_msg(ulid: &str, priority: &str) -> BusPopoverMessage {
        BusPopoverMessage {
            ulid: ulid.to_string(),
            topic: "fleet/announce".to_string(),
            priority: priority.to_string(),
            title: format!("{priority} message"),
            body: "body".to_string(),
        }
    }

    #[test]
    fn bucket_by_priority_groups_three_buckets() {
        let msgs = vec![
            make_msg("01ZAAA", "urgent"),
            make_msg("01ZBBB", "high"),
            make_msg("01ZCCC", "default"),
        ];
        let acked = std::collections::HashSet::new();
        let (urgent, high, default) = bucket_by_priority(&msgs, &acked);
        assert_eq!(urgent.len(), 1);
        assert_eq!(high.len(), 1);
        assert_eq!(default.len(), 1);
        assert_eq!(urgent[0].ulid, "01ZAAA");
        assert_eq!(high[0].ulid, "01ZBBB");
        assert_eq!(default[0].ulid, "01ZCCC");
    }

    #[test]
    fn bucket_by_priority_excludes_acked_ulids() {
        let msgs = vec![
            make_msg("01ZAAA", "urgent"),
            make_msg("01ZBBB", "high"),
        ];
        let mut acked = std::collections::HashSet::new();
        acked.insert("01ZAAA".to_string());
        let (urgent, high, _) = bucket_by_priority(&msgs, &acked);
        assert!(urgent.is_empty(), "acked urgent must be excluded");
        assert_eq!(high.len(), 1);
    }

    #[test]
    fn bucket_by_priority_newest_first_within_bucket() {
        let msgs = vec![
            make_msg("01ZAAA", "high"),  // older ULID
            make_msg("01ZZZZ", "high"),  // newer ULID
        ];
        let acked = std::collections::HashSet::new();
        let (_, high, _) = bucket_by_priority(&msgs, &acked);
        assert_eq!(high[0].ulid, "01ZZZZ", "newer ULID must sort first");
    }

    #[test]
    fn load_bus_messages_reads_nested_topic_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let topic_dir = dir.path().join("fleet").join("announce");
        std::fs::create_dir_all(&topic_dir).unwrap();
        let ulid = "01JABCDEFGHJKMNPQRST";
        let json = format!(r#"{{"ulid":"{ulid}","topic":"fleet/announce","priority":"default","title":"Mesh event","body":"peer joined","ts_unix_ms":0,"file_path":""}}"#);
        std::fs::write(topic_dir.join(format!("{ulid}.json")), &json).unwrap();
        // audit/ files must be skipped
        let audit_dir = dir.path().join("audit");
        std::fs::create_dir_all(&audit_dir).unwrap();
        std::fs::write(audit_dir.join("2026-05-29.jsonl"), "skip me\n").unwrap();

        let msgs = load_bus_messages(dir.path());
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].ulid, ulid);
        assert_eq!(msgs[0].topic, "fleet/announce");
    }
}
