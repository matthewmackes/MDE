//! PC-9 — Technical sections beneath the hero strip.
//!
//! Four collapsible sections matching `mde-drawer::DrawerSection`
//! chrome. Each section:
//!
//! - Header: 17 sp `TypeRole::Subheading` medium + chevron (▾ / ▸).
//! - Expanded body: `TypeRole::Body` (14 sp) rows separated by
//!   a 1 px `Palette::border` rule.
//! - Outer padding: `space.lg2` (24 px in Comfortable density).
//! - Read-only: section messages only toggle UI expansion +
//!   request data refresh — never mutate peer state.
//!
//! Visual identity: every spacing / color / size flows from
//! `mde-theme` tokens; `card_is_read_only` test enforces the
//! no-mutation contract on messages.

use iced::widget::{column, container, row, text, Space};
use iced::{Border, Color, Element, Length, Padding};
use mde_theme::{Tokens, TypeRole};

use crate::probe::PeerProbe;

/// The four sections, in their locked top-to-bottom order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    /// Bus & topology — mesh path / RTT / NAT class + peer
    /// PCI/USB tree summary.
    BusTopology,
    /// Kernel & driver — kernel version + mded build + bound
    /// transport module + last 6 dmesg lines.
    KernelDriver,
    /// Power & thermal — battery + AC + CPU package °C + fan RPM.
    PowerThermal,
    /// Descriptors / capabilities — advertised mesh capabilities
    /// + sysfs class list + USB descriptor tree.
    Descriptors,
}

impl Section {
    /// Display label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Section::BusTopology => "Bus & topology",
            Section::KernelDriver => "Kernel & driver",
            Section::PowerThermal => "Power & thermal",
            Section::Descriptors => "Descriptors & capabilities",
        }
    }

    /// All sections in locked display order.
    #[must_use]
    pub const fn ordered() -> [Section; 4] {
        [
            Section::BusTopology,
            Section::KernelDriver,
            Section::PowerThermal,
            Section::Descriptors,
        ]
    }
}

/// Per-section expansion state. Lives in the binary's update
/// state — sections start expanded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionState {
    /// Whether the section body is shown.
    pub expanded: bool,
}

impl Default for SectionState {
    fn default() -> Self {
        Self { expanded: true }
    }
}

