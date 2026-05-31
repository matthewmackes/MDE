//! VOIP-27 (v6.0) — `mde-voice-hud` Iced + wlr-layer-shell scaffold
//! + idle dialer view.
//!
//! Opens a 420 × 720 layer-shell HUD anchored bottom-right with
//! 16 px right + 56 px bottom clearance per
//! `docs/design/v6.0-pjsip-presence-and-hud.md` §2.5. Renders the
//! topbar (account dot + peer name + registration status), a
//! dialer display + resolved-chip strip, and a 3 × 4 keypad.
//!
//! VOIP-27's acceptance bullets are bench-observable on a clean
//! `cargo run -p mde-voice-hud`:
//!
//! 1. Workspace member registered + `cargo build` exits 0.
//! 2. Layer-shell surface opens at Overlay layer with the
//!    Bottom|Right anchor + margin / size lock.
//! 3. Topbar renders the account dot + peer name placeholder +
//!    presence pip + `Registered · 127.0.0.1:5060` static string.
//! 4. Display field accepts keypad input; the resolved chip
//!    renders mesh / PSTN / partial / invalid per the
//!    `resolve_target` heuristic against the live roster.
//!
//! VOIP-28 wires the Bus subscription for live registration
//! data; VOIP-29 wires the actual PJSIP call placement. This
//! scaffold ships idle-state only.

#![forbid(unsafe_code)]

use iced::widget::{button, column, container, row, text, text_input};
use iced::{Color, Element, Length, Padding, Task, Theme};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

mod recents;
mod resolve;
mod roster;
mod theme;

use resolve::{resolve_target, Resolved};
use roster::{Peer, RosterLoad};

/// VOIP-27 §2.5 size lock — cozy density default.
const WIDTH: u32 = 420;
/// VOIP-27 §2.5 size lock — cozy density default.
const HEIGHT: u32 = 720;
/// VOIP-27 §2.5 margin lock: right=16 px, bottom=56 px (over
/// dock clearance). `LayerShellSettings::margin` order is
/// `(top, right, bottom, left)` per iced_layershell convention.
const MARGIN_RIGHT: i32 = 16;
const MARGIN_BOTTOM: i32 = 56;

/// VOIP-27 — registration status string. Live data wires in
/// VOIP-28; until then the topbar shows the design-bundle
/// reference value verbatim.
const REGISTRATION_PLACEHOLDER: &str = "Registered · 127.0.0.1:5060";

/// VOIP-27 — account-dot initials placeholder. Matches the
/// design bundle's `app.jsx` initial state.
const ACCOUNT_INITIALS: &str = "BT";

/// VOIP-27 — local-peer display name placeholder. VOIP-28 wires
/// the live mded hostname read.
const PEER_NAME_PLACEHOLDER: &str = "Operator";

/// Iced application messages. `#[to_layer_message]` derives the
/// `TryInto<LayershellCustomActions>` bound that the
/// iced_layershell 0.18 builder requires; the attribute also
/// adds variants for layer-shell actions (size, anchor, margin
/// changes etc.) which we don't use directly here but the
/// runtime expects to exist.
// The `to_layer_message` proc-macro injects layer-shell-specific
// variants (size / anchor / margin / etc.) onto the enum but
// doesn't propagate the hand-written doc comments. Allow-list
// scoped to the enum keeps the macro-side warnings quiet while
// preserving the doc requirement on every hand-written variant.
#[allow(missing_docs)]
#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    /// Operator typed into the display text-input or clicked a
    /// keypad button. Carries the full new contents (text-input
    /// emits this on every char; keypad clicks build the new
    /// string in the update handler).
    DialerInputChanged(String),
    /// Keypad button pressed. The handler appends the char to
    /// the current display contents.
    KeypadPressed(char),
    /// Operator clicked the backspace key (or pressed Backspace
    /// on the hardware keyboard). Removes the last char.
    Backspace,
    /// Operator pressed Escape. VOIP-27 ships idle-state only;
    /// Escape exits the process (active-call → minimize-to-
    /// dock-pill ships with VOIP-29). The handler invokes
    /// `Task::done(Message::Exit)` which routes through the
    /// runtime to a graceful exit.
    Escape,
    /// Sentinel that the runtime uses to flag exit. Currently
    /// triggers `std::process::exit(0)` since iced_layershell
    /// 0.18 doesn't expose a clean shutdown API.
    Exit,
}

