//! v4.0.1 WM-3 (2026-05-23) — window-actions popover.
//!
//! Right-click on a dock cell pops this surface above the panel
//! edge with quick actions on the targeted sway window:
//!
//!   * Move to workspace 1 / 2 / 3 / 4 (`swaymsg [con_id=N]
//!     move container to workspace M`).
//!   * Close window (`swaymsg [con_id=N] kill`).
//!   * Pin to dock / Unpin from dock (toggles
//!     `~/.config/mde/panel.toml` via `mackes_config::pin_app`
//!     / `unpin_app`).
//!
//! Spawn from another binary by setting two env vars and exec
//! ing `mde-popover window-actions`:
//!
//!   MDE_WINDOW_CON_ID    sway container id of the window
//!   MDE_WINDOW_APP_ID    `app_id` (so the popover can label
//!                        the surface + know what `.desktop`
//!                        to pin/unpin).
//!
//! Both default to empty; an empty CON_ID means the
//! "swaymsg [con_id=]" arms become no-ops (defensive — the
//! popover still renders but the buttons can't do anything).
//!
//! Anchor: bottom (popover floats above the dock), 240 px wide,
//! ~auto height (header + 6 buttons). Esc / outside-click /
//! close-button dismiss.

use std::process::{Command, Stdio};

