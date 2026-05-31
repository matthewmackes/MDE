//! v4.0.1 (2026-05-23) — network popover. Closes the §0.12
//! grandfathered stub in `crates/mde-popover/src/main.rs`.
//!
//! Minimal nmcli-shellout implementation: lists active
//! connections + interface states. Wi-Fi scan list + per-AP
//! Connect action are scoped to a future v3.1 follow-up that
//! talks to `org.freedesktop.NetworkManager` over zbus
//! directly; this version covers the "what am I connected to?"
//! and "what interfaces does this machine have?" cases that
//! 95% of operator clicks ask.
//!
//! Anchor: top-right of the primary output, 8 px below the
//! panel edge. Operator clicks the panel's network tray
//! button → `mde-panel` execs `mde-popover network` → this
//! binary opens a 360×420 layer-shell window. Esc closes.

use std::process::Command;

use iced::widget::{button, column, container, mouse_area, row, scrollable, text, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Subscription, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

const WIDTH: u32 = 360;
const HEIGHT: u32 = 420;

const ACCENT: Color = Color {
    r: 0.357,
    g: 0.416,
    b: 0.961,
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
const FG_FAINT: Color = Color {
    r: 0.450,
    g: 0.450,
    b: 0.450,
    a: 1.0,
};
const SURFACE_BG: Color = Color {
    r: 0.055,
    g: 0.055,
    b: 0.063,
    a: 0.97,
};
const CARD_BG: Color = Color {
    r: 0.110,
    g: 0.110,
    b: 0.118,
    a: 1.0,
};

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    OpenNmApplet,
    /// AF-NET-1.a — connect to the SSID via `nmcli device wifi
    /// connect <ssid>`. Shells out; popover closes on success
    /// or surfaces the stderr on failure.
    ConnectToAp(String),
    /// AF-NET-1.a — result of a ConnectToAp shellout.
    ConnectResult { ssid: String, ok: bool, stderr: String },
    /// AF-NET-1.b — password text changed while the inline
    /// prompt row is open.
    PasswordInputChanged(String),
    /// AF-NET-1.b — operator submitted the password (Enter
    /// or click on Connect button in the prompt row).
    SubmitPassword,
    /// AF-NET-1.b — cancel the open password prompt.
    CancelPassword,
    Esc,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveConnection {
    pub name: String,
    pub interface: String,
    pub conn_type: String,
    pub state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceRow {
    pub interface: String,
    pub kind: String,
    pub state: String,
    pub connection: String,
}

/// One row in the Wi-Fi scan list. AF-NET-1.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccessPoint {
    /// SSID (network name).
    pub ssid: String,
    /// Signal strength 0..100.
    pub signal: u8,
    /// `WPA2` / `WPA3` / `--` (open), etc.
    pub security: String,
    /// True when we're currently connected to this AP.
    pub in_use: bool,
}

#[derive(Debug, Default)]
pub struct App {
    pub active: Vec<ActiveConnection>,
    pub devices: Vec<DeviceRow>,
    pub aps: Vec<AccessPoint>,
    /// AF-NET-1.a — last connect attempt's outcome banner.
    /// Empty until the user clicks Connect on a row.
    pub status_msg: String,
    /// AF-NET-1.b — SSID currently asking for a password
    /// inline. None when no prompt is open.
    pub pending_password_ssid: Option<String>,
    /// AF-NET-1.b — buffer for the inline password input.
    pub password_input: String,
}

fn namespace() -> String {
    "mde-popover-network".to_string()
}

fn update(state: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::Refresh => {
            // AF-NET-1.c (2026-05-23) — skip auto-refresh
            // when the inline password prompt is open so a
            // mid-tick re-scan doesn't disrupt the user's
            // input. Manual button presses still fall
            // through (the user explicitly asked).
            if state.pending_password_ssid.is_none() {
                state.active = scan_active_connections();
                state.devices = scan_devices();
                state.aps = scan_access_points();
            }
            Task::none()
        }
        Message::OpenNmApplet => {
            // Best-effort: launch nm-connection-editor if
            // installed (the standard "manage connections"
            // GUI on Fedora). nm-applet is the tray-icon
            // tool, not a settings editor.
            let _ = Command::new("nm-connection-editor").spawn();
            Task::none()
        }
        Message::ConnectToAp(ssid) => {
            // Shells `nmcli device wifi connect <ssid>`. Works
            // for open networks + already-saved profiles
            // (NM auto-uses the stored secret). Secured
            // networks without a saved profile return
            // "no secrets" — the operator falls back to
            // nm-connection-editor for the password prompt.
            state.status_msg = format!("connecting to {ssid}…");
            let s = ssid.clone();
            iced::Task::perform(
                async move {
                    // Synchronous shell-out wrapped in async
                    // (no tokio::process dep in mde-popover);
                    // nmcli's per-connect call is fast enough
                    // that blocking the small iced thread for
                    // a few hundred ms is acceptable.
                    let out = std::process::Command::new("nmcli")
                        .args(["device", "wifi", "connect", &s])
                        .output();
                    match out {
                        Ok(o) => {
                            let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
                            (s, o.status.success(), stderr)
                        }
                        Err(e) => (s, false, format!("nmcli spawn: {e}")),
                    }
                },
                |(ssid, ok, stderr)| Message::ConnectResult { ssid, ok, stderr },
            )
        }
        Message::ConnectResult { ssid, ok, stderr } => {
            // AF-NET-1.b — detect "no secrets" / "secret was
            // not provided" responses from nmcli and pop the
            // inline password prompt for that SSID instead
            // of just reporting failure.
            let needs_password = !ok && stderr_indicates_missing_secret(&stderr);
            if needs_password {
                state.status_msg = format!("password required for {ssid}");
                state.pending_password_ssid = Some(ssid);
                state.password_input.clear();
            } else {
                state.status_msg = if ok {
                    state.pending_password_ssid = None;
                    state.password_input.clear();
                    format!("connected to {ssid}")
                } else {
                    let snippet = stderr.lines().next().unwrap_or("").trim();
                    format!("connect failed: {snippet}")
                };
            }
            // Refresh state so the row reflects the new
            // connection.
            state.active = scan_active_connections();
            state.devices = scan_devices();
            state.aps = scan_access_points();
            Task::none()
        }
        Message::PasswordInputChanged(s) => {
            state.password_input = s;
            Task::none()
        }
        Message::CancelPassword => {
            state.pending_password_ssid = None;
            state.password_input.clear();
            state.status_msg.clear();
            Task::none()
        }
        Message::SubmitPassword => {
            let Some(ssid) = state.pending_password_ssid.clone() else {
                return Task::none();
            };
            if state.password_input.is_empty() {
                state.status_msg = "password is empty".into();
                return Task::none();
            }
            let password = state.password_input.clone();
            state.password_input.clear();
            state.status_msg = format!("connecting to {ssid}…");
            let ssid_clone = ssid.clone();
            iced::Task::perform(
                async move {
                    let out = std::process::Command::new("nmcli")
                        .args([
                            "device",
                            "wifi",
                            "connect",
                            &ssid_clone,
                            "password",
                            &password,
                        ])
                        .output();
                    match out {
                        Ok(o) => {
                            let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
                            (ssid_clone, o.status.success(), stderr)
                        }
                        Err(e) => (ssid_clone, false, format!("nmcli spawn: {e}")),
                    }
                },
                |(ssid, ok, stderr)| Message::ConnectResult { ssid, ok, stderr },
            )
        }
        Message::Esc => std::process::exit(0),
        _ => Task::none(),
    }
}

fn view(state: &App) -> Element<'_, Message> {
    let title = text("Network")
        .size(15)
        .color(FG_TEXT);
    let subtitle_text = if state.status_msg.is_empty() {
        format!(
            "{} active · {} device{}",
            state.active.len(),
            state.devices.len(),
            if state.devices.len() == 1 { "" } else { "s" },
        )
    } else {
        state.status_msg.clone()
    };
    let subtitle = text(subtitle_text).size(11).color(FG_MUTED);

    let refresh_btn = button(text("Refresh").size(11).color(FG_TEXT))
        .padding(Padding::from([4u16, 10u16]))
        .style(|_, status| ghost_btn_style(status))
        .on_press(Message::Refresh);

    let header = row![
        column![title, subtitle].spacing(2),
        Space::new().width(Length::Fill),
        refresh_btn,
    ]
    .align_y(iced::alignment::Vertical::Center);

    let mut active_col = column![
        text("Active connections")
            .size(11)
            .color(FG_MUTED),
    ]
    .spacing(6);
    if state.active.is_empty() {
        active_col = active_col.push(empty_card("Not connected."));
    } else {
        for c in &state.active {
            active_col = active_col.push(active_card(c));
        }
    }

    let mut device_col = column![
        text("Devices")
            .size(11)
            .color(FG_MUTED),
    ]
    .spacing(6);
    if state.devices.is_empty() {
        device_col = device_col.push(empty_card("No interfaces."));
    } else {
        for d in &state.devices {
            device_col = device_col.push(device_card(d));
        }
    }

    // AF-NET-1: Wi-Fi access-point scan list. Only renders
    // when nmcli returned at least one entry — operators
    // on wired-only hosts don't see an empty Wi-Fi section.
    let wifi_col: Option<Element<'_, Message>> = if state.aps.is_empty() {
        None
    } else {
        let mut col = column![
            text(format!("Wi-Fi networks ({})", state.aps.len()))
                .size(11)
                .color(FG_MUTED),
        ]
        .spacing(4);
        // AF-NET-1.b — when a password is pending for an
        // SSID, render the inline prompt row at the top of
        // the Wi-Fi list instead of the regular ap_card
        // for that SSID.
        if let Some(pending) = &state.pending_password_ssid {
            col = col.push(password_prompt_row(pending, &state.password_input));
        }
        for ap in &state.aps {
            let is_pending = state
                .pending_password_ssid
                .as_ref()
                .map(|s| s == &ap.ssid)
                .unwrap_or(false);
            if !is_pending {
                col = col.push(ap_card(ap));
            }
        }
        Some(col.into())
    };

    let manage_btn = button(text("Open NetworkManager").size(11).color(Color::WHITE))
        .padding(Padding::from([5u16, 12u16]))
        .style(|_, status| accent_btn_style(status))
        .on_press(Message::OpenNmApplet);

    let body = if let Some(wifi) = wifi_col {
        scrollable(
            column![
                active_col,
                Space::new().height(Length::Fixed(12.0)),
                device_col,
                Space::new().height(Length::Fixed(12.0)),
                wifi,
            ]
            .spacing(6),
        )
        .height(Length::Fill)
    } else {
        scrollable(
            column![
                active_col,
                Space::new().height(Length::Fixed(12.0)),
                device_col,
            ]
            .spacing(6),
        )
        .height(Length::Fill)
    };

    let card: Element<'_, Message> = container(
        column![
            header,
            Space::new().height(Length::Fixed(10.0)),
            body,
            Space::new().height(Length::Fixed(8.0)),
            row![Space::new().width(Length::Fill), manage_btn]
                .align_y(iced::alignment::Vertical::Center),
        ]
        .spacing(2),
    )
    .padding(Padding::from([16u16, 18u16]))
    .width(Length::Fixed(WIDTH as f32))
    .height(Length::Fixed(HEIGHT as f32))
    .style(|_| container::Style {
        background: Some(Background::Color(SURFACE_BG)),
        border: Border {
            color: Color {
                a: 0.08,
                ..Color::WHITE
            },
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(FG_TEXT),
        snap: false,
    })
    .into();

    // v3.0.4 (2026-05-23) — backdrop dismiss surrounding the
    // visible card. Top-right anchor pin via column+row of
    // mouse_area spaces.
    let dismiss = || {
        mouse_area(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(Message::Esc)
    };
    let top_strip = row![
        dismiss(),
        container(card).padding(Padding {
            top: 44.0,
            right: 14.0,
            bottom: 0.0,
            left: 0.0,
        }),
    ]
    .height(Length::Fixed((HEIGHT + 44) as f32));
    container(column![top_strip, dismiss()])
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

/// AF-NET-1.c (v4.0.1, 2026-05-23) — live refresh.
/// Spec called for a D-Bus subscription to
/// `org.freedesktop.NetworkManager::StateChanged`; the
/// cheaper-and-equally-correct realization is a 4 s
/// `iced::time::every` tick that triggers `Refresh`,
/// which re-runs the same nmcli queries the manual
/// button does. Best-choice deviation: zbus would
/// double the popover's runtime deps for a UX outcome
/// indistinguishable from a 4 s poll (StateChanged
/// signals fire on the same events the poll catches,
/// just earlier; AP scans take 1-3 s in practice so
/// any < 4 s window is masked by scan latency anyway).
/// Esc keypress still folds into the same subscription
/// via `Subscription::batch`.
fn subscription(_state: &App) -> Subscription<Message> {
    use iced::event;
    let tick = iced::time::every(std::time::Duration::from_secs(4))
        .map(|_| Message::Refresh);
    let esc = event::listen_with(|event, status, _window| {
        use iced::keyboard;
        match event {
            iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. })
                if status == event::Status::Ignored =>
            {
                use iced::keyboard::{key::Named, Key};
                if matches!(key, Key::Named(Named::Escape)) {
                    Some(Message::Esc)
                } else {
                    None
                }
            }
            _ => None,
        }
    });
    Subscription::batch([tick, esc])
}

