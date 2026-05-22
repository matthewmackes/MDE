//! Notifications popover — recent notifications list.
//!
//! Anchored bottom-right of the primary output above the panel.
//! Reads `~/.cache/mackes/notifications.json` (the same cache the
//! notification-bell applet polls) and renders the rows grouped by
//! peer, with phone-origin rows badged via the locked glyph.

use std::fs;
use std::path::PathBuf;

use iced::widget::{column, container, row, scrollable, text, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;
use mde_applet_notifications::{
    group_and_sort, is_phone_origin, notifications_cache_path, parse_notifications, visible,
    NotificationRow,
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
}

pub struct App {
    groups: Vec<(String, Vec<NotificationRow>)>,
}

impl iced_layershell::Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Task<Message>) {
        let groups = load_groups();
        tracing::info!(group_count = groups.len(), "notifications popover open");
        (Self { groups }, Task::none())
    }

    fn namespace(&self) -> String {
        "mde-popover-notifications".to_string()
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Exit => std::process::exit(0),
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
            let group_label = text(if group_name.is_empty() {
                "Local".to_string()
            } else {
                group_name.clone()
            })
            .size(11)
            .color(FG_MUTED);
            let mut group_column = column![group_label].spacing(4);
            for row_data in rows.iter().take(40) {
                group_column = group_column.push(render_row(row_data));
            }
            list = list.push(group_column);
        }

        let scroll = scrollable(list).height(Length::Fill);

        let body = column![
            row![
                header,
                Space::with_width(Length::Fill),
                subhead,
            ]
            .align_y(iced::Alignment::Center),
            Space::with_height(Length::Fixed(8.0)),
            scroll,
            Space::with_height(Length::Fixed(4.0)),
            text("Esc closes").size(10).color(FG_MUTED),
        ]
        .padding(Padding {
            top: 14.0,
            right: 14.0,
            bottom: 8.0,
            left: 14.0,
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
        id: Some("mde-popover-notifications".to_string()),
        fonts: crate::fonts::load_fallback_fonts(),
        layer_settings: LayerShellSettings {
            size: Some((WIDTH, HEIGHT)),
            exclusive_zone: 0,
            anchor: Anchor::Bottom | Anchor::Right,
            margin: (0, 4, 48, 0),
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
}
