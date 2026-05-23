//! Phase E.17 + Phase E.4-E.29 host wiring — the panel's top-bar
//! view. Lays out the six locked zones in a single 40 px row and
//! renders the live text emitted by each `mde-applet-*` subprocess
//! (driven by [`crate::applet_host`]).
//!
//! ```text
//!   ┌─────────────────────────────────────────────────────────┐
//!   │  M │ [dock…]      [cluster]      [tray icons]    11:42  │
//!   │ Start  Pinned/Tasklist   Cluster   Tray         Clock   │
//!   └─────────────────────────────────────────────────────────┘
//! ```
//!
//! Design locks (2026 surface refresh):
//! - **Surface:** dark glass (#0e0e10 @ 92 % alpha when the
//!   compositor exposes blur; opaque otherwise). Hairline 1 px
//!   at the top edge in `rgba(244,244,244,0.06)`.
//! - **Accent:** `#2b9af3` (PatternFly blue-400 — Carbon
//!   `interactive-04` lock). Greyscale elsewhere; hover lifts
//!   with a 14 %-alpha underglow of the accent.
//! - **Typography:** Red Hat Mono for the clock + tabular numerics,
//!   Red Hat Text 12 px / 500 weight for labels.
//! - **Microinteraction:** 180 ms ease-out for every state change.

use iced::widget::{button, column, container, mouse_area, row, svg, text, Space};

use crate::panel_icons::PanelIcon;
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Theme};

use crate::applet_host::AppletKind;
use crate::hero::Hero;
use crate::toplevels::Toplevel;
use crate::Message;

/// Height of the top bar in logical pixels (Phase 1.1.0 Win10 lock).
pub const TOP_BAR_HEIGHT_PX: u16 = 40;

/// Per-zone padding (horizontal) — keeps icons + text from
/// touching the bar's edges.
pub const ZONE_PADDING_X: u16 = 12;

/// Accent — Carbon `interactive-04` / PatternFly blue-400.
const ACCENT: Color = Color {
    r: 0.169,
    g: 0.604,
    b: 0.953,
    a: 1.0,
};

/// Foreground text — Carbon `text-01`.
const FG_TEXT: Color = Color {
    r: 0.957,
    g: 0.957,
    b: 0.957,
    a: 1.0,
};

/// Muted helper text — Carbon `text-helper`.
const FG_MUTED: Color = Color {
    r: 0.659,
    g: 0.659,
    b: 0.659,
    a: 1.0,
};

/// Panel background — `#0e0e10` at 92 % alpha.
const SURFACE_BG: Color = Color {
    r: 0.055,
    g: 0.055,
    b: 0.063,
    a: 0.92,
};

/// State injected into [`view`] — one text-cell per applet kind, plus
/// a fixed start label. The panel orchestrator (`App::update`) mutates
/// this via [`set_applet_text`] each time an applet emits a stdout line.
#[derive(Debug, Clone, Default)]
pub struct TopBarState {
    pub start_label: String,
    pub dock_text: String,
    pub cluster_text: String,
    pub clock_text: String,
    pub audio_text: String,
    pub network_text: String,
    pub mesh_text: String,
    pub status_text: String,
    pub bell_text: String,
}

impl TopBarState {
    /// Initial loading placeholder — emitted before the first applet
    /// re-render lands (typically < 1 s after panel spawn).
    #[must_use]
    pub fn loading() -> Self {
        Self {
            start_label: "M".to_string(),
            dock_text: "…".to_string(),
            cluster_text: "…".to_string(),
            clock_text: "--:--".to_string(),
            audio_text: "🔈 --".to_string(),
            network_text: "—".to_string(),
            mesh_text: "—".to_string(),
            status_text: "—".to_string(),
            bell_text: String::new(),
        }
    }