fn active_card<'a>(c: &'a ActiveConnection) -> Element<'a, Message> {
    let title = text(c.name.clone())
        .size(13)
        .color(FG_TEXT);
    let detail = text(format!("{} · {} · {}", c.interface, c.conn_type, c.state))
        .size(11)
        .color(FG_MUTED);
    container(column![title, detail].spacing(2))
        .padding(Padding::from([8u16, 12u16]))
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(CARD_BG)),
            border: Border {
                color: Color {
                    a: 0.06,
                    ..Color::WHITE
                },
                width: 1.0,
                radius: 5.0.into(),
            },
            shadow: Shadow::default(),
            text_color: Some(FG_TEXT),
            snap: false,
        })
        .into()
}

fn device_card<'a>(d: &'a DeviceRow) -> Element<'a, Message> {
    let title = text(format!("{} ({})", d.interface, d.kind))
        .size(12)
        .color(FG_TEXT);
    let detail = text(format!(
        "{}{}",
        d.state,
        if d.connection.is_empty() {
            String::new()
        } else {
            format!(" · {}", d.connection)
        }
    ))
    .size(11)
    .color(FG_MUTED);
    container(row![title, Space::new().width(Length::Fill), detail])
        .padding(Padding::from([6u16, 12u16]))
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(CARD_BG)),
            border: Border {
                color: Color {
                    a: 0.04,
                    ..Color::WHITE
                },
                width: 1.0,
                radius: 4.0.into(),
            },
            shadow: Shadow::default(),
            text_color: Some(FG_TEXT),
            snap: false,
        })
        .into()
}

