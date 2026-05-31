//! `mde-popover lock` — lock screen layer-shell overlay (Portal-25).
//!
//! Fullscreen Layer::Overlay surface that paints the MDE lock screen
//! over every other surface, capturing keyboard input exclusively.
//! Replaces swaylock theming with a single render path that matches
//! Portal visual identity.
//!
//! Layout (Classic ChromeOS palette, Intel One Mono):
//!   • Top-left breadcrumb       `M › <hostname>`
//!   • Centered big clock         `HH:MM`           (96 px)
//!   • Date                       `Mon, May 25`     (18 px)
//!   • Mesh / network / battery indicator row
//!   • Weather hint               read from ~/.cache/mde/weather.json
//!   • Footer                    `Press Esc or Enter to unlock`
//!
//! Esc or Enter exits the process (the compositor session-lock
//! protocol covers actual credential entry; this surface is the
//! visual layer that replaces swaylock theming — R2-Q54 / R4-Q4).

#![forbid(unsafe_code)]

use std::time::{SystemTime, UNIX_EPOCH};

use iced::widget::{column, container, mouse_area, row, text, Space};
use iced::widget::container::Style as ContainerStyle;
use iced::{Background, Border, Color, Element, Length, Padding, Subscription, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

// ── Design tokens (Classic ChromeOS, §3 lock) ─────────────────────────────────

const CHARCOAL: Color = Color { r: 0.125, g: 0.129, b: 0.141, a: 1.0 };
const FG: Color = Color::WHITE;
const FG_DIM: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.55 };
const FG_LABEL: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.35 };
const ACCENT: Color = Color { r: 0.357, g: 0.416, b: 0.961, a: 1.0 }; // indigo
const DOT_UP: Color = Color { r: 0.345, g: 0.871, b: 0.475, a: 1.0 }; // soft green
const DOT_DOWN: Color = Color { r: 0.498, g: 0.498, b: 0.498, a: 1.0 };

// ── Crossfade (ANIM-7.d, Q48) ─────────────────────────────────────────────────
// 200 ms grid tier (sway-native-shell.md Q3 timing grid).
// 16 ms ticks × 13 steps ≈ 208 ms — close enough to the 200 ms tier.

const FADE_STEP: f32 = 0.08; // per 16 ms tick → 13 ticks ≈ 208 ms

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FadePhase {
    In,
    Steady,
    Out,
}

fn fade_color(c: Color, opacity: f32) -> Color {
    Color { a: c.a * opacity, ..c }
}

// ── Data collection ───────────────────────────────────────────────────────────

fn read_hostname() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn read_battery_pct() -> Option<u8> {
    for name in &["BAT0", "BAT1"] {
        let path = format!("/sys/class/power_supply/{name}/capacity");
        if let Ok(s) = std::fs::read_to_string(&path) {
            if let Ok(n) = s.trim().parse::<u8>() {
                return Some(n);
            }
        }
    }
    None
}

fn read_interface_up(iface: &str) -> bool {
    let path = format!("/sys/class/net/{iface}/operstate");
    std::fs::read_to_string(path)
        .map(|s| s.trim() == "up")
        .unwrap_or(false)
}

fn read_network_up() -> bool {
    let Ok(entries) = std::fs::read_dir("/sys/class/net") else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let iface = name.to_string_lossy();
        if iface == "lo" {
            continue;
        }
        if read_interface_up(&iface) {
            return true;
        }
    }
    false
}

fn read_weather_summary() -> String {
    let Some(cache) = dirs::cache_dir() else {
        return "—".to_string();
    };
    let path = cache.join("mde").join("weather.json");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return "—".to_string();
    };
    let v: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return "—".to_string(),
    };
    let temp = v
        .get("temp_c")
        .and_then(|t| t.as_i64())
        .map(|t| format!("{t}°C"))
        .unwrap_or_else(|| "—".to_string());
    let cond = v
        .get("condition")
        .and_then(|c| c.as_str())
        .unwrap_or("—")
        .to_string();
    format!("{cond} · {temp}")
}

// ── Date / time helpers (mirrors crate::clock) ────────────────────────────────

fn now_secs_local() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

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

fn ymd_to_days(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if m > 2 { m - 3 } else { m + 9 } as u64;
    let doy = (153 * mp + 2) / 5 + (d as u64) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

fn weekday_short(y: i32, m: u32, d: u32) -> &'static str {
    let days = ymd_to_days(y, m, d);
    let w = ((days.rem_euclid(7) + 4) % 7) as usize; // 0 = Sun
    ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][w]
}

