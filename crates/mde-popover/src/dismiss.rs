//! v3.0.3 shared close button — bug fix for "popover won't close."
//!
//! All four popover kinds (start_menu, audio, clock, notifications)
//! previously shipped with Esc as the only close path. Under the
//! `KeyboardInteractivity::OnDemand` layer-shell setting that's
//! unreliable: if focus isn't on the popover surface when the user
//! presses Esc, the keystroke goes elsewhere and the popover stays
//! open. The combination of that + the panel's missing toggle dedup
//! produced the operator-reported "start menu won't close" /
//! "notifications panel won't close" bugs.
//!
//! This module ships a single Iced widget — a small "×" button —
//! that every popover view embeds in its header row so the dismiss
//! path is always visible and always clickable. Esc still works via
//! the popover's existing keyboard subscription; this is the
//! guaranteed-reachable mouse path.
//!
//! Hover state uses the destructive-action red the rest of MDE
//! reserves for close/discard surfaces so the button reads as
//! "this dismisses" without a tooltip.

use iced::widget::{button, text};
use iced::{Background, Border, Color, Element, Padding, Shadow, Theme};

/// Foreground glyph color — matches the panel's `text-helper` muted
/// tone so the button reads as secondary at rest.
const FG_MUTED: Color = Color {
    r: 0.659,
    g: 0.659,
    b: 0.659,
    a: 1.0,
};

/// Hover/pressed glyph color — same `text-01` the popover bodies
/// use so the button gains visual weight on hover.
const FG_HOVER: Color = Color {
    r: 0.957,
    g: 0.957,
    b: 0.957,
    a: 1.0,
};

/// Destructive accent — the same `support-error-inverse` color the
/// admin-menu's destructive actions use. Hover/pressed bg only.
const DESTRUCTIVE: Color = Color {
    r: 0.98,
    g: 0.31,
    b: 0.34,
    a: 1.0,
};

/// Build a close button bound to `on_close`. The popover's `Message`
/// type provides the variant (e.g. `Message::Exit`); this helper is
/// generic over message type so all four popovers share one
/// implementation.
///
/// Returns a small button with a centered "×" glyph. The caller is
/// responsible for placing it in the view (typically in the header
/// row, right-aligned via a `Space::new().width(Length::Fill)`
/// spacer to its left).
pub fn close_button<'a, Msg: Clone + 'a>(on_close: Msg) -> Element<'a, Msg> {
    button(text("×").size(18).color(FG_MUTED))
        .padding(Padding {
            top: 2.0,
            right: 8.0,
            bottom: 2.0,
            left: 8.0,
        })
        .style(close_button_style)
        .on_press(on_close)
        .into()
}

fn close_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, fg) = match status {
        button::Status::Hovered => (
            Some(Background::Color(Color {
                a: 0.20,
                ..DESTRUCTIVE
            })),
            FG_HOVER,
        ),
        button::Status::Pressed => (
            Some(Background::Color(Color {
                a: 0.35,
                ..DESTRUCTIVE
            })),
            FG_HOVER,
        ),
        _ => (None, FG_MUTED),
    };
    button::Style { snap: false,
        background: bg,
        text_color: fg,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The close button must be constructible for every concrete
    /// message type that the popover crates use. This compile-time
    /// test asserts the generic signature accepts a unit-struct
    /// message; the real popover Message enums (which derive
    /// `Clone`) follow the same bound.
    #[test]
    fn close_button_accepts_clone_message() {
        #[derive(Clone)]
        struct UnitMessage;
        let _: Element<'static, UnitMessage> = close_button(UnitMessage);
    }

    /// Status mapping: at rest we use muted FG with no background;
    /// hover lifts FG to text-01 with a 20% destructive overlay;
    /// pressed deepens the overlay to 35%. The visual contract is
    /// "at rest = subtle, hovered = obviously interactable, pressed
    /// = committing." This test pins the alpha values.
    #[test]
    fn hover_uses_20pct_destructive_alpha() {
        let style = close_button_style(&Theme::Dark, button::Status::Hovered);
        let Some(Background::Color(c)) = style.background else {
            panic!("hovered must have a background");
        };
        assert!((c.a - 0.20).abs() < 1e-6);
        assert_eq!(style.text_color, FG_HOVER);
    }

    #[test]
    fn pressed_uses_35pct_destructive_alpha() {
        let style = close_button_style(&Theme::Dark, button::Status::Pressed);
        let Some(Background::Color(c)) = style.background else {
            panic!("pressed must have a background");
        };
        assert!((c.a - 0.35).abs() < 1e-6);
    }

    #[test]
    fn rest_has_no_background() {
        let style = close_button_style(&Theme::Dark, button::Status::Active);
        assert!(style.background.is_none());
        assert_eq!(style.text_color, FG_MUTED);
    }
}