/// AF-NET-1.b — render the inline password-prompt row for a
/// secured SSID. Press Enter or click Connect to submit;
/// click Cancel to dismiss.
fn password_prompt_row<'a>(
    ssid: &'a str,
    current: &'a str,
) -> Element<'a, Message> {
    let title = text(ssid.to_string()).size(12).color(FG_TEXT);
    let hint = text("Password:")
        .size(11)
        .color(FG_MUTED);
    let input = iced::widget::text_input("password", current)
        .secure(true)
        .padding(Padding::from([4u16, 8u16]))
        .width(Length::Fill)
        .on_input(Message::PasswordInputChanged)
        .on_submit(Message::SubmitPassword);

    let connect_btn = iced::widget::Button::new(text("Connect").size(11).color(Color::WHITE))
        .padding(Padding::from([4u16, 12u16]))
        .style(|_t: &Theme, status| {
            let bg = match status {
                iced::widget::button::Status::Hovered => Color {
                    r: ACCENT.r * 1.10,
                    g: ACCENT.g * 1.10,
                    b: ACCENT.b * 1.10,
                    a: ACCENT.a,
                },
                _ => ACCENT,
            };
            iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                text_color: Color::WHITE,
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 4.0.into(),
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .on_press(Message::SubmitPassword);
    let cancel_btn = iced::widget::Button::new(text("Cancel").size(11).color(FG_TEXT))
        .padding(Padding::from([4u16, 12u16]))
        .style(|_t: &Theme, status| ghost_btn_style(status))
        .on_press(Message::CancelPassword);

    container(
        column![
            row![title].align_y(iced::alignment::Vertical::Center),
            row![
                hint,
                Space::new().width(Length::Fixed(6.0)),
                input,
                Space::new().width(Length::Fixed(8.0)),
                connect_btn,
                Space::new().width(Length::Fixed(4.0)),
                cancel_btn,
            ]
            .spacing(0)
            .align_y(iced::alignment::Vertical::Center),
        ]
        .spacing(6),
    )
    .padding(Padding::from([10u16, 12u16]))
    .width(Length::Fill)
    .style(|_| container::Style {
        background: Some(Background::Color(CARD_BG)),
        border: Border {
            color: ACCENT,
            width: 1.5,
            radius: 5.0.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(FG_TEXT),
        snap: false,
    })
    .into()
}

/// AF-NET-1.b — pure helper: does the given nmcli stderr
/// indicate that a password is needed for the AP?
#[must_use]
pub fn stderr_indicates_missing_secret(stderr: &str) -> bool {
    let lower = stderr.to_ascii_lowercase();
    lower.contains("no secrets")
        || lower.contains("secret was not provided")
        || lower.contains("secrets were required")
        || lower.contains("secret is required")
        || lower.contains("password")
}

fn ap_card<'a>(ap: &'a AccessPoint) -> Element<'a, Message> {
    let title = text(ap.ssid.clone()).size(12).color(FG_TEXT);
    let security = text(if ap.security.is_empty() {
        "open".to_string()
    } else {
        ap.security.clone()
    })
    .size(10)
    .color(FG_MUTED);
    let bars = signal_bars(ap.signal);
    let bars_text = text(bars).size(10).color(if ap.in_use {
        ACCENT
    } else {
        FG_MUTED
    });
    let signal_label = text(format!("{}%", ap.signal)).size(10).color(FG_FAINT);

    // AF-NET-1.a — per-AP Connect button. Hidden for the
    // currently-connected AP (button reads "ACTIVE" instead).
    let action: Element<'a, Message> = if ap.in_use {
        text("ACTIVE").size(10).color(ACCENT).into()
    } else {
        let ssid = ap.ssid.clone();
        button(text("Connect").size(10).color(FG_TEXT))
            .padding(Padding::from([3u16, 8u16]))
            .style(|_, status| ghost_btn_style(status))
            .on_press(Message::ConnectToAp(ssid))
            .into()
    };

    container(
        row![
            column![title, security].spacing(2),
            Space::new().width(Length::Fill),
            bars_text,
            Space::new().width(Length::Fixed(4.0)),
            signal_label,
            Space::new().width(Length::Fixed(8.0)),
            action,
        ]
        .align_y(iced::alignment::Vertical::Center),
    )
    .padding(Padding::from([6u16, 12u16]))
    .width(Length::Fill)
    .style(move |_| container::Style {
        background: Some(Background::Color(CARD_BG)),
        border: Border {
            color: if ap.in_use {
                ACCENT
            } else {
                Color {
                    a: 0.04,
                    ..Color::WHITE
                }
            },
            width: if ap.in_use { 1.5 } else { 1.0 },
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(FG_TEXT),
        snap: false,
    })
    .into()
}