/// Top-level HUD state.
pub struct VoiceHud {
    /// Current contents of the dialer display field.
    pub dialer_input: String,
    /// Loaded mesh roster — drives the `Resolved::Mesh` lookup
    /// in the resolved-chip rendering.
    pub roster: Vec<Peer>,
}

// ── iced_layershell 0.18 builder-pattern functions ──────────────────────────

fn namespace() -> String {
    "mde-voice-hud".to_string()
}

pub fn update(state: &mut VoiceHud, message: Message) -> Task<Message> {
    match message {
        Message::DialerInputChanged(value) => {
            state.dialer_input = filter_dialer_chars(&value);
        }
        Message::KeypadPressed(c) => {
            if is_dialer_char(c) {
                state.dialer_input.push(c);
            }
        }
        Message::Backspace => {
            state.dialer_input.pop();
        }
        Message::Escape => {
            return Task::done(Message::Exit);
        }
        Message::Exit => {
            std::process::exit(0);
        }
        // `#[to_layer_message]` injects extra variants for
        // layer-shell control actions (anchor / margin / etc.
        // changes). VOIP-27 ships idle-state only — no
        // runtime relayout, so these are unreachable. The
        // wildcard arm keeps the match exhaustive without
        // pulling in the LayershellCustomActions imports.
        _ => {}
    }
    Task::none()
}

pub fn view(state: &VoiceHud) -> Element<'_, Message> {
    container(
        column![
            build_topbar(),
            build_display(state),
            build_keypad(),
        ]
        .spacing(12),
    )
    .padding(Padding::from([16, 16]))
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_: &Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(theme::SURF)),
        ..Default::default()
    })
    .into()
}