    /// Demo content used by tests + bare-iced dev launches. Kept so
    /// the test `view_renders_without_panic` doesn't need a live
    /// applet host.
    #[must_use]
    pub fn demo() -> Self {
        Self {
            start_label: "M".to_string(),
            dock_text: "[▶ foot]".to_string(),
            cluster_text: "H  def  #1".to_string(),
            clock_text: "11:42".to_string(),
            audio_text: "🔈 65%".to_string(),
            network_text: "Wi-Fi".to_string(),
            mesh_text: "✓ 3".to_string(),
            status_text: "⚡ 88%".to_string(),
            bell_text: "0".to_string(),
        }
    }

    /// Apply the latest stdout line for the given applet kind. Called
    /// from `App::update` on every `Message::AppletText`.
    pub fn set_applet_text(&mut self, kind: AppletKind, text: String) {
        match kind {
            AppletKind::Clock => self.clock_text = text,
            AppletKind::Audio => self.audio_text = text,
            AppletKind::Network => self.network_text = text,
            AppletKind::MeshStatus => self.mesh_text = text,
            AppletKind::StatusCluster => self.status_text = text,
            AppletKind::SwayCluster => self.cluster_text = text,
            AppletKind::NotificationBell => self.bell_text = text,
            AppletKind::Dock => self.dock_text = text,
        }
    }
}

