//! Clock popover — month-grid calendar.
//!
//! Anchored bottom-right above the panel clock. Shows the current
//! month as a 7×6 grid with the current day highlighted. No event
//! integration — pure display for v3.1.

use std::time::{SystemTime, UNIX_EPOCH};

use iced::widget::{column, container, row, text, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

const WIDTH: u32 = 300;
const HEIGHT: u32 = 340;

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
    today: (i32, u32, u32),
    month_grid: Vec<Vec<Option<u32>>>, // 6 rows × 7 cols, Some(day) or None for padding
    month_name: String,
    weekday_today: u32,
    hms: (u32, u32),
}

impl iced_layershell::Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Task<Message>) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64);
        let today = days_to_ymd(now / 86_400);
        let secs_in_day = (now % 86_400) as u32;
        let hms = (secs_in_day / 3600, (secs_in_day % 3600) / 60);
        let month_name = MONTH_NAMES[(today.1 - 1) as usize].to_string();
        let weekday_today = weekday_of(today.0, today.1, today.2);
        let month_grid = build_month_grid(today.0, today.1);
        tracing::info!(date = ?today, "clock popover open");
        (
            Self {
                today,
                month_grid,
                month_name,
                weekday_today,
                hms,
            },
            Task::none(),
        )
    }

    fn namespace(&self) -> String {
        "mde-popover-clock".to_string()
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Exit => std::process::exit(0),
            _ => Task::none(),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let (y, _, d) = self.today;
        let hour = self.hms.0;
        let minute = self.hms.1;
        let weekday_name = WEEKDAY_NAMES[self.weekday_today as usize];

        let big_time = text(format!("{hour:02}:{minute:02}"))
            .size(40)
            .color(FG_TEXT);
        let date_line = text(format!("{weekday_name}, {} {d}, {y}", self.month_name))
            .size(13)
            .color(FG_MUTED);

        // Weekday headers — Sun..Sat
        let mut header_row = row![]
            .spacing(4)
            .align_y(iced::Alignment::Center);
        for wd in WEEKDAY_INITIALS {
            header_row = header_row.push(
                container(text(*wd).size(10).color(FG_MUTED))
                    .width(Length::Fixed(32.0))
                    .center_x(Length::Fixed(32.0)),
            );
        }

        // Grid rows
        let mut grid = column![header_row].spacing(2);
        for row_cells in &self.month_grid {
            let mut grid_row = row![].spacing(4);
            for cell in row_cells {
                let cell_widget: Element<'_, Message> = match cell {
                    Some(day) => {
                        let is_today = *day == self.today.2;
                        let day_text = text(day.to_string()).size(12).color(if is_today {
                            FG_TEXT
                        } else {
                            FG_MUTED
                        });
                        container(day_text)
                            .width(Length::Fixed(32.0))
                            .height(Length::Fixed(28.0))
                            .center_x(Length::Fixed(32.0))
                            .center_y(Length::Fixed(28.0))
                            .style(if is_today {
                                today_cell_style
                            } else {
                                empty_cell_style
                            })
                            .into()
                    }
                    None => container(text(" "))
                        .width(Length::Fixed(32.0))
                        .height(Length::Fixed(28.0))
                        .into(),
                };
                grid_row = grid_row.push(cell_widget);
            }
            grid = grid.push(grid_row);
        }

        let body = column![
            big_time,
            Space::with_height(Length::Fixed(2.0)),
            date_line,
            Space::with_height(Length::Fixed(14.0)),
            grid,
            Space::with_height(Length::Fill),
            text("Esc closes").size(10).color(FG_MUTED),
        ]
        .padding(Padding {
            top: 16.0,
            right: 18.0,
            bottom: 10.0,
            left: 18.0,
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
        id: Some("mde-popover-clock".to_string()),
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

const MONTH_NAMES: &[&str] = &[
    "January", "February", "March", "April", "May", "June", "July", "August", "September",
    "October", "November", "December",
];

const WEEKDAY_INITIALS: &[&str] = &["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const WEEKDAY_NAMES: &[&str] = &[
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// Howard Hinnant civil-from-days. Same algorithm the clock applet
/// ships in its lib; copied here so the popover stays a thin
/// dependency.
fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if mo <= 2 { y + 1 } else { y };
    (year as i32, mo as u32, d as u32)
}

/// Convert Y/M/D to a days-from-epoch (Howard Hinnant). Used to
/// compute weekday + month-grid alignment.
fn ymd_to_days(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if m > 2 { m - 3 } else { m + 9 } as u64;
    let doy = (153 * mp + 2) / 5 + (d as u64) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

/// 0..6 — Sunday = 0.
fn weekday_of(y: i32, m: u32, d: u32) -> u32 {
    let days = ymd_to_days(y, m, d);
    // Unix epoch 1970-01-01 was Thursday = 4.
    ((days.rem_euclid(7) + 4) % 7) as u32
}

/// Number of days in a calendar month, leap-year aware.
fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Build a 6×7 grid of `Option<day>` for the given year+month.
/// Leading `None`s pad to the start weekday; trailing `None`s pad
/// to a fixed 6-row height so the grid is visually stable.
fn build_month_grid(y: i32, m: u32) -> Vec<Vec<Option<u32>>> {
    let start_weekday = weekday_of(y, m, 1) as usize;
    let total_days = days_in_month(y, m);
    let mut cells: Vec<Option<u32>> = vec![None; start_weekday];
    for d in 1..=total_days {
        cells.push(Some(d));
    }
    while cells.len() < 42 {
        cells.push(None);
    }
    cells.chunks(7).map(|c| c.to_vec()).collect()
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

fn today_cell_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color {
            r: ACCENT.r,
            g: ACCENT.g,
            b: ACCENT.b,
            a: 0.20,
        })),
        border: Border {
            color: ACCENT,
            width: 1.0,
            radius: 14.0.into(),
        },
        text_color: Some(FG_TEXT),
        shadow: Shadow::default(),
    }
}

fn empty_cell_style(_theme: &Theme) -> container::Style {
    container::Style::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_pinned_for_visual_consistency() {
        assert_eq!(WIDTH, 300);
        assert_eq!(HEIGHT, 340);
    }

    #[test]
    fn days_in_february_handles_leap_year() {
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2025, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(1900, 2), 28);
    }

    #[test]
    fn weekday_of_known_dates() {
        // 1970-01-01 was a Thursday (= 4).
        assert_eq!(weekday_of(1970, 1, 1), 4);
        // 2026-05-22 is a Friday (= 5) per CLAUDE.md currentDate context.
        assert_eq!(weekday_of(2026, 5, 22), 5);
    }

    #[test]
    fn build_month_grid_has_6_rows_of_7() {
        let g = build_month_grid(2026, 5);
        assert_eq!(g.len(), 6);
        for row in &g {
            assert_eq!(row.len(), 7);
        }
    }

    #[test]
    fn build_month_grid_aligns_start_weekday() {
        // May 2026 — May 1 is a Friday (= 5). So the first 5 cells
        // of row 0 should be None, then May 1 in cell 5.
        let g = build_month_grid(2026, 5);
        for i in 0..5 {
            assert!(g[0][i].is_none(), "expected None at index {i}");
        }
        assert_eq!(g[0][5], Some(1));
    }

    #[test]
    fn ymd_round_trips_through_days() {
        for ymd in [(2026, 5, 22), (1970, 1, 1), (2000, 2, 29)] {
            let days = ymd_to_days(ymd.0, ymd.1, ymd.2);
            assert_eq!(days_to_ymd(days), ymd);
        }
    }
}