use iced::widget::{button, column, container, mouse_area, row, text, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Subscription, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

const WIDTH: u32 = 240;
const HEIGHT: u32 = 320;

const ACCENT: Color = Color {
    r: 0.357,
    g: 0.416,
    b: 0.961,
    a: 1.0,
};
const URGENT: Color = Color {
    r: 0.953,
    g: 0.514,
    b: 0.137,
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
    /// Workspace number 1-4. Maps to `swaymsg [con_id=N] move
    /// container to workspace M`.
    MoveToWorkspace(u32),
    /// `swaymsg [con_id=N] kill`.
    CloseWindow,
    /// Toggle pin state via `mackes_config`.
    TogglePin,
    /// v3.0.3 E.19 wiring — spawn the icon-mapper popover for
    /// this cell's app_id.
    OpenIconMapper,
    /// Close the popover without taking action.
    Exit,
}

pub struct App {
    con_id: String,
    app_id: String,
    pinned: bool,
    last_status: Option<String>,
}

fn namespace() -> String {
    "mde-popover-window-actions".to_string()
}

fn update(state: &mut App, msg: Message) -> Task<Message> {
    match msg {
        Message::MoveToWorkspace(n) => {
            run_move_to_workspace(&state.con_id, n);
            std::process::exit(0);
        }
        Message::CloseWindow => {
            run_close_window(&state.con_id);
            std::process::exit(0);
        }
        Message::TogglePin => {
            let toggled = toggle_pin(&state.app_id);
            state.pinned = toggled;
            state.last_status = Some(if toggled {
                "Pinned".to_string()
            } else {
                "Unpinned".to_string()
            });
            Task::none()
        }
        Message::OpenIconMapper => {
            spawn_icon_mapper(&state.app_id);
            std::process::exit(0);
        }
        Message::Exit => std::process::exit(0),
        _ => Task::none(),
    }
}

fn view(state: &App) -> Element<'_, Message> {
    let header_label = if state.app_id.is_empty() {
        "Window".to_string()
    } else {
        state.app_id.clone()
    };
    let header = row![
        text(header_label).size(13).color(FG_TEXT),
        Space::new().width(Length::Fill),
        crate::dismiss::close_button(Message::Exit),
    ]
    .align_y(iced::Alignment::Center);

    let mut body = column![
        header,
        Space::new().height(Length::Fixed(10.0)),
        text("Move to workspace").size(10).color(FG_MUTED),
        Space::new().height(Length::Fixed(4.0)),
    ]
    .spacing(0)
    .padding(Padding {
        top: 14.0,
        right: 14.0,
        bottom: 12.0,
        left: 14.0,
    });

    let mut ws_row = row![].spacing(6).align_y(iced::Alignment::Center);
    for n in 1u32..=4 {
        ws_row = ws_row.push(workspace_button(n));
    }
    body = body.push(ws_row);
    body = body.push(Space::new().height(Length::Fixed(12.0)));
    body = body.push(
        menu_button("Close window", URGENT, Message::CloseWindow),
    );
    body = body.push(Space::new().height(Length::Fixed(6.0)));
    let pin_label = if state.pinned {
        "Unpin from dock"
    } else {
        "Pin to dock"
    };
    body = body.push(menu_button(pin_label, ACCENT, Message::TogglePin));
    body = body.push(Space::new().height(Length::Fixed(6.0)));
    body = body.push(menu_button(
        "Customize icon…",
        ACCENT,
        Message::OpenIconMapper,
    ));
    if let Some(status) = &state.last_status {
        body = body.push(Space::new().height(Length::Fixed(6.0)));
        body = body.push(text(status.clone()).size(10).color(FG_MUTED));
    }
    body = body.push(Space::new().height(Length::Fill));
    body = body.push(
        text("Esc closes · click outside dismisses")
            .size(9)
            .color(FG_MUTED),
    );

    let card: Element<'_, Message> = container(body)
        .width(Length::Fixed(WIDTH as f32))
        .height(Length::Fixed(HEIGHT as f32))
        .style(popover_surface)
        .into();

    // v3.0.4 backdrop. Pinned bottom-left so the popover
    // floats above the dock with a small left gutter (the
    // dock entries that trigger this popover sit on the
    // left edge of the screen first).
    let dismiss = || {
        mouse_area(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(Message::Exit)
    };
    let bottom_strip = row![
        container(card).padding(Padding {
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

fn subscription(_state: &App) -> Subscription<Message> {
    use iced::event;
    event::listen_with(|event, status, _window| {
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
    })
}

pub fn run() -> iced_layershell::Result {
    iced_layershell::application(
        || {
            let con_id = std::env::var("MDE_WINDOW_CON_ID").unwrap_or_default();
            let app_id = std::env::var("MDE_WINDOW_APP_ID").unwrap_or_default();
            let pinned = is_pinned(&app_id);
            tracing::info!(
                con_id = %con_id,
                app_id = %app_id,
                pinned,
                "window-actions popover open",
            );
            App {
                con_id,
                app_id,
                pinned,
                last_status: None,
            }
        },
        namespace,
        update,
        view,
    )
    .theme(|_: &App| Theme::Dark)
    .subscription(subscription)
    .settings(Settings {
        id: Some("mde-popover-window-actions".to_string()),
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

fn workspace_button(n: u32) -> Element<'static, Message> {
    button(
        text(n.to_string())
            .size(14)
            .color(FG_TEXT)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .padding(Padding {
        top: 8.0,
        right: 10.0,
        bottom: 8.0,
        left: 10.0,
    })
    .width(Length::Fixed(44.0))
    .on_press(Message::MoveToWorkspace(n))
    .style(|_t: &Theme, status: button::Status| accent_button_style(status))
    .into()
}

fn menu_button<'a>(label: &'a str, tint: Color, msg: Message) -> Element<'a, Message> {
    button(text(label).size(12).color(FG_TEXT))
        .padding(Padding {
            top: 8.0,
            right: 12.0,
            bottom: 8.0,
            left: 12.0,
        })
        .width(Length::Fill)
        .on_press(msg)
        .style(move |_t: &Theme, status: button::Status| tinted_button_style(status, tint))
        .into()
}

fn accent_button_style(status: button::Status) -> button::Style {
    tinted_button_style(status, ACCENT)
}

fn tinted_button_style(status: button::Status, tint: Color) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(Background::Color(Color {
            r: tint.r,
            g: tint.g,
            b: tint.b,
            a: 0.20,
        })),
        button::Status::Pressed => Some(Background::Color(Color {
            r: tint.r,
            g: tint.g,
            b: tint.b,
            a: 0.32,
        })),
        _ => Some(Background::Color(Color {
            r: tint.r,
            g: tint.g,
            b: tint.b,
            a: 0.08,
        })),
    };
    button::Style {
        background: bg,
        text_color: FG_TEXT,
        border: Border {
            color: Color {
                r: tint.r,
                g: tint.g,
                b: tint.b,
                a: 0.40,
            },
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

fn popover_surface(_t: &Theme) -> container::Style {
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

fn run_move_to_workspace(con_id: &str, n: u32) {
    if con_id.is_empty() {
        return;
    }
    let arg = format!("[con_id={con_id}]");
    let to = format!("workspace number {n}");
    let _ = Command::new("swaymsg")
        .args([&arg, "move", "container", "to", &to])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn spawn_icon_mapper(app_id: &str) {
    if app_id.is_empty() {
        return;
    }
    let _ = Command::new("mde-popover")
        .arg("icon-mapper")
        .env("MDE_ICON_MAPPER_APP_ID", app_id)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

fn run_close_window(con_id: &str) {
    if con_id.is_empty() {
        return;
    }
    let arg = format!("[con_id={con_id}]");
    let _ = Command::new("swaymsg")
        .args([&arg, "kill"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn is_pinned(app_id: &str) -> bool {
    let cfg_path = panel_config_path();
    let Ok(raw) = std::fs::read_to_string(&cfg_path) else {
        return false;
    };
    let Ok(cfg) = mackes_config::parse(&raw) else {
        return false;
    };
    let bare = app_id.trim_end_matches(".desktop");
    cfg.dock.items.iter().any(|i| match i {
        mackes_config::DockItem::App { desktop: d } => d.trim_end_matches(".desktop") == bare,
        mackes_config::DockItem::Mesh { .. } => false,
    })
}

/// Returns the new pinned state (true = now pinned, false =
/// now unpinned).
fn toggle_pin(app_id: &str) -> bool {
    let bare = app_id.trim_end_matches(".desktop").to_string();
    if bare.is_empty() {
        return false;
    }
    let cfg_path = panel_config_path();
    let raw = std::fs::read_to_string(&cfg_path).unwrap_or_default();
    let mut cfg = mackes_config::parse(&raw).unwrap_or_else(|_| mackes_config::default_config());
    let already_pinned = cfg.dock.items.iter().any(|i| match i {
        mackes_config::DockItem::App { desktop: d } => d.trim_end_matches(".desktop") == bare,
        mackes_config::DockItem::Mesh { .. } => false,
    });
    if already_pinned {
        mackes_config::unpin_app(&mut cfg, &format!("{bare}.desktop"));
    } else {
        mackes_config::pin_app(&mut cfg, &format!("{bare}.desktop"));
    }
    if let Ok(s) = mackes_config::to_toml_string(&cfg) {
        if let Some(parent) = cfg_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&cfg_path, s);
    }
    !already_pinned
}

fn panel_config_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    std::path::PathBuf::from(home).join(".config/mde/panel.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_pinned_for_visual_consistency() {
        assert_eq!(WIDTH, 240);
        assert_eq!(HEIGHT, 320);
    }

    #[test]
    fn workspace_button_handles_all_four_workspaces() {
        // Smoke-test that the helper returns an Element for
        // every workspace number — guards against off-by-one
        // tweaks to the 1..=4 range above.
        for n in 1u32..=4 {
            let _: Element<'static, Message> = workspace_button(n);
        }
    }

    #[test]
    fn run_move_to_workspace_with_empty_con_id_is_noop() {
        // The function returns Unit; the no-op contract is that
        // it doesn't panic and doesn't shell out — we can only
        // assert the panic-free half here. The shell-out half is
        // covered by the empty-check guard.
        run_move_to_workspace("", 1);
    }

    #[test]
    fn run_close_window_with_empty_con_id_is_noop() {
        run_close_window("");
    }
}