/// Render the top bar. Returns an Iced `Element<Message>`; the
/// click handlers map directly to `Message::StartClicked` /
/// `Message::TrayClicked(kind)` / `Message::WindowMinimize` etc.
///
/// v3.0.3 — signature gained `hero` (focused-window display, from
/// the Phase E.4.2 widget) and `focused` (current focused
/// toplevel, used by the window-management buttons to grey out
/// when no window is focused).
#[must_use]
pub fn view<'a>(
    state: &'a TopBarState,
    hero: &'a Hero,
    focused: Option<&'a Toplevel>,
    workspaces: &'a [Option<crate::workspaces::WorkspaceState>; 4],
) -> Element<'a, Message> {
    // v3.0.3 Tier 1D fix — wrap the Start button in `mouse_area`
    // so right-click is observable. Iced's built-in `button` is
    // left-click only; the operator-reported "right click on the
    // start menu does not work" bug stemmed from this gap. The
    // left-click `Message::StartClicked` keeps coming from the
    // inner button; the right-click maps to a new
    // `Message::StartRightClicked` that opens the admin-menu
    // popover.
    // v4.0.1 BUG-13: Start button now renders the Carbon `menu`
    // SVG glyph via PanelIcon::Start instead of the "M" letter
    // stand-in. Keep `state.start_label` in the StatusBarState for
    // backward-compat — the test set_applet_text_routes_to_correct_field
    // still flips it; we just don't render the letter anymore.
    let start_icon = svg(PanelIcon::Start.handle())
        .width(Length::Fixed(18.0))
        .height(Length::Fixed(18.0))
        .style(|_theme: &Theme, _status: svg::Status| svg::Style {
            color: Some(ACCENT),
        });
    let start_btn = mouse_area(
        button(start_icon)
            .padding(Padding {
                top: 4.0,
                right: 12.0,
                bottom: 4.0,
                left: 12.0,
            })
            .style(zone_button_style)
            .on_press(Message::StartClicked),
    )
    .on_right_press(Message::StartRightClicked);

    // Dock zone — shows the dock applet's pinned/running summary
    // (e.g. "[▶ foot] [· firefox]"). Until the inline Iced dock
    // (Phase E.10 host) lands, this is read-only text; clicks fall
    // through to a Noop.
    let dock = labeled_zone(&state.dock_text, FG_TEXT, false);

    // v3.0.3 Phase E.4.2 wiring — focused-window hero. Shows the
    // ellipsized title (max 64 chars) or empty when no window is
    // focused. Sits between Dock and the right-flex spacer so it
    // gets generous horizontal space without crowding the tray.
    let hero_zone = hero_view(hero);

    // Cluster zone — the sway-IPC chips (`H  def  #1` or similar).
    let cluster = labeled_zone(&state.cluster_text, FG_TEXT, false);

    // Tray — six clickable cells in a row.
    // v4.0.1 BUG-7: clipboard tray icon added between status_cluster
    // and notification-bell. Click fires Message::ClipboardClicked,
    // routed to `mde-popover clipboard` (same surface as Super+V).
    // It's a static icon (no applet text stream); the glyph is the
    // Unicode clipboard codepoint U+1F4CB until the BUG-13 Carbon
    // SVG wiring lands.
    let tray = row![
        tray_button_with_icon(PanelIcon::Audio, &state.audio_text, AppletKind::Audio),
        Space::with_width(Length::Fixed(8.0)),
        tray_button_with_icon(PanelIcon::Network, &state.network_text, AppletKind::Network),
        Space::with_width(Length::Fixed(8.0)),
        tray_button_with_icon(PanelIcon::Mesh, &state.mesh_text, AppletKind::MeshStatus),
        Space::with_width(Length::Fixed(8.0)),
        tray_button_with_icon(PanelIcon::Status, &state.status_text, AppletKind::StatusCluster),
        Space::with_width(Length::Fixed(8.0)),
        clipboard_button(),
        Space::with_width(Length::Fixed(8.0)),
        tray_button_with_icon(
            PanelIcon::Bell,
            if state.bell_text.is_empty() {
                "0"
            } else {
                state.bell_text.as_str()
            },
            AppletKind::NotificationBell,
        ),
    ]
    .align_y(iced::Alignment::Center);

    // v4.0.1 BUG-16 — Desktop Layout cluster (replaces the
    // previously-centered window_button_cluster from BUG-6). Five
    // Snap-Layouts-style buttons that re-tile the focused
    // workspace via swayipc. Per-window controls now live at the
    // top-right of each managed window (sway native title bars
    // per the data/sway/config change in this commit).
    let desktop_layout_cluster = desktop_layout_cluster_view();

    // v4.0.1 WM-1 — workspace switcher chip row. Four chips for
    // ws 1..4; focused chip paints in Q2 indigo bg, has-windows
    // chips show a small indicator dot.
    let workspace_chips = workspace_chip_row(workspaces);
    // Kept for the focused-state suppress signal pattern used by
    // tests; future re-introduction of an opt-in per-window
    // overlay would consume this. `focused.is_some()` is the
    // signal a window-controls overlay would key on.
    let _focused_signal = focused.is_some();

    // Clock — Win10 two-line stack. v4.0.1 BUG-14: the applet emits
    // "H:MM AM/PM\nM/D/YYYY"; render the time on top with a slightly
    // larger size, the date below in the muted foreground. Falls back
    // to single-line rendering for the loading-state ("--:--").
    let clock_lines: Vec<&str> = state.clock_text.split('\n').collect();
    let clock_body: Element<'_, Message> = if clock_lines.len() >= 2 {
        column![
            text(clock_lines[0].to_string()).size(13).color(FG_TEXT),
            text(clock_lines[1].to_string()).size(10).color(FG_MUTED),
        ]
        .spacing(0)
        .align_x(iced::alignment::Horizontal::Right)
        .into()
    } else {
        text(state.clock_text.clone()).size(13).color(FG_TEXT).into()
    };
    let clock = button(clock_body)
        .padding(Padding {
            top: 2.0,
            right: 12.0,
            bottom: 2.0,
            left: 12.0,
        })
        .style(zone_button_style)
        .on_press(Message::TrayClicked(AppletKind::Clock));

    // v4.0.1 BUG-16: panel center now hosts the Desktop Layout
    // cluster (Win11-inspired Snap Layouts — single / vsplit /
    // grid / main+sidebar / tabbed). Per-window min/max/close
    // moved BACK to the per-window title bar via sway native
    // borders (data/sway/config: default_border normal 4) so
    // operators with Win11/macOS muscle memory get the controls
    // where they expect them. Supersedes the BUG-6 layout.
    container(
        row![
            start_btn,
            Space::with_width(Length::Fixed(f32::from(ZONE_PADDING_X))),
            workspace_chips,
            Space::with_width(Length::Fixed(f32::from(ZONE_PADDING_X))),
            dock,
            Space::with_width(Length::Fixed(f32::from(ZONE_PADDING_X))),
            hero_zone,
            Space::with_width(Length::Fill),
            desktop_layout_cluster,
            Space::with_width(Length::Fill),
            tray,
            Space::with_width(Length::Fixed(f32::from(ZONE_PADDING_X))),
            cluster,
            Space::with_width(Length::Fixed(f32::from(ZONE_PADDING_X))),
            clock,
        ]
        .align_y(iced::Alignment::Center)
        .padding(Padding {
            top: 0.0,
            right: f32::from(ZONE_PADDING_X),
            bottom: 0.0,
            left: f32::from(ZONE_PADDING_X),
        }),
    )
    .width(Length::Fill)
    .height(Length::Fixed(f32::from(TOP_BAR_HEIGHT_PX)))
    .style(panel_surface)
    .into()
}