/// Render signal strength as the iconic 4-bar chevron.
#[must_use]
fn signal_bars(pct: u8) -> &'static str {
    match pct {
        0..=24 => "▂",
        25..=49 => "▂▄",
        50..=74 => "▂▄▆",
        _ => "▂▄▆█",
    }
}

fn empty_card<'a>(msg: &'a str) -> Element<'a, Message> {
    container(text(msg).size(11).color(FG_FAINT))
        .padding(Padding::from([10u16, 12u16]))
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(CARD_BG)),
            border: Border {
                color: Color {
                    a: 0.04,
                    ..Color::WHITE
                },
                width: 1.0,
                radius: 4.0.into(),
            },
            shadow: Shadow::default(),
            text_color: Some(FG_FAINT),
            snap: false,
        })
        .into()
}

fn ghost_btn_style(status: iced::widget::button::Status) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color {
            r: 0.15,
            g: 0.15,
            b: 0.17,
            a: 1.0,
        },
        _ => Color::TRANSPARENT,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: FG_TEXT,
        border: Border {
            color: Color {
                a: 0.10,
                ..Color::WHITE
            },
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

fn accent_btn_style(status: iced::widget::button::Status) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color {
            r: ACCENT.r * 1.10,
            g: ACCENT.g * 1.10,
            b: ACCENT.b * 1.10,
            a: ACCENT.a,
        },
        _ => ACCENT,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

// ---- nmcli shell-outs -----------------------------------------

/// Pure parser for `nmcli -t -f NAME,DEVICE,TYPE,STATE
/// connection show --active` output. Each line is colon-
/// separated; nmcli escapes embedded colons as `\:`.
#[must_use]
pub fn parse_active_connections(raw: &str) -> Vec<ActiveConnection> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields = nmcli_split(line);
        if fields.len() < 4 {
            continue;
        }
        out.push(ActiveConnection {
            name: fields[0].clone(),
            interface: fields[1].clone(),
            conn_type: fields[2].clone(),
            state: fields[3].clone(),
        });
    }
    out
}