fn subscription(_state: &VoiceHud) -> iced::Subscription<Message> {
    use iced::event;
    use iced::keyboard;
    event::listen_with(|event, status, _window| match event {
        iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. })
            if status == event::Status::Ignored =>
        {
            use iced::keyboard::{key::Named, Key};
            match key {
                Key::Named(Named::Escape) => Some(Message::Escape),
                Key::Named(Named::Backspace) => Some(Message::Backspace),
                Key::Character(s) => {
                    let c = s.chars().next()?;
                    if is_dialer_char(c) {
                        Some(Message::KeypadPressed(c))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        _ => None,
    })
}

/// Build the topbar — account dot + peer name + presence pip +
/// registration string. Live data lands in VOIP-28; the strings
/// here are operator-visible placeholders so the bench
/// observable view matches §3.5 mde-voice-hud surface.
fn build_topbar<'a>() -> Element<'a, Message> {
    let account_dot = container(text(ACCOUNT_INITIALS).size(13.0).color(theme::ON_PRIMARY))
        .style(|_: &Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(theme::PRIMARY)),
            border: iced::Border {
                radius: iced::border::Radius::from(16.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .width(Length::Fixed(32.0))
        .height(Length::Fixed(32.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    let presence_pip = container(iced::widget::Space::new())
        .style(|_: &Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(theme::PRESENCE_AVAILABLE)),
            border: iced::Border {
                radius: iced::border::Radius::from(4.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .width(Length::Fixed(8.0))
        .height(Length::Fixed(8.0));

    let name_col = column![
        text(PEER_NAME_PLACEHOLDER).size(14.0).color(theme::ON_SURF),
        row![
            presence_pip,
            iced::widget::space().width(Length::Fixed(6.0)),
            text(REGISTRATION_PLACEHOLDER).size(11.0).color(theme::ON_SURF_VAR),
        ]
        .align_y(iced::Alignment::Center),
    ]
    .spacing(2);

    row![
        account_dot,
        iced::widget::space().width(Length::Fixed(12.0)),
        name_col,
    ]
    .align_y(iced::Alignment::Center)
    .into()
}

/// Build the display + resolved-chip strip. The text-input
/// receives keypad/keyboard input; the chip to its right
/// renders the `resolve_target` classification.
fn build_display<'a>(state: &VoiceHud) -> Element<'a, Message> {
    let display = text_input("Type 1NNN for mesh, 9 + E.164 for PSTN", &state.dialer_input)
        .on_input(Message::DialerInputChanged)
        .size(20.0)
        .padding(Padding::from([10, 12]))
        .width(Length::Fill);

    let resolved = resolve_target(&state.dialer_input, &state.roster);
    let chip = build_resolved_chip(&resolved);

    column![
        container(display)
            .style(|_: &Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(theme::SURF_C)),
                border: iced::Border {
                    radius: iced::border::Radius::from(8.0),
                    color: theme::OUTLINE_VAR,
                    width: 1.0,
                },
                ..Default::default()
            }),
        chip,
    ]
    .spacing(8)
    .into()
}

/// Build the resolved-classification chip for the current
/// display contents. One pill per state, colored by category.
fn build_resolved_chip<'a>(resolved: &Resolved) -> Element<'a, Message> {
    let (label, color) = resolved_chip_label_and_color(resolved);
    container(text(label).size(12.0).color(Color::WHITE))
        .style(move |_: &Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(color)),
            border: iced::Border {
                radius: iced::border::Radius::from(12.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .padding(Padding::from([4, 10]))
        .into()
}

/// Map a `Resolved` state to its chip label + tint.
#[must_use]
pub fn resolved_chip_label_and_color(resolved: &Resolved) -> (String, Color) {
    match resolved {
        Resolved::Empty => (
            "type 1NNN or 9+E.164".to_string(),
            theme::SURF_C_HI,
        ),
        Resolved::Mesh { name, .. } => (
            format!("mesh · {name}"),
            theme::PRESENCE_AVAILABLE,
        ),
        Resolved::MeshUnknown => (
            "mesh · not in roster".to_string(),
            theme::ERROR,
        ),
        Resolved::MeshPartial { remaining } => (
            format!("{remaining} more digit{}", if *remaining == 1 { "" } else { "s" }),
            theme::INFO,
        ),
        Resolved::Pstn { formatted } => (
            format!("PSTN · {formatted}"),
            theme::PRIMARY,
        ),
        Resolved::PstnPartial { remaining } => (
            format!("{remaining} more digit{} via Vitelity", if *remaining == 1 { "" } else { "s" }),
            theme::INFO,
        ),
        Resolved::Invalid => (
            "invalid prefix".to_string(),
            theme::ERROR,
        ),
    }
}

/// Build the 3 × 4 keypad. Numeric 1-9 + *, 0, # in the standard
/// phone-pad layout. Each button click appends to the dialer
/// input.
fn build_keypad<'a>() -> Element<'a, Message> {
    let rows: [[char; 3]; 4] = [
        ['1', '2', '3'],
        ['4', '5', '6'],
        ['7', '8', '9'],
        ['*', '0', '#'],
    ];
    let mut col: Vec<Element<'a, Message>> = Vec::with_capacity(4);
    for line in rows {
        let mut row_buf: Vec<Element<'a, Message>> = Vec::with_capacity(3);
        for c in line {
            row_buf.push(keypad_button(c));
        }
        col.push(
            row(row_buf)
                .spacing(8)
                .into(),
        );
    }
    column(col).spacing(8).into()
}

/// One 3 × 4 keypad button. Renders the digit/symbol on a
/// surface-container background; click fires
/// `Message::KeypadPressed(c)`.
fn keypad_button<'a>(c: char) -> Element<'a, Message> {
    button(
        container(text(c.to_string()).size(22.0).color(theme::ON_SURF))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .on_press(Message::KeypadPressed(c))
    .width(Length::Fill)
    .height(Length::Fixed(56.0))
    .style(|_: &Theme, _status| iced::widget::button::Style {
        background: Some(iced::Background::Color(theme::SURF_C)),
        text_color: theme::ON_SURF,
        border: iced::Border {
            radius: iced::border::Radius::from(8.0),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

// ── Pure helpers ────────────────────────────────────────────────────────

/// `true` if `c` is a valid dialer character: ASCII digit, `*`,
/// or `#`. Keypad + keyboard inputs filter through this to keep
/// the display field strictly dialer-shaped.
#[must_use]
pub fn is_dialer_char(c: char) -> bool {
    c.is_ascii_digit() || c == '*' || c == '#'
}

/// Strip non-dialer characters from a pasted string. Operator
/// pasting `"(415) 555-1234"` into the field should resolve to
/// `"4155551234"`. Spaces, parens, dashes all drop.
#[must_use]
pub fn filter_dialer_chars(s: &str) -> String {
    s.chars().filter(|c| is_dialer_char(*c)).collect()
}

// ── Main ────────────────────────────────────────────────────────────────

fn main() -> iced_layershell::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("MDE_VOICE_HUD_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("mde_voice_hud=info,warn")),
        )
        .json()
        .init();

    iced_layershell::application(
        || {
            let RosterLoad { peers, source } = roster::load();
            tracing::info!(roster_count = peers.len(), ?source, "voice-hud: roster loaded");
            VoiceHud {
                dialer_input: String::new(),
                roster: peers,
            }
        },
        namespace,
        update,
        view,
    )
    .subscription(subscription)
    .settings(Settings {
        id: Some("mde-voice-hud".to_string()),
        layer_settings: LayerShellSettings {
            size: Some((WIDTH, HEIGHT)),
            exclusive_zone: 0,
            // (top, right, bottom, left) per iced_layershell convention.
            margin: (0, MARGIN_RIGHT, MARGIN_BOTTOM, 0),
            anchor: Anchor::Bottom | Anchor::Right,
            // §2.5 Layer lock: Overlay (above normal windows; below lock).
            layer: Layer::Overlay,
            // §2.5 keyboard lock: OnDemand — focus on click.
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        },
        ..Default::default()
    })
    .run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roster::Peer;

    fn sample_roster() -> Vec<Peer> {
        vec![Peer {
            ext: "1003".to_string(),
            name: "alice".to_string(),
            role: "GUI".to_string(),
            presence: "available".to_string(),
            lan: true,
            hint: "Alice's ThinkPad".to_string(),
        }]
    }

    fn make_hud() -> VoiceHud {
        VoiceHud { dialer_input: String::new(), roster: vec![] }
    }

    #[test]
    fn is_dialer_char_accepts_digits_star_hash() {
        for c in '0'..='9' {
            assert!(is_dialer_char(c), "digit {c} should be a dialer char");
        }
        assert!(is_dialer_char('*'));
        assert!(is_dialer_char('#'));
        // Letters + whitespace + punctuation are not dialer chars.
        assert!(!is_dialer_char('a'));
        assert!(!is_dialer_char(' '));
        assert!(!is_dialer_char('-'));
        assert!(!is_dialer_char('+'));
    }

    #[test]
    fn filter_dialer_chars_strips_formatting() {
        assert_eq!(filter_dialer_chars("(415) 555-1234"), "4155551234");
        assert_eq!(filter_dialer_chars("9 800 555 0199"), "98005550199");
        assert_eq!(filter_dialer_chars("1003"), "1003");
        assert_eq!(filter_dialer_chars(""), "");
        // Letters dropped, digits preserved.
        assert_eq!(filter_dialer_chars("call 1003 now"), "1003");
    }

    #[test]
    fn resolved_chip_empty_state() {
        let (label, _color) = resolved_chip_label_and_color(&Resolved::Empty);
        assert_eq!(label, "type 1NNN or 9+E.164");
    }

    #[test]
    fn resolved_chip_mesh_with_peer_name() {
        let roster = sample_roster();
        let resolved = resolve_target("1003", &roster);
        let (label, _color) = resolved_chip_label_and_color(&resolved);
        assert!(label.starts_with("mesh · "));
        assert!(label.contains("alice"));
    }

    #[test]
    fn resolved_chip_mesh_unknown() {
        let roster = sample_roster();
        // 1999 doesn't exist in the sample roster.
        let resolved = resolve_target("1999", &roster);
        let (label, _color) = resolved_chip_label_and_color(&resolved);
        assert_eq!(label, "mesh · not in roster");
    }

    #[test]
    fn resolved_chip_mesh_partial_singular_and_plural() {
        let roster = sample_roster();
        // 1 char → "3 more digits".
        let (label, _) = resolved_chip_label_and_color(&resolve_target("1", &roster));
        assert_eq!(label, "3 more digits");
        // 3 chars → "1 more digit" (singular).
        let (label, _) = resolved_chip_label_and_color(&resolve_target("100", &roster));
        assert_eq!(label, "1 more digit");
    }

    #[test]
    fn resolved_chip_pstn_formatted() {
        let roster = sample_roster();
        // `9` + 11 digits.
        let resolved = resolve_target("914155551234", &roster);
        let (label, _) = resolved_chip_label_and_color(&resolved);
        assert!(label.starts_with("PSTN · "));
    }

    #[test]
    fn resolved_chip_pstn_partial_singular_and_plural() {
        let roster = sample_roster();
        // 1 digit after 9 → "10 more digits via Vitelity".
        let (label, _) = resolved_chip_label_and_color(&resolve_target("91", &roster));
        assert_eq!(label, "10 more digits via Vitelity");
        // 10 digits after 9 → "1 more digit via Vitelity".
        let (label, _) = resolved_chip_label_and_color(&resolve_target("94155551234", &roster));
        assert_eq!(label, "1 more digit via Vitelity");
    }

    #[test]
    fn resolved_chip_invalid_prefix() {
        let roster = sample_roster();
        // Prefix not in [1, 9] range.
        let resolved = resolve_target("5555", &roster);
        let (label, _) = resolved_chip_label_and_color(&resolved);
        assert_eq!(label, "invalid prefix");
    }

    #[test]
    fn voice_hud_keypad_pressed_appends_to_input() {
        let mut hud = make_hud();
        assert_eq!(hud.dialer_input, "");
        for c in "1003".chars() {
            let _ = update(&mut hud, Message::KeypadPressed(c));
        }
        assert_eq!(hud.dialer_input, "1003");
    }

    #[test]
    fn voice_hud_keypad_rejects_non_dialer_char() {
        let mut hud = make_hud();
        let _ = update(&mut hud, Message::KeypadPressed('a'));
        assert_eq!(hud.dialer_input, "");
    }

    #[test]
    fn voice_hud_backspace_removes_last_char() {
        let mut hud = make_hud();
        for c in "1003".chars() {
            let _ = update(&mut hud, Message::KeypadPressed(c));
        }
        let _ = update(&mut hud, Message::Backspace);
        assert_eq!(hud.dialer_input, "100");
        // Backspace on empty input is a no-op.
        for _ in 0..10 {
            let _ = update(&mut hud, Message::Backspace);
        }
        assert_eq!(hud.dialer_input, "");
    }

    #[test]
    fn voice_hud_dialer_input_changed_filters_input() {
        let mut hud = make_hud();
        // Paste a formatted number — non-dialer chars drop.
        let _ = update(&mut hud, Message::DialerInputChanged("(415) 555-1234".to_string()));
        assert_eq!(hud.dialer_input, "4155551234");
    }

    #[test]
    fn layer_settings_match_design_lock() {
        // Compile-time + const-eval check that VOIP-27's §2.5
        // values are wired correctly.
        assert_eq!(WIDTH, 420);
        assert_eq!(HEIGHT, 720);
        assert_eq!(MARGIN_RIGHT, 16);
        assert_eq!(MARGIN_BOTTOM, 56);
        assert_eq!(REGISTRATION_PLACEHOLDER, "Registered · 127.0.0.1:5060");
        assert_eq!(ACCOUNT_INITIALS, "BT");
    }
}