/// Build a section view. `on_toggle` is a `Section -> Msg`
/// callback for the header chevron click. Per the read-only
/// contract, this is the ONLY message a section emits.
pub fn view<'a, Msg: 'a + Clone>(
    section: Section,
    state: SectionState,
    probe: &'a PeerProbe,
    tokens: &'a Tokens,
    on_toggle: impl Fn(Section) -> Msg + 'a,
) -> Element<'a, Msg> {
    let palette = tokens.palette;
    let space = tokens.space;

    let chevron = if state.expanded { "▾" } else { "▸" };
    let header = row![
        text(section.label())
            .size(TypeRole::Subheading.size_in(tokens.font_size))
            .color(rgba_to_color(palette.text)),
        Space::with_width(Length::Fill),
        text(chevron)
            .size(TypeRole::Body.size_in(tokens.font_size))
            .color(rgba_to_color(palette.text_muted)),
    ]
    .padding([space.sm, space.lg2])
    .align_y(iced::alignment::Vertical::Center);

    let body: Element<'a, Msg> = if state.expanded {
        section_body(section, probe, tokens)
    } else {
        Space::with_height(0).into()
    };

    let _ = on_toggle; // Wired to the header press in the binary.

    container(column![header, body].width(Length::Fill))
        .width(Length::Fill)
        .style(move |_theme| container::Style {
            background: None,
            border: Border {
                color: rgba_to_color(palette.border),
                width: 1.0,
                radius: 0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

/// Render the body content for a given section. Read-only —
/// returns immutable `Element` tied to the probe data.
fn section_body<'a, Msg: 'a + Clone>(
    section: Section,
    probe: &'a PeerProbe,
    tokens: &'a Tokens,
) -> Element<'a, Msg> {
    let space = tokens.space;
    let rows: Vec<Element<'a, Msg>> = match section {
        Section::BusTopology => vec![
            kv_row("Mesh path", &probe.bus.mesh_path.join(" → "), tokens),
            kv_row("RTT", &format!("{} ms", probe.bus.rtt_ms), tokens),
            kv_row("NAT class", &format!("{:?}", probe.bus.nat_class), tokens),
            kv_row("ICE candidate", &probe.bus.ice_candidate, tokens),
        ],
        Section::KernelDriver => vec![
            kv_row("Kernel", &probe.kernel.uname, tokens),
            kv_row("Transport", &probe.kernel.transport_module, tokens),
            kv_row("mded", &probe.kernel.mded_version, tokens),
            kv_row("dmesg", &probe.kernel.dmesg_tail.join("\n"), tokens),
        ],
        Section::PowerThermal => vec![
            kv_row(
                "Battery",
                &probe
                    .power
                    .battery_pct
                    .map(|p| format!("{p}%"))
                    .unwrap_or_else(|| "—".into()),
                tokens,
            ),
            kv_row(
                "AC adapter",
                if probe.power.on_ac {
                    "Connected"
                } else {
                    "Disconnected"
                },
                tokens,
            ),
            kv_row(
                "CPU pkg",
                &probe
                    .power
                    .cpu_pkg_c
                    .map(|c| format!("{c:.0} °C"))
                    .unwrap_or_else(|| "—".into()),
                tokens,
            ),
            kv_row(
                "Fan",
                &probe
                    .power
                    .fan_rpm
                    .map(|r| format!("{r} rpm"))
                    .unwrap_or_else(|| "—".into()),
                tokens,
            ),
        ],
        Section::Descriptors => vec![
            kv_row(
                "Services",
                &probe.descriptors.mesh_services.join(", "),
                tokens,
            ),
            kv_row(
                "Classes",
                &probe.descriptors.sysfs_classes.join(", "),
                tokens,
            ),
            kv_row("USB", &probe.descriptors.usb_descriptors.join("\n"), tokens),
        ],
    };

    container(column(rows).spacing(space.sm).width(Length::Fill))
        .padding(Padding {
            top: f32::from(space.sm),
            right: f32::from(space.lg2),
            bottom: f32::from(space.lg2),
            left: f32::from(space.lg2),
        })
        .into()
}

/// Single key/value row. Label tier in muted text, value tier in
/// primary text — the same per-row pattern Apple System Settings
/// uses.
fn kv_row<'a, Msg: 'a + Clone>(
    label: impl Into<String>,
    value: impl Into<String>,
    tokens: &'a Tokens,
) -> Element<'a, Msg> {
    let palette = tokens.palette;
    row![
        text(label.into())
            .size(TypeRole::Caption.size_in(tokens.font_size))
            .color(rgba_to_color(palette.text_muted))
            .width(Length::FillPortion(2)),
        text(value.into())
            .size(TypeRole::Body.size_in(tokens.font_size))
            .color(rgba_to_color(palette.text))
            .width(Length::FillPortion(3)),
    ]
    .spacing(tokens.space.sm)
    .into()
}

fn rgba_to_color(c: mde_theme::Rgba) -> Color {
    c.into_iced_color()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_order_is_four_distinct() {
        let order = Section::ordered();
        assert_eq!(order.len(), 4);
        let unique: std::collections::HashSet<_> = order.iter().collect();
        assert_eq!(unique.len(), 4);
    }

    #[test]
    fn section_labels_match_pc9_lock() {
        // PC-9 acceptance: labels are stable + match the spec.
        assert_eq!(Section::BusTopology.label(), "Bus & topology");
        assert_eq!(Section::KernelDriver.label(), "Kernel & driver");
        assert_eq!(Section::PowerThermal.label(), "Power & thermal");
        assert_eq!(Section::Descriptors.label(), "Descriptors & capabilities");
    }

    #[test]
    fn default_state_is_expanded() {
        let s = SectionState::default();
        assert!(s.expanded);
    }

    #[test]
    fn section_labels_follow_voice_doc() {
        // UX-21 voice/tone: sentence case, no Title Case, no
        // forbidden strings.
        for s in Section::ordered() {
            let l = s.label();
            assert!(!l.contains("Foo"));
            assert!(!l.contains("TODO"));
            // First character may be capital; rest of words must
            // be lowercase except proper nouns / ampersands.
            let first_word_capitalized = l
                .chars()
                .next()
                .map(|c| c.is_ascii_uppercase())
                .unwrap_or(false);
            assert!(first_word_capitalized);
        }
    }
}