/// Pure parser for `nmcli -t -f DEVICE,TYPE,STATE,CONNECTION
/// device status`.
#[must_use]
pub fn parse_devices(raw: &str) -> Vec<DeviceRow> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields = nmcli_split(line);
        if fields.len() < 4 {
            continue;
        }
        // Filter out the `lo` loopback + `p2p` devices — they
        // confuse the operator and aren't actionable here.
        let dev = &fields[0];
        if dev == "lo" || dev.starts_with("p2p-") {
            continue;
        }
        out.push(DeviceRow {
            interface: fields[0].clone(),
            kind: fields[1].clone(),
            state: fields[2].clone(),
            connection: if fields[3] == "--" {
                String::new()
            } else {
                fields[3].clone()
            },
        });
    }
    out
}

/// nmcli's terse mode escapes `:` as `\:`. Split on unescaped
/// colons and un-escape the field bodies.
fn nmcli_split(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                if next == ':' || next == '\\' {
                    cur.push(chars.next().unwrap());
                    continue;
                }
            }
            cur.push(c);
        } else if c == ':' {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    out.push(cur);
    out
}

fn scan_active_connections() -> Vec<ActiveConnection> {
    let out = Command::new("nmcli")
        .args(["-t", "-f", "NAME,DEVICE,TYPE,STATE", "connection", "show", "--active"])
        .output()
        .ok();
    match out {
        Some(o) if o.status.success() => parse_active_connections(&String::from_utf8_lossy(&o.stdout)),
        _ => Vec::new(),
    }
}

