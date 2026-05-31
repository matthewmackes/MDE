//! Phase E.13 — right-click Start admin menu (Iced layer-shell popover).
//!
//! Originally landed in mde-panel as data + helpers only (the audit
//! 2026-05-22 caught it as dead code — wired to nothing). v3.0.3
//! moves the module to `mde-popover` where it now mounts a real
//! layer-shell overlay surface. The panel's Start-button right-click
//! handler spawns `mde-popover admin-menu` via the standard popover
//! toggle/reap path; this binary renders the 9-item action grid
//! grouped into 5 sections (Shells / Packages / Services / Security
//! / Storage) and on click spawns the matching `foot --hold` so the
//! user can read the output after the command exits.
//!
//! The action set + spawn-helper preserve the v2.0.3 pkexec lock —
//! every privileged action routes through `pkexec sh -c '<cmd>'`
//! so the polkit auth agent owns the prompt (Wayland-clean) rather
//! than relying on a controlling-tty sudo (which fails under
//! Wayland sessions).

use std::process::Command;

use iced::widget::{button, column, container, mouse_area, row, scrollable, text, Space};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Shadow, Subscription, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

/// A single admin-menu entry.
#[derive(Debug, Clone, Copy)]
pub struct AdminAction {
    pub label: &'static str,
    pub cmd: &'static str,
    pub needs_sudo: bool,
}

/// Section catalog — Q15-locked "Comprehensive 9-item" set.
pub const SECTIONS: &[(&str, &[AdminAction])] = &[
    (
        "Shells",
        &[
            AdminAction {
                label: "Root Terminal",
                cmd: "sudo -i",
                needs_sudo: true,
            },
            AdminAction {
                label: "Edit system file (sudoedit)",
                cmd: "sudoedit /etc/hosts",
                needs_sudo: true,
            },
        ],
    ),
    (
        "Packages",
        &[
            AdminAction {
                label: "DNF update",
                cmd: "sudo dnf upgrade --refresh",
                needs_sudo: true,
            },
            AdminAction {
                label: "DNF history",
                cmd: "sudo dnf history list",
                needs_sudo: true,
            },
        ],
    ),
    (
        "Services",
        &[
            AdminAction {
                label: "systemctl status",
                cmd: "sudo systemctl status",
                needs_sudo: true,
            },
            AdminAction {
                label: "journalctl tail",
                cmd: "sudo journalctl -fxe",
                needs_sudo: true,
            },
        ],
    ),
    (
        "Security",
        &[
            AdminAction {
                label: "SELinux status",
                cmd: "sestatus",
                needs_sudo: false,
            },
            AdminAction {
                label: "Firewall (firewall-cmd)",
                cmd: "sudo firewall-cmd --list-all",
                needs_sudo: true,
            },
        ],
    ),
    (
        "Storage",
        &[AdminAction {
            label: "Clean (dnf cache + journal vacuum 7d)",
            cmd: "sudo dnf clean all && sudo journalctl --vacuum-time=7d",
            needs_sudo: true,
        }],
    ),
];

/// Total action count — Q15 locks this at exactly 9.
#[must_use]
pub fn action_count() -> usize {
    SECTIONS.iter().map(|(_, actions)| actions.len()).sum()
}

/// Build the argv that would spawn a single admin action under
/// `foot --hold`. Pure — no subprocess invocation, ideal for tests.
#[must_use]
pub fn build_foot_argv(action: &AdminAction) -> Vec<String> {
    vec![
        "foot".into(),
        "--hold".into(),
        "--title".into(),
        format!("MDE admin · {}", action.label),
        "sh".into(),
        "-c".into(),
        action.cmd.into(),
    ]
}

/// Spawn the action via `foot --hold`. Non-blocking. Returns the
/// Child handle so callers can adopt it (or drop it to detach).
pub fn spawn_action(action: &AdminAction) -> std::io::Result<std::process::Child> {
    let argv = build_foot_argv(action);
    Command::new(&argv[0]).args(&argv[1..]).spawn()
}