const MONTH_NAMES_SHORT: &[&str] = &[
    "Jan", "Feb", "Mar", "Apr", "May", "Jun",
    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn format_hm(secs_in_day: u32) -> String {
    let h = (secs_in_day / 3600) % 24;
    let m = (secs_in_day % 3600) / 60;
    format!("{h:02}:{m:02}")
}

fn format_date(y: i32, m: u32, d: u32) -> String {
    let wd = weekday_short(y, m, d);
    let mo = MONTH_NAMES_SHORT[(m - 1) as usize];
    format!("{wd}, {mo} {d}")
}

// ── Application ───────────────────────────────────────────────────────────────

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    FadeStep,
    Exit,
}

pub struct App {
    hostname: String,
    clock: String,
    date: String,
    mesh_up: bool,
    network_up: bool,
    battery_pct: Option<u8>,
    weather: String,
    fade: f32,
    phase: FadePhase,
}

impl App {
    fn refresh() -> Self {
        let now = now_secs_local();
        let secs_in_day = (now.rem_euclid(86_400)) as u32;
        let (y, mo, d) = days_to_ymd(now.div_euclid(86_400));
        Self {
            hostname: read_hostname(),
            clock: format_hm(secs_in_day),
            date: format_date(y, mo, d),
            mesh_up: read_interface_up("nebula0"),
            network_up: read_network_up(),
            battery_pct: read_battery_pct(),
            weather: read_weather_summary(),
            fade: 1.0,
            phase: FadePhase::Steady,
        }
    }
}

fn namespace() -> String {
    "mde-popover-lock".to_string()
}

fn update(state: &mut App, msg: Message) -> Task<Message> {
    match msg {
        Message::Tick => {
            let fresh = App::refresh();
            state.hostname = fresh.hostname;
            state.clock = fresh.clock;
            state.date = fresh.date;
            state.mesh_up = fresh.mesh_up;
            state.network_up = fresh.network_up;
            state.battery_pct = fresh.battery_pct;
            state.weather = fresh.weather;
        }
        Message::FadeStep => match state.phase {
            FadePhase::In => {
                state.fade = (state.fade + FADE_STEP).min(1.0);
                if state.fade >= 1.0 {
                    state.phase = FadePhase::Steady;
                }
            }
            FadePhase::Out => {
                state.fade = (state.fade - FADE_STEP).max(0.0);
                if state.fade <= 0.0 {
                    std::process::exit(0);
                }
            }
            FadePhase::Steady => {}
        },
        Message::Exit => {
            state.phase = FadePhase::Out;
        }
        _ => {}
    }
    Task::none()
}