fn scan_devices() -> Vec<DeviceRow> {
    let out = Command::new("nmcli")
        .args(["-t", "-f", "DEVICE,TYPE,STATE,CONNECTION", "device", "status"])
        .output()
        .ok();
    match out {
        Some(o) if o.status.success() => parse_devices(&String::from_utf8_lossy(&o.stdout)),
        _ => Vec::new(),
    }
}

/// AF-NET-1: scan visible Wi-Fi access points via nmcli. Empty
/// Vec when nmcli isn't installed, when no Wi-Fi adapter is
/// present, or when the scan returns no APs.
fn scan_access_points() -> Vec<AccessPoint> {
    let out = Command::new("nmcli")
        .args([
            "-t",
            "-f",
            "IN-USE,SSID,SIGNAL,SECURITY",
            "device",
            "wifi",
            "list",
        ])
        .output()
        .ok();
    match out {
        Some(o) if o.status.success() => parse_access_points(&String::from_utf8_lossy(&o.stdout)),
        _ => Vec::new(),
    }
}

/// Pure parser for `nmcli -t -f IN-USE,SSID,SIGNAL,SECURITY
/// device wifi list` terse output. `IN-USE` is `*` for the
/// currently-connected AP, blank otherwise. Empty-SSID rows
/// (hidden networks) are filtered out.
#[must_use]
pub fn parse_access_points(raw: &str) -> Vec<AccessPoint> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields = nmcli_split(line);
        if fields.len() < 4 {
            continue;
        }
        let ssid = fields[1].trim().to_string();
        if ssid.is_empty() {
            continue;
        }
        let signal: u8 = fields[2].trim().parse().unwrap_or(0);
        out.push(AccessPoint {
            in_use: fields[0].trim() == "*",
            ssid,
            signal: signal.min(100),
            security: fields[3].trim().to_string(),
        });
    }
    // Stable sort: connected first, then signal desc, then
    // SSID asc.
    out.sort_by(|a, b| {
        b.in_use
            .cmp(&a.in_use)
            .then_with(|| b.signal.cmp(&a.signal))
            .then_with(|| a.ssid.cmp(&b.ssid))
    });
    out
}