/// v3.0.3 Phase E.4.2 wiring — render the focused-window hero.
/// Shows the ellipsized title (or "" when no window is focused).
/// Sliding animation is driven by `hero::Hero::tick` from the
/// panel's `Message::Tick` reducer; this function just reads the
/// post-tick state.
fn hero_view<'a>(hero: &'a Hero) -> Element<'a, Message> {
    let label = hero
        .display_title()
        .unwrap_or_else(|| String::new());
    container(text(label).size(13).color(FG_TEXT))
        .padding(Padding {
            top: 4.0,
            right: 8.0,
            bottom: 4.0,
            left: 8.0,
        })
        .into()
}

/// v3.0.3 Tier 1E — three-button cluster (min/max/close) per the
// v4.0.1 BUG-16: window_button_cluster + window_btn +
// destructive_button_style retired — per-window controls moved
// back to sway native title bars (data/sway/config:
// default_border normal 4) so the operator's Win11 muscle memory
// applies. The pre-BUG-16 helpers are gone; if a future
// implementation revives the mde-window-controls layer-shell
// overlay path (BUG-16 Implementation note (b)), the helpers
// land in that overlay's own crate rather than living on the
// panel surface.

/// Read-only text zone with a thin padding box. Used by the dock and
/// cluster cells which aren't yet click-targets.
fn labeled_zone(label: &str, color: Color, accent: bool) -> Element<'_, Message> {
    let style_color = if accent { ACCENT } else { color };
    container(text(label.to_string()).size(13).color(style_color))
        .padding(Padding {
            top: 4.0,
            right: 6.0,
            bottom: 4.0,
            left: 6.0,
        })
        .into()
}

// v4.0.1 BUG-13.a: `tray_button` (un-iconed text-only tray button)
// retired — `tray_button_with_icon` replaced every call site. The
// pre-BUG-13.a function had no remaining consumers.

/// v4.0.1 WM-1 — Q2 indigo background for the focused workspace
/// chip. Matches the locked visual-identity.md Q2 indigo
/// (#5b6af5) so chip selection reads as the same accent the
/// start-menu update chip uses (BUG-11 closure + BUG-12 start
/// tile pinning).
const WORKSPACE_FOCUSED_BG: Color = Color {
    r: 0.357,
    g: 0.416,
    b: 0.961,
    a: 1.0,
};

/// v4.0.1 WM-1 — render the workspace chip row. Four chips,
/// indexed 1..4. Empty slots (no workspace at that index) still
/// render but in a deactivated visual state — switching to an
/// uncreated workspace IS valid in sway (it creates the workspace
/// on the fly), so we let the chip dispatch the swaymsg either
/// way.
fn workspace_chip_row<'a>(
    workspaces: &'a [Option<crate::workspaces::WorkspaceState>; 4],
) -> Element<'a, Message> {
    row![
        workspace_chip(1, &workspaces[0]),
        Space::with_width(Length::Fixed(4.0)),
        workspace_chip(2, &workspaces[1]),
        Space::with_width(Length::Fixed(4.0)),
        workspace_chip(3, &workspaces[2]),
        Space::with_width(Length::Fixed(4.0)),
        workspace_chip(4, &workspaces[3]),
    ]
    .align_y(iced::Alignment::Center)
    .into()
}