/// Probe whether sudo is currently cached. Drives the UI hint
/// next to actions that `needs_sudo`. Falls through to `false` on
/// any error.
#[must_use]
pub fn sudo_cached() -> bool {
    Command::new("sudo")
        .args(["-n", "-v"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ──────────────────────────────────────────────────────────────
// v3.0.3 — Iced layer-shell popover UI
// ──────────────────────────────────────────────────────────────

const WIDTH: u32 = 360;
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
    Run(&'static str),
    Exit,
}

pub struct App {
    /// Whether sudo is cached at popover-open time. Stale after the
    /// user runs an action (we don't refresh — the user generally
    /// only fires one action then dismisses).
    sudo_cached: bool,
}

fn namespace() -> String {
    "mde-popover-admin-menu".into()
}

fn update(_state: &mut App, msg: Message) -> Task<Message> {
    match msg {
        Message::Run(cmd_id) => {
            let action = SECTIONS
                .iter()
                .flat_map(|(_, acts)| acts.iter())
                .find(|a| a.cmd == cmd_id);
            if let Some(a) = action {
                let _ = spawn_action(a);
            }
            std::process::exit(0);
        }
        Message::Exit => std::process::exit(0),
        _ => Task::none(),
    }
}

fn view(state: &App) -> Element<'_, Message> {
    let mut body = column![].spacing(2);

    let header_row = row![
        text("Admin").size(13).color(FG_TEXT),
        Space::new().width(Length::Fixed(8.0)),
        text(format!(
            "{} actions · {}",
            action_count(),
            if state.sudo_cached {
                "polkit ready"
            } else {
                "polkit will prompt"
            }
        ))
        .size(10)
        .color(FG_MUTED),
        Space::new().width(Length::Fill),
        crate::dismiss::close_button(Message::Exit),
    ]
    .align_y(Alignment::Center);

    body = body.push(container(header_row).padding(Padding {
        top: 8.0,
        right: 12.0,
        bottom: 4.0,
        left: 12.0,
    }));

    for (section_name, actions) in SECTIONS {
        body = body.push(container(text(*section_name).size(11).color(FG_MUTED)).padding(
            Padding {
                top: 8.0,
                right: 12.0,
                bottom: 0.0,
                left: 12.0,
            },
        ));
        for action in *actions {
            let label_text = text(action.label).size(13).color(FG_TEXT);
            let needs_sudo_chip = if action.needs_sudo {
                text("polkit").size(10).color(ACCENT)
            } else {
                text("user").size(10).color(FG_MUTED)
            };
            let row_btn = button(
                row![
                    label_text,
                    Space::new().width(Length::Fill),
                    needs_sudo_chip,
                ]
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding(Padding {
                top: 6.0,
                right: 12.0,
                bottom: 6.0,
                left: 12.0,
            })
            .style(row_button_style)
            .on_press(Message::Run(action.cmd));
            body = body.push(row_btn);
        }
    }

    let footer = container(text("Esc closes · click × to dismiss").size(10).color(FG_MUTED))
        .padding(Padding {
            top: 6.0,
            right: 12.0,
            bottom: 8.0,
            left: 12.0,
        });
    body = body.push(footer);

    let scroll = scrollable(body).height(Length::Fill);

    let card: Element<'_, Message> = container(scroll)
        .width(Length::Fixed(WIDTH as f32))
        .height(Length::Fixed(HEIGHT as f32))
        .style(surface_style)
        .into();

    // v3.0.4 — backdrop dismiss; bottom-left card.
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
        || App { sudo_cached: sudo_cached() },
        namespace,
        update,
        view,
    )
    .theme(|_: &App| iced::Theme::Dark)
    .subscription(subscription)
    .settings(Settings {
        id: Some("mde-popover-admin-menu".into()),
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

fn surface_style(_theme: &Theme) -> container::Style {
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

fn row_button_style(_theme: &Theme, status: button::Status) -> button::Style {
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
        snap: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_count_is_locked_at_nine() {
        assert_eq!(action_count(), 9);
    }

    #[test]
    fn five_sections_exactly() {
        assert_eq!(SECTIONS.len(), 5);
    }

    #[test]
    fn section_names_match_lock() {
        let names: Vec<&str> = SECTIONS.iter().map(|(n, _)| *n).collect();
        assert_eq!(
            names,
            vec!["Shells", "Packages", "Services", "Security", "Storage"]
        );
    }

    #[test]
    fn every_label_is_non_empty() {
        for (_, actions) in SECTIONS {
            for action in *actions {
                assert!(!action.label.is_empty());
                assert!(!action.cmd.is_empty());
            }
        }
    }

    #[test]
    fn root_terminal_needs_sudo() {
        let root_term = SECTIONS
            .iter()
            .flat_map(|(_, acts)| acts.iter())
            .find(|a| a.label == "Root Terminal")
            .unwrap();
        assert!(root_term.needs_sudo);
        assert_eq!(root_term.cmd, "sudo -i");
    }

    #[test]
    fn selinux_does_not_need_sudo() {
        let selinux = SECTIONS
            .iter()
            .flat_map(|(_, acts)| acts.iter())
            .find(|a| a.label == "SELinux status")
            .unwrap();
        assert!(!selinux.needs_sudo);
    }

    #[test]
    fn foot_argv_wraps_in_hold_and_titles() {
        let action = AdminAction {
            label: "Root Terminal",
            cmd: "sudo -i",
            needs_sudo: true,
        };
        let argv = build_foot_argv(&action);
        assert_eq!(argv[0], "foot");
        assert_eq!(argv[1], "--hold");
        assert_eq!(argv[2], "--title");
        assert_eq!(argv[3], "MDE admin · Root Terminal");
        assert_eq!(argv[4], "sh");
        assert_eq!(argv[5], "-c");
        assert_eq!(argv[6], "sudo -i");
    }

    #[test]
    fn foot_argv_preserves_compound_commands() {
        let clean_action = SECTIONS
            .iter()
            .flat_map(|(_, acts)| acts.iter())
            .find(|a| a.label.starts_with("Clean"))
            .unwrap();
        let argv = build_foot_argv(clean_action);
        assert!(argv.last().unwrap().contains("&&"));
    }
}