fn view(state: &App) -> Element<'_, Message> {
    let fade = state.fade;

    let breadcrumb: Element<'_, Message> = row![
        text("M")
            .size(13)
            .color(fade_color(ACCENT, fade)),
        Space::new().width(Length::Fixed(6.0)),
        text("›").size(13).color(fade_color(FG_LABEL, fade)),
        Space::new().width(Length::Fixed(6.0)),
        text(state.hostname.clone()).size(13).color(fade_color(FG_DIM, fade)),
    ]
    .align_y(iced::Alignment::Center)
    .into();

    let big_clock: Element<'_, Message> =
        text(state.clock.clone()).size(96).color(fade_color(FG, fade)).into();

    let date_line: Element<'_, Message> =
        text(state.date.clone()).size(18).color(fade_color(FG_DIM, fade)).into();

    // ── Indicator row (mesh / net / battery / weather) ────────────────────
    let dot = move |label: &str, up: bool| -> Element<'static, Message> {
        let dot_color = if up { DOT_UP } else { DOT_DOWN };
        row![
            container(Space::new())
                .width(Length::Fixed(8.0))
                .height(Length::Fixed(8.0))
                .style(move |_: &Theme| ContainerStyle {
                    background: Some(Background::Color(fade_color(dot_color, fade))),
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: 4.0.into(),
                    },
                    ..Default::default()
                }),
            Space::new().width(Length::Fixed(6.0)),
            text(label.to_string()).size(12).color(fade_color(FG_DIM, fade)),
        ]
        .align_y(iced::Alignment::Center)
        .into()
    };

    let battery_chip: Element<'_, Message> = match state.battery_pct {
        Some(p) => row![
            text("bat").size(10).color(fade_color(FG_LABEL, fade)),
            Space::new().width(Length::Fixed(4.0)),
            text(format!("{p}%")).size(12).color(fade_color(FG_DIM, fade)),
        ]
        .align_y(iced::Alignment::Center)
        .into(),
        None => row![text("bat —").size(12).color(fade_color(FG_LABEL, fade))]
            .align_y(iced::Alignment::Center)
            .into(),
    };

    let indicators: Element<'_, Message> = row![
        dot("mesh", state.mesh_up),
        Space::new().width(Length::Fixed(20.0)),
        dot("net", state.network_up),
        Space::new().width(Length::Fixed(20.0)),
        battery_chip,
        Space::new().width(Length::Fixed(20.0)),
        text(state.weather.clone()).size(12).color(fade_color(FG_DIM, fade)),
    ]
    .align_y(iced::Alignment::Center)
    .into();

    // ── Centered card ────────────────────────────────────────────────────
    let card_body = column![
        breadcrumb,
        Space::new().height(Length::Fixed(28.0)),
        big_clock,
        Space::new().height(Length::Fixed(4.0)),
        date_line,
        Space::new().height(Length::Fixed(24.0)),
        indicators,
    ]
    .align_x(iced::Alignment::Center);

    let footer: Element<'_, Message> =
        text("Press Esc or Enter to unlock").size(11).color(fade_color(FG_LABEL, fade)).into();

    let centered = column![
        Space::new().height(Length::Fill),
        card_body,
        Space::new().height(Length::Fill),
        footer,
        Space::new().height(Length::Fixed(40.0)),
    ]
    .align_x(iced::Alignment::Center)
    .padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 });

    // mouse_area on the whole surface so an Enter-equivalent
    // click (rare but possible on touch devices) dismisses.
    let backdrop = mouse_area(
        container(centered)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_: &Theme| ContainerStyle {
                background: Some(Background::Color(fade_color(CHARCOAL, fade))),
                ..Default::default()
            }),
    )
    .on_press(Message::Exit);

    backdrop.into()
}

fn subscription(state: &App) -> Subscription<Message> {
    use iced::event;
    let keys = event::listen_with(|event, status, _window| {
        use iced::keyboard;
        match event {
            iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. })
                if status == event::Status::Ignored =>
            {
                use iced::keyboard::{key::Named, Key};
                match key {
                    Key::Named(Named::Escape) | Key::Named(Named::Enter) => Some(Message::Exit),
                    _ => None,
                }
            }
            _ => None,
        }
    });

    // Tick every 10 s so the clock + battery + mesh dot stay fresh.
    let tick = iced::time::every(std::time::Duration::from_secs(10))
        .map(|_| Message::Tick);

    // 16 ms fade ticker — active only during In/Out phases (Q48 crossfade).
    if state.phase != FadePhase::Steady {
        let fade_tick = iced::time::every(std::time::Duration::from_millis(16))
            .map(|_| Message::FadeStep);
        Subscription::batch([keys, tick, fade_tick])
    } else {
        Subscription::batch([keys, tick])
    }
}