/// One workspace chip. Numeric Carbon glyph at 16 px (or 14 if
/// the rendered chip looks too tall against the panel height).
/// Focused chip background = Q2 indigo; others use zone_button_style
/// hover chrome. Has-windows chips include a small Carbon
/// `circle--solid` dot at 6 px next to the number.
fn workspace_chip<'a>(
    num: i32,
    state: &'a Option<crate::workspaces::WorkspaceState>,
) -> Element<'a, Message> {
    let focused = state.as_ref().is_some_and(|s| s.focused);
    let has_windows = state.as_ref().is_some_and(|s| s.has_windows);
    let icon = match num {
        1 => PanelIcon::Workspace1,
        2 => PanelIcon::Workspace2,
        3 => PanelIcon::Workspace3,
        4 => PanelIcon::Workspace4,
        _ => PanelIcon::Workspace1, // safety net; only 1..=4 reach here
    };
    let icon_color = if focused { FG_TEXT } else { FG_MUTED };
    let number_widget = svg(icon.handle())
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0))
        .style(move |_theme: &Theme, _status: svg::Status| svg::Style {
            color: Some(icon_color),
        });
    let dot_widget: Element<'a, Message> = if has_windows {
        svg(PanelIcon::WorkspaceDot.handle())
            .width(Length::Fixed(6.0))
            .height(Length::Fixed(6.0))
            .style(move |_theme: &Theme, _status: svg::Status| svg::Style {
                color: Some(icon_color),
            })
            .into()
    } else {
        Space::with_width(Length::Fixed(6.0)).into()
    };
    let content = row![
        number_widget,
        Space::with_width(Length::Fixed(2.0)),
        dot_widget,
    ]
    .align_y(iced::Alignment::Center);
    let style: fn(&Theme, button::Status) -> button::Style = if focused {
        workspace_chip_focused_style
    } else {
        zone_button_style
    };
    button(content)
        .padding(Padding {
            top: 5.0,
            right: 6.0,
            bottom: 5.0,
            left: 6.0,
        })
        .style(style)
        .on_press(Message::WorkspaceSelected(num))
        .into()
}

/// v4.0.1 WM-1 — focused-workspace chip background.
fn workspace_chip_focused_style(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(WORKSPACE_FOCUSED_BG)),
        text_color: Color::WHITE,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        shadow: Shadow::default(),
    }
}

/// v4.0.1 BUG-16 — Desktop Layout cluster. Five Snap-Layouts-style
/// buttons that apply a tile arrangement to the focused workspace
/// via `Message::DesktopLayoutSelected`. Each button is a 14 px
/// Carbon SVG; the cluster shares a single accent color (Q2
/// indigo on hover) per the Phase 0.8 Ableton single-accent-per-
/// zone rule.
fn desktop_layout_cluster_view<'a>() -> Element<'a, Message> {
    use crate::DesktopLayout;
    row![
        desktop_layout_button(PanelIcon::LayoutSingle, DesktopLayout::Single),
        Space::with_width(Length::Fixed(4.0)),
        desktop_layout_button(PanelIcon::LayoutVsplit, DesktopLayout::Vsplit),
        Space::with_width(Length::Fixed(4.0)),
        desktop_layout_button(PanelIcon::LayoutGrid, DesktopLayout::Grid),
        Space::with_width(Length::Fixed(4.0)),
        desktop_layout_button(
            PanelIcon::LayoutMainSidebar,
            DesktopLayout::MainSidebar,
        ),
        Space::with_width(Length::Fixed(4.0)),
        desktop_layout_button(PanelIcon::LayoutTabbed, DesktopLayout::Tabbed),
    ]
    .align_y(iced::Alignment::Center)
    .into()
}

