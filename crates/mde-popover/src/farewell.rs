//! `mde-popover farewell` — session-end fade-out overlay (ANIM-7.c, Q40).
//!
//! Fullscreen Layer::Overlay that fades from transparent to opaque
//! charcoal (~200 ms, Q3 grid tier) then executes the session-exit
//! command. Mirrors ANIM-7.d's lock crossfade; this is the egress
//! counterpart (fade-out = session ending, fade-in = lock appearing).
//!
//! Actions:
//!   `logout`   → `swaymsg exit`
//!   `restart`  → `systemctl reboot`
//!   `shutdown` → `systemctl poweroff`
//!
//! Wired via `$mod+Shift+e` in `data/sway/config.d/mackes-defaults.conf`
//! (logout), and called by any power-button surface in mde-panel/mde-portal
//! by passing the matching action slug.

#![forbid(unsafe_code)]

use std::str::FromStr;

use clap::Parser;
use iced::widget::{container, mouse_area, Space};
use iced::widget::container::Style as ContainerStyle;
use iced::{Background, Color, Element, Length, Subscription, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

// ── Design token (lock palette §3) ───────────────────────────────────────────
const CHARCOAL: Color = Color { r: 0.125, g: 0.129, b: 0.141, a: 1.0 };

// ── Timing (sway-native-shell.md Q3 — 200 ms grid tier) ──────────────────────
const FADE_STEP: f32 = 0.08; // 13 ticks × 16 ms ≈ 208 ms

// ── Action ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Action {
    #[default]
    Logout,
    Restart,
    Shutdown,
}

impl Action {
    fn command(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Action::Logout => ("swaymsg", &["exit"]),
            Action::Restart => ("systemctl", &["reboot"]),
            Action::Shutdown => ("systemctl", &["poweroff"]),
        }
    }
}

impl FromStr for Action {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "logout" => Ok(Action::Logout),
            "restart" => Ok(Action::Restart),
            "shutdown" => Ok(Action::Shutdown),
            _ => Err(format!("unknown action {s:?} (expected logout|restart|shutdown)")),
        }
    }
}

fn execute_action(action: Action) {
    let (cmd, args) = action.command();
    let _ = std::process::Command::new(cmd).args(args).spawn();
    std::process::exit(0);
}

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
#[command(name = "mde-popover-farewell", about = "Session-end fade-out overlay")]
struct Cli {
    /// Session action to run after the fade completes.
    #[arg(long, default_value = "logout")]
    action: String,
}

// ── Application ───────────────────────────────────────────────────────────────

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    FadeStep,
    Cancel,
}

pub struct App {
    action: Action,
    fade: f32,
}

fn namespace() -> String {
    "mde-popover-farewell".to_string()
}

fn update(state: &mut App, msg: Message) -> Task<Message> {
    match msg {
        Message::FadeStep => {
            state.fade = (state.fade + FADE_STEP).min(1.0);
            if state.fade >= 1.0 {
                execute_action(state.action);
            }
        }
        Message::Cancel => std::process::exit(0),
        _ => {}
    }
    Task::none()
}

fn view(state: &App) -> Element<'_, Message> {
    let fade = state.fade;
    let bg = Color { a: CHARCOAL.a * fade, ..CHARCOAL };

    mouse_area(
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_: &Theme| ContainerStyle {
                background: Some(Background::Color(bg)),
                ..Default::default()
            }),
    )
    .on_press(Message::Cancel)
    .into()
}

fn subscription(_state: &App) -> Subscription<Message> {
    iced::time::every(std::time::Duration::from_millis(16))
        .map(|_| Message::FadeStep)
}

pub fn run() -> iced_layershell::Result {
    let cli = Cli::parse();
    let action = cli.action.parse::<Action>().unwrap_or_else(|e| {
        eprintln!("mde-popover farewell: {e}");
        std::process::exit(2);
    });
    tracing::info!(?action, "farewell overlay spawned");

    iced_layershell::application(
        move || App { action, fade: 0.0 },
        namespace,
        update,
        view,
    )
    .theme(|_: &App| iced::Theme::Dark)
    .subscription(subscription)
    .settings(Settings {
        id: Some("mde-popover-farewell".to_string()),
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
    fn action_from_str_logout() {
        assert_eq!("logout".parse::<Action>().unwrap(), Action::Logout);
    }

    #[test]
    fn action_from_str_restart() {
        assert_eq!("restart".parse::<Action>().unwrap(), Action::Restart);
    }

    #[test]
    fn action_from_str_shutdown() {
        assert_eq!("shutdown".parse::<Action>().unwrap(), Action::Shutdown);
    }

    #[test]
    fn action_from_str_unknown_errors() {
        assert!("xyzzy".parse::<Action>().is_err());
    }

    #[test]
    fn action_command_logout_is_swaymsg_exit() {
        let (cmd, args) = Action::Logout.command();
        assert_eq!(cmd, "swaymsg");
        assert_eq!(args, &["exit"]);
    }

    #[test]
    fn action_command_restart_is_systemctl_reboot() {
        let (cmd, args) = Action::Restart.command();
        assert_eq!(cmd, "systemctl");
        assert_eq!(args, &["reboot"]);
    }

    #[test]
    fn action_command_shutdown_is_systemctl_poweroff() {
        let (cmd, args) = Action::Shutdown.command();
        assert_eq!(cmd, "systemctl");
        assert_eq!(args, &["poweroff"]);
    }

    #[test]
    fn fade_step_reaches_one_after_enough_steps() {
        let mut fade: f32 = 0.0;
        for _ in 0..100 {
            fade = (fade + FADE_STEP).min(1.0);
            if fade >= 1.0 {
                break;
            }
        }
        assert_eq!(fade, 1.0);
    }

    #[test]
    fn charcoal_alpha_at_zero_opacity() {
        let bg = Color { a: CHARCOAL.a * 0.0, ..CHARCOAL };
        assert_eq!(bg.a, 0.0);
    }

    #[test]
    fn charcoal_alpha_at_full_opacity() {
        let bg = Color { a: CHARCOAL.a * 1.0, ..CHARCOAL };
        assert!((bg.a - CHARCOAL.a).abs() < f32::EPSILON);
    }

    #[test]
    fn charcoal_rgb_is_202124() {
        let r = (CHARCOAL.r * 255.0).round() as u8;
        let g = (CHARCOAL.g * 255.0).round() as u8;
        let b = (CHARCOAL.b * 255.0).round() as u8;
        assert_eq!((r, g, b), (32, 33, 36));
    }
}
