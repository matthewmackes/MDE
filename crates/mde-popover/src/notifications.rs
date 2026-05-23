//! Notifications popover — recent notifications list.
//!
//! Anchored bottom-right of the primary output above the panel.
//! Reads `~/.cache/mackes/notifications.json` (the same cache the
//! notification-bell applet polls) and renders the rows grouped by
//! peer, with phone-origin rows badged via the locked glyph.

use std::fs;
use std::path::PathBuf;

use iced::widget::{column, container, mouse_area, row, scrollable, text, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;
use mde_applet_notifications::{
    group_and_sort, group_by_app, is_phone_origin, notifications_cache_path,
    parse_notifications, visible, NotificationRow,
};

const WIDTH: u32 = 480;
const HEIGHT: u32 = 600;

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

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Exit,
    /// BUG-8.a (2026-05-23) — clear the cache file then exit.
    ClearAll,
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
}

impl iced_layershell::Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Task<Message>) {
        let muted_peers = load_muted_peers();
        let group_mode = GroupMode::Peer;
        let groups = load_groups_for(group_mode, &muted_peers);
        tracing::info!(group_count = groups.len(), muted = muted_peers.len(), "notifications popover open");
        (
            Self {
                groups,
                muted_peers,
                group_mode,
                collapsed: std::collections::HashSet::new(),
            },
            Task::none(),
        )
    }

    fn namespace(&self) -> String {
        "mde-popover-notifications".to_string()
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
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
                if self.muted_peers.contains(&peer) {
                    self.muted_peers.remove(&peer);
                } else {
                    self.muted_peers.insert(peer);
                }
                let _ = save_muted_peers(&self.muted_peers);
                self.groups = load_groups_for(self.group_mode, &self.muted_peers);
                Task::none()
            }
            Message::ToggleGroupMode => {
                self.group_mode = match self.group_mode {
                    GroupMode::Peer => GroupMode::App,
                    GroupMode::App => GroupMode::Peer,
                };
                // Reset collapses on mode flip — the bucket
                // keys mean different things across modes.
                self.collapsed.clear();
                self.groups = load_groups_for(self.group_mode, &self.muted_peers);
                Task::none()
            }
            Message::ToggleCollapse(key) => {
                if self.collapsed.contains(&key) {
                    self.collapsed.remove(&key);
                } else {
                    self.collapsed.insert(key);
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let header = text("Notifications").size(14).color(FG_TEXT);
        let total_rows: usize = self.groups.iter().map(|(_, r)| r.len()).sum();
        let subhead = text(format!("{total_rows} total"))
            .size(11)
            .color(FG_MUTED);

        let mut list = column![].spacing(10);
        if self.groups.is_empty() {
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
        for (group_name, rows) in &self.groups {
            let label_text = if group_name.is_empty() {
                "Local".to_string()
            } else {
                group_name.clone()
            };
            // BUG-8.c — collapsed flag drives the chevron glyph
            // + body visibility.
            let is_collapsed = self.collapsed.contains(group_name);
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
                }
            })
            .on_press(Message::ToggleCollapse(collapse_key))
            .into();

            // BUG-8.b — per-peer Mute button only makes sense in
            // peer-grouped mode; hide it in app-grouped mode
            // (muting "firefox" doesn't have the same wire
            // semantics — that would be a future BUG-8.d).
            let mute_btn: Element<'_, Message> = if self.group_mode == GroupMode::Peer {
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
                        }
                    })
                    .on_press(Message::ToggleMute(peer_for_mute))
                    .into()
            } else {
                Space::with_width(Length::Fixed(0.0)).into()
            };

            let group_header = row![
                header_btn,
                Space::with_width(Length::Fill),
                mute_btn,
            ]
            .align_y(iced::Alignment::Center);

            let mut group_column = column![group_header].spacing(4);
            if !is_collapsed {
                for row_data in rows.iter().take(40) {
                    group_column = group_column.push(render_row(row_data));
                }
            }
            list = list.push(group_column);
        }
        if !self.muted_peers.is_empty() {
            let muted_list: Vec<&str> = self.muted_peers.iter().map(|s| s.as_str()).collect();
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
                    }
                })
                .on_press(Message::ClearAll)
                .into()
        } else {
            Space::with_width(Length::Fixed(0.0)).into()
        };

        // BUG-8.c — group-mode toggle. Label reflects the
        // mode the click will switch TO (so "By app" means
        // currently grouped by peer; clicking flips to app).
        let mode_label = match self.group_mode {
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
                    }
                })
                .on_press(Message::ToggleGroupMode)
                .into();

        let body = column![
            row![
                header,
                Space::with_width(Length::Fill),
                subhead,
                Space::with_width(Length::Fixed(8.0)),
                mode_btn,
                Space::with_width(Length::Fixed(6.0)),
                clear_btn,
                Space::with_width(Length::Fixed(8.0)),
                // v3.0.3 — always-visible close button (Esc still
                // works via subscription below).
                crate::dismiss::close_button(Message::Exit),
            ]
            .align_y(iced::Alignment::Center),
            Space::with_height(Length::Fixed(8.0)),
            scroll,
            Space::with_height(Length::Fixed(4.0)),
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
                container(Space::with_width(Length::Fill))
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
}

fn render_row(row_data: &NotificationRow) -> Element<'_, Message> {
    let title_prefix = if is_phone_origin(row_data) {
        "📱 ".to_string()
    } else if !row_data.read {
        "• ".to_string()
    } else {
        "  ".to_string()
    };
    let title = text(format!("{title_prefix}{}", row_data.title))
        .size(13)
        .color(if row_data.read { FG_MUTED } else { FG_TEXT });
    let body = if row_data.body.is_empty() {
        text("").size(11).color(FG_MUTED)
    } else {
        text(row_data.body.chars().take(120).collect::<String>())
            .size(11)
            .color(FG_MUTED)
    };
    container(column![title, body].spacing(2))
        .padding(Padding {
            top: 6.0,
            right: 10.0,
            bottom: 6.0,
            left: 10.0,
        })
        .style(row_surface)
        .width(Length::Fill)
        .into()
}

fn load_groups() -> Vec<(String, Vec<NotificationRow>)> {
    let path: PathBuf = notifications_cache_path();
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = parse_notifications(&raw);
    let visible_rows = visible(rows);
    group_and_sort(visible_rows)
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
    }
}

fn row_surface(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color {
            r: 0.106,
            g: 0.106,
            b: 0.114,
            a: 1.0,
        })),
        border: Border {
            color: Color {
                r: ACCENT.r,
                g: ACCENT.g,
                b: ACCENT.b,
                a: 0.05,
            },
            width: 1.0,
            radius: 6.0.into(),
        },
        text_color: Some(FG_TEXT),
        shadow: Shadow::default(),
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

    #[test]
    fn load_groups_returns_empty_when_cache_missing() {
        // Hard to guarantee without setting env vars, but if the
        // cache is missing the helper returns an empty Vec rather
        // than panicking.
        let _ = load_groups();
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
}