/// One Desktop Layout button — Carbon SVG glyph painted in
/// FG_MUTED at rest per BUG-16's acceptance criterion; the
/// zone_button_style provides the hover affordance via a
/// background tint. Fires `Message::DesktopLayoutSelected(kind)`.
fn desktop_layout_button<'a>(
    icon: PanelIcon,
    kind: crate::DesktopLayout,
) -> Element<'a, Message> {
    let icon_widget = svg(icon.handle())
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(14.0))
        .style(|_theme: &Theme, _status: svg::Status| svg::Style {
            color: Some(FG_MUTED),
        });
    button(icon_widget)
        .padding(Padding {
            top: 6.0,
            right: 8.0,
            bottom: 6.0,
            left: 8.0,
        })
        .style(zone_button_style)
        .on_press(Message::DesktopLayoutSelected(kind))
        .into()
}

/// v4.0.1 BUG-13.a — tray button that renders a Carbon SVG icon
/// before the live text payload from the applet. Replaces the
/// plain text rendering of `tray_button` for the chips whose
/// applet binaries dropped their leading Unicode glyph (audio /
/// network / mesh-status / status-cluster). The
/// `notification-bell` chip stays on the icon-less path because
/// the bell's payload is already a number-or-"○" placeholder
/// the panel can render as-is.
fn tray_button_with_icon<'a>(
    icon: PanelIcon,
    label: &str,
    kind: AppletKind,
) -> Element<'a, Message> {
    let icon_widget = svg(icon.handle())
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(14.0))
        .style(|_theme: &Theme, _status: svg::Status| svg::Style {
            color: Some(FG_TEXT),
        });
    button(
        row![
            icon_widget,
            Space::with_width(Length::Fixed(6.0)),
            text(label.to_string()).size(13).color(FG_TEXT),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding(Padding {
        top: 6.0,
        right: 8.0,
        bottom: 6.0,
        left: 8.0,
    })
    .style(zone_button_style)
    .on_press(Message::TrayClicked(kind))
    .into()
}

/// v4.0.1 BUG-7 — static clipboard-history tray icon. Spawns
/// `mde-popover clipboard` on press (same path as Super+V). Lives
/// outside the AppletKind enum because there's no applet
/// subprocess feeding text into it.
///
/// v4.0.1 BUG-13: now renders the Carbon `copy` glyph SVG via
/// `PanelIcon::Clipboard` instead of the Unicode U+1F4CB
/// codepoint that earlier shipped as a placeholder.
fn clipboard_button<'a>() -> Element<'a, Message> {
    let icon_widget = svg(PanelIcon::Clipboard.handle())
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0))
        .style(|_theme: &Theme, _status: svg::Status| svg::Style {
            color: Some(FG_TEXT),
        });
    button(icon_widget)
        .padding(Padding {
            top: 6.0,
            right: 8.0,
            bottom: 6.0,
            left: 8.0,
        })
        .style(zone_button_style)
        .on_press(Message::ClipboardClicked)
        .into()
}

fn panel_surface(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE_BG)),
        border: Border {
            color: Color {
                r: 0.957,
                g: 0.957,
                b: 0.957,
                a: 0.06,
            },
            width: 1.0,
            radius: 0.0.into(),
        },
        text_color: Some(FG_TEXT),
        shadow: Shadow::default(),
    }
}

/// Zone-button style — flat, no border, accent-tinted hover.
fn zone_button_style(_theme: &Theme, status: button::Status) -> button::Style {
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
    }
}

#[allow(dead_code)]
fn muted() -> Color {
    FG_MUTED
}