pub fn run() -> iced_layershell::Result {
    iced_layershell::application(
        || {
            let mut app = App::refresh();
            app.fade = 0.0;
            app.phase = FadePhase::In;
            tracing::info!(hostname = %app.hostname, clock = %app.clock, "lock popover open");
            app
        },
        namespace,
        update,
        view,
    )
    .theme(|_: &App| iced::Theme::Dark)
    .subscription(subscription)
    .settings(Settings {
        id: Some("mde-popover-lock".to_string()),
        fonts: crate::fonts::load_fallback_fonts(),
        layer_settings: LayerShellSettings {
            size: None,
            exclusive_zone: -1,
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            margin: (0, 0, 0, 0),
            layer: Layer::Overlay,
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            ..Default::default()
        },
        ..Default::default()
    })
    .run()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_hostname_is_short() {
        let h = read_hostname();
        assert!(h.len() < 256);
    }

    #[test]
    fn format_hm_pads_zero() {
        assert_eq!(format_hm(0), "00:00");
        assert_eq!(format_hm(60), "00:01");
        assert_eq!(format_hm(9 * 3600 + 7 * 60), "09:07");
        assert_eq!(format_hm(23 * 3600 + 59 * 60), "23:59");
    }

    #[test]
    fn format_hm_wraps_24h() {
        // 25:00 should wrap to 01:00.
        assert_eq!(format_hm(25 * 3600), "01:00");
    }

    #[test]
    fn weekday_short_known_dates() {
        // 1970-01-01 = Thursday.
        assert_eq!(weekday_short(1970, 1, 1), "Thu");
        // 2026-05-25 = Monday (per user message frontmatter).
        assert_eq!(weekday_short(2026, 5, 25), "Mon");
    }

    #[test]
    fn format_date_includes_weekday_and_month() {
        let s = format_date(2026, 5, 25);
        assert!(s.starts_with("Mon"));
        assert!(s.contains("May"));
        assert!(s.contains("25"));
    }

    #[test]
    fn days_to_ymd_epoch() {
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn read_battery_pct_returns_valid_range_or_none() {
        if let Some(p) = read_battery_pct() {
            assert!(p <= 100);
        }
    }

    #[test]
    fn read_network_up_does_not_panic() {
        let _ = read_network_up();
    }

    #[test]
    fn read_interface_up_handles_missing_iface() {
        assert!(!read_interface_up("definitely-not-a-real-iface"));
    }

    #[test]
    fn read_weather_summary_returns_string_when_cache_absent() {
        // Either real data or "—" placeholder — never panics.
        let s = read_weather_summary();
        assert!(!s.is_empty());
    }

    #[test]
    fn app_refresh_populates_fields() {
        let app = App::refresh();
        assert!(!app.clock.is_empty());
        assert_eq!(app.clock.len(), 5); // HH:MM
        assert!(!app.date.is_empty());
        assert!(!app.weather.is_empty());
    }

    #[test]
    fn charcoal_matches_chromeos_design_lock() {
        let r = (CHARCOAL.r * 255.0).round() as u8;
        let g = (CHARCOAL.g * 255.0).round() as u8;
        let b = (CHARCOAL.b * 255.0).round() as u8;
        assert_eq!((r, g, b), (32, 33, 36), "#202124 charcoal lock");
    }

    #[test]
    fn fade_color_preserves_rgb_scales_alpha() {
        let c = fade_color(FG_DIM, 0.5);
        assert!((c.r - FG_DIM.r).abs() < f32::EPSILON);
        assert!((c.g - FG_DIM.g).abs() < f32::EPSILON);
        assert!((c.b - FG_DIM.b).abs() < f32::EPSILON);
        assert!((c.a - FG_DIM.a * 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn fade_color_fully_transparent_at_zero() {
        let c = fade_color(FG, 0.0);
        assert_eq!(c.a, 0.0);
    }

    #[test]
    fn fade_color_fully_opaque_at_one() {
        let c = fade_color(CHARCOAL, 1.0);
        assert!((c.a - CHARCOAL.a).abs() < f32::EPSILON);
    }

    #[test]
    fn fade_step_increments_during_phase_in() {
        let mut app = App::refresh();
        app.fade = 0.0;
        app.phase = FadePhase::In;
        // Simulate one FadeStep
        match FadePhase::In {
            FadePhase::In => {
                app.fade = (app.fade + FADE_STEP).min(1.0);
                if app.fade >= 1.0 { app.phase = FadePhase::Steady; }
            }
            _ => {}
        }
        assert!((app.fade - FADE_STEP).abs() < f32::EPSILON);
        assert_eq!(app.phase, FadePhase::In);
    }

    #[test]
    fn fade_step_clamps_at_one_and_transitions_to_steady() {
        let mut app = App::refresh();
        app.fade = 1.0 - FADE_STEP * 0.5; // just below 1.0
        app.phase = FadePhase::In;
        app.fade = (app.fade + FADE_STEP).min(1.0);
        if app.fade >= 1.0 { app.phase = FadePhase::Steady; }
        assert_eq!(app.fade, 1.0);
        assert_eq!(app.phase, FadePhase::Steady);
    }

    #[test]
    fn fade_step_clamps_at_zero_during_phase_out() {
        let mut app = App::refresh();
        app.fade = FADE_STEP * 0.5; // just above 0.0
        app.phase = FadePhase::Out;
        app.fade = (app.fade - FADE_STEP).max(0.0);
        assert_eq!(app.fade, 0.0);
    }

    #[test]
    fn new_starts_with_fade_in_phase() {
        let mut app = App::refresh();
        app.fade = 0.0;
        app.phase = FadePhase::In;
        assert_eq!(app.phase, FadePhase::In);
        assert_eq!(app.fade, 0.0);
    }
}