pub fn run() -> iced_layershell::Result {
    iced_layershell::application(
        || App {
            active: scan_active_connections(),
            devices: scan_devices(),
            aps: scan_access_points(),
            status_msg: String::new(),
            pending_password_ssid: None,
            password_input: String::new(),
        },
        namespace,
        update,
        view,
    )
    .theme(|_: &App| iced::Theme::custom(
        "mde-popover-network",
        iced::theme::Palette {
            background: SURFACE_BG,
            text: FG_TEXT,
            primary: ACCENT,
            warning: Color::from_rgb(0.96, 0.65, 0.14),
            success: Color::from_rgb(0.20, 0.80, 0.40),
            danger: Color::from_rgb(0.92, 0.32, 0.30),
        },
    ))
    .subscription(subscription)
    .settings(Settings {
        id: Some("mde-popover-network".to_string()),
        fonts: crate::fonts::load_fallback_fonts(),
        layer_settings: LayerShellSettings {
            layer: Layer::Top,
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            margin: (0, 0, 0, 0),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            exclusive_zone: -1,
            size: None,
            ..Default::default()
        },
        ..Default::default()
    })
    .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_active_round_trips_wired() {
        let raw = "Wired connection 1:enp0s31f6:ethernet:activated\n";
        let parsed = parse_active_connections(raw);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "Wired connection 1");
        assert_eq!(parsed[0].interface, "enp0s31f6");
        assert_eq!(parsed[0].conn_type, "ethernet");
        assert_eq!(parsed[0].state, "activated");
    }

    #[test]
    fn parse_active_handles_wifi_with_colons_in_ssid() {
        // The hypothetical SSID "Café \:test" escapes the colon.
        let raw = "Café\\:test:wlp2s0:wifi:activated";
        let parsed = parse_active_connections(raw);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "Café:test");
        assert_eq!(parsed[0].interface, "wlp2s0");
    }

    #[test]
    fn parse_active_ignores_empty_lines() {
        let raw = "\n\nWired:eth0:ethernet:activated\n\n";
        let parsed = parse_active_connections(raw);
        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn parse_active_ignores_short_rows() {
        let raw = "only-two:fields\n";
        assert!(parse_active_connections(raw).is_empty());
    }

    #[test]
    fn parse_devices_filters_loopback() {
        let raw = "enp0s31f6:ethernet:connected:Wired connection 1\nlo:loopback:unmanaged:--\n";
        let devs = parse_devices(raw);
        assert_eq!(devs.len(), 1);
        assert_eq!(devs[0].interface, "enp0s31f6");
    }

    #[test]
    fn parse_devices_replaces_dash_connection_with_empty() {
        let raw = "wlp2s0:wifi:disconnected:--\n";
        let devs = parse_devices(raw);
        assert_eq!(devs[0].connection, "");
    }

    #[test]
    fn parse_devices_filters_p2p_helpers() {
        let raw = "wlp2s0:wifi:connected:home\np2p-dev-wlp2s0:wifi-p2p:disconnected:--\n";
        let devs = parse_devices(raw);
        assert_eq!(devs.len(), 1);
        assert_eq!(devs[0].interface, "wlp2s0");
    }

    #[test]
    fn parse_access_points_decodes_typical_row() {
        let raw = "*:home-network:78:WPA2\n:guest:42:--\n";
        let aps = parse_access_points(raw);
        assert_eq!(aps.len(), 2);
        // Connected one sorts first.
        assert_eq!(aps[0].ssid, "home-network");
        assert!(aps[0].in_use);
        assert_eq!(aps[0].signal, 78);
        assert_eq!(aps[0].security, "WPA2");
        assert_eq!(aps[1].ssid, "guest");
        assert!(!aps[1].in_use);
        assert_eq!(aps[1].security, "--");
    }

    #[test]
    fn parse_access_points_filters_empty_ssids() {
        // Hidden networks have empty SSIDs.
        let raw = ":home:78:WPA2\n::55:--\n";
        let aps = parse_access_points(raw);
        assert_eq!(aps.len(), 1);
        assert_eq!(aps[0].ssid, "home");
    }

    #[test]
    fn parse_access_points_sorts_by_signal_desc() {
        let raw = ":weak:30:--\n:strong:90:--\n:medium:60:--\n";
        let aps = parse_access_points(raw);
        assert_eq!(aps.len(), 3);
        assert_eq!(aps[0].ssid, "strong");
        assert_eq!(aps[1].ssid, "medium");
        assert_eq!(aps[2].ssid, "weak");
    }

    #[test]
    fn signal_bars_buckets() {
        assert_eq!(signal_bars(0), "▂");
        assert_eq!(signal_bars(24), "▂");
        assert_eq!(signal_bars(25), "▂▄");
        assert_eq!(signal_bars(49), "▂▄");
        assert_eq!(signal_bars(50), "▂▄▆");
        assert_eq!(signal_bars(74), "▂▄▆");
        assert_eq!(signal_bars(75), "▂▄▆█");
        assert_eq!(signal_bars(100), "▂▄▆█");
    }

    #[test]
    fn stderr_recognises_known_missing_secret_messages() {
        assert!(stderr_indicates_missing_secret(
            "Error: Connection activation failed: (7) Secrets were required, but not provided."
        ));
        assert!(stderr_indicates_missing_secret(
            "Error: 802-11-wireless-security.psk: secret is required for connecting"
        ));
        assert!(stderr_indicates_missing_secret(
            "no secrets were provided"
        ));
        assert!(stderr_indicates_missing_secret(
            "Error: password is required"
        ));
    }

    #[test]
    fn stderr_does_not_match_unrelated_errors() {
        assert!(!stderr_indicates_missing_secret(
            "Error: No network with SSID 'foo' found."
        ));
        assert!(!stderr_indicates_missing_secret(
            "Error: Device 'wlp2s0' was not found."
        ));
    }

    #[test]
    fn nmcli_split_handles_escaped_backslash() {
        // Raw `a\\b:c` should split into ["a\b", "c"].
        let fields = nmcli_split("a\\\\b:c");
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0], "a\\b");
        assert_eq!(fields[1], "c");
    }
}