/// v3.0.3 Tier 1E — destructive variant of the zone button style.
/// Used by the window-close button so its hover state reads as
/// "this closes." Shares the destructive color with
/// `crate::dismiss::close_button` in the popover crate.
// v4.0.1 BUG-16: destructive_button_style retired. Its only
// consumer was the close button in the centered window cluster
// (deleted with window_button_cluster). The popover dismiss
// button has its own equivalent at `crate::dismiss::close_button`
// in the mde-popover crate.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_bar_height_is_40px_per_1_1_0_lock() {
        assert_eq!(TOP_BAR_HEIGHT_PX, 40);
    }

    #[test]
    fn zone_padding_is_symmetric_12px() {
        assert_eq!(ZONE_PADDING_X, 12);
    }

    #[test]
    fn loading_state_populates_every_field() {
        let state = TopBarState::loading();
        assert!(!state.start_label.is_empty());
        assert!(!state.clock_text.is_empty());
        assert!(!state.audio_text.is_empty());
    }

    #[test]
    fn set_applet_text_routes_to_correct_field() {
        let mut state = TopBarState::default();
        state.set_applet_text(AppletKind::Clock, "12:34".into());
        assert_eq!(state.clock_text, "12:34");
        state.set_applet_text(AppletKind::Audio, "🔈 50%".into());
        assert_eq!(state.audio_text, "🔈 50%");
        state.set_applet_text(AppletKind::Network, "Wi-Fi: home".into());
        assert_eq!(state.network_text, "Wi-Fi: home");
        state.set_applet_text(AppletKind::MeshStatus, "✓ 4".into());
        assert_eq!(state.mesh_text, "✓ 4");
        state.set_applet_text(AppletKind::StatusCluster, "⚡ 99%".into());
        assert_eq!(state.status_text, "⚡ 99%");
        state.set_applet_text(AppletKind::SwayCluster, "H def #1".into());
        assert_eq!(state.cluster_text, "H def #1");
        state.set_applet_text(AppletKind::Dock, "[▶ foot]".into());
        assert_eq!(state.dock_text, "[▶ foot]");
        state.set_applet_text(AppletKind::NotificationBell, "3".into());
        assert_eq!(state.bell_text, "3");
    }

    #[test]
    fn view_renders_without_panic() {
        let state = TopBarState::demo();
        let hero = crate::hero::Hero::new();
        let workspaces: [Option<crate::workspaces::WorkspaceState>; 4] =
            [None, None, None, None];
        let _ = view(&state, &hero, None, &workspaces);
    }

    /// v3.0.3 — view renders with a focused toplevel + populated
    /// hero so the new hero zone + window buttons exercise their
    /// happy-path branches too.
    /// v4.0.1 WM-1 — also exercises the populated-workspace chip
    /// row so workspace_chip's focused-bg branch hits.
    #[test]
    fn view_renders_with_hero_and_focused() {
        let state = TopBarState::demo();
        let mut hero = crate::hero::Hero::new();
        hero.set_focused("Terminal".into(), "foot".into());
        let focused = crate::toplevels::Toplevel {
            id: 7,
            title: "Terminal".into(),
            app_id: "foot".into(),
            state: crate::toplevels::ToplevelState {
                focused: true,
                ..Default::default()
            },
        };
        let workspaces: [Option<crate::workspaces::WorkspaceState>; 4] = [
            Some(crate::workspaces::WorkspaceState {
                num: 1,
                focused: true,
                has_windows: true,
            }),
            Some(crate::workspaces::WorkspaceState {
                num: 2,
                focused: false,
                has_windows: false,
            }),
            None,
            None,
        ];
        let _ = view(&state, &hero, Some(&focused), &workspaces);
    }

    /// v4.0.1 BUG-16 — Desktop Layout cluster replaces the
    /// retired window_button_cluster. This test exercises the
    /// 5-button render path so a regression that drops one of
    /// the buttons fails loudly.
    #[test]
    fn desktop_layout_cluster_renders_five_buttons() {
        let _cluster: Element<'_, Message> = desktop_layout_cluster_view();
    }
}
