//! `mde-peer-card` binary — Iced layer-shell modal spawned on
//! mesh-peer connection.
//!
//! Surface chrome per the 50-Q + FU + NFU + UX-24..UX-28 lock
//! set, anchored to the visual identity at
//! `docs/design/visual-identity.md`:
//!
//! - 360 px wide (re-uses `mde-drawer::DRAWER_WIDTH_PX`).
//! - 280 ms slide-in (`SLIDE_DURATION_MS`).
//! - `Palette::surface` background, `Radii::modal` (16 px)
//!   corners, `Shadow::modal()` elevation.
//! - Esc + click-outside dismiss (PC-1 + UX-27).
//! - Read-only — only Dismiss / Toggle / OpenWorkbench /
//!   Enrichment messages exist; nothing mutates peer state
//!   (`card_is_read_only` test enforces).
//!
//! Worklist: PC-1 (skeleton landed 2026-05-21).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::HashMap;

use clap::Parser;
use iced::keyboard::{self, key::Named};
use iced::widget::{column, container, scrollable};
use iced::{event, Background, Color, Element, Length, Size, Subscription, Task};
use mde_peer_card::{
    hero, sections,
    sections::{Section, SectionState},
    Enrichment, PeerCardData, PeerProbe, DRAWER_WIDTH_PX, SLIDE_DURATION_MS,
};
use mde_theme::{Density, Theme, Tokens};

/// CLI surface — `mde-peer-card --peer <id>` is the canonical
/// invocation from `mded`'s peer-join worker (PC-3); `--dry-run`
/// renders against a fixture peer for layout previews.
#[derive(Parser, Debug)]
#[command(
    name = "mde-peer-card",
    about = "MDE Peer Connection Card — modal shown on mesh-peer join",
    version
)]
struct Args {
    /// Peer ID to load from
    /// `~/.cache/mde/peers/<peer-id>/probe.json`.
    #[arg(long, value_name = "ID")]
    peer: Option<String>,

    /// Render with the deterministic fixture probe — for layout
    /// previews and screenshot capture.
    #[arg(long)]
    dry_run: bool,
}

/// Read-only message set. PC-11 `card_is_read_only` test enforces
/// that no variant mutates peer state:
///
/// - `Dismiss` closes the modal.
/// - `ToggleSection` flips section expansion (UI-only state).
/// - `OpenWorkbench` launches `mde-workbench --focus <id>` as a
///   separate process — no peer write.
/// - `EnrichmentReady(Enrichment)` streams in cached enrichment;
///   the message carries a fully-resolved value, not a mutation.
#[derive(Debug, Clone)]
#[allow(dead_code)] // OpenWorkbench + EnrichmentReady await wiring (PC-3 / PC-5..7)
enum Message {
    /// Close the modal.
    Dismiss,
    /// Expand or collapse the named section.
    ToggleSection(Section),
    /// Deep-link to the workbench peer panel (separate process;
    /// no write occurs from this binary).
    OpenWorkbench,
    /// Stream-in callback when an enrichment source resolves.
    /// The carried value is treated as immutable once received.
    EnrichmentReady(Enrichment),
}

/// Application state.
struct PeerCard {
    /// Probe + resolved enrichment for the peer being shown.
    data: PeerCardData,
    /// Per-section expanded/collapsed state.
    section_state: HashMap<Section, SectionState>,
    /// Resolved design tokens (theme + density). Read once at
    /// startup; live-switch is a Settings-panel concern handled
    /// at the daemon / workbench level.
    tokens: Tokens,
}

impl Default for PeerCard {
    fn default() -> Self {
        let mut section_state = HashMap::new();
        for s in Section::ordered() {
            section_state.insert(s, SectionState::default());
        }
        Self {
            data: PeerCardData::hwdb_only(PeerProbe::fixture()),
            section_state,
            tokens: Tokens::resolve(Theme::Dark, Density::Comfortable),
        }
    }
}

impl PeerCard {
    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Dismiss => {
                std::process::exit(0);
            }
            Message::ToggleSection(s) => {
                let state = self
                    .section_state
                    .entry(s)
                    .or_insert_with(SectionState::default);
                state.expanded = !state.expanded;
                Task::none()
            }
            Message::OpenWorkbench => {
                let peer_id = self.data.probe.peer_id.clone();
                let _ = std::process::Command::new("mde-workbench")
                    .arg("--focus")
                    .arg(format!("peers:{peer_id}"))
                    .spawn();
                Task::none()
            }
            Message::EnrichmentReady(e) => {
                self.data.enrichment = e;
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;

        // Hero strip — full identity, ~280 px.
        let hero_block =
            hero::view::<Message>(&self.data.probe, &self.data.enrichment, &self.tokens);

        // Sections — four collapsible, scrollable rows.
        let section_views = Section::ordered().into_iter().map(|s| {
            let state = self.section_state.get(&s).copied().unwrap_or_default();
            sections::view::<Message>(
                s,
                state,
                &self.data.probe,
                &self.tokens,
                Message::ToggleSection,
            )
        });
        let sections_col = column(section_views.collect::<Vec<_>>())
            .spacing(space.xs2)
            .width(Length::Fill);

        // Combine. Hero is fixed; sections scroll.
        let inner = column![hero_block, scrollable(sections_col).height(Length::Fill),]
            .width(Length::Fill)
            .height(Length::Fill);

        // Outer modal chrome: surface ground, 16 px corners (Q45),
        // modal shadow (Q20).
        container(inner)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_theme| container::Style {
                background: Some(Background::Color(rgba_to_color(palette.surface))),
                border: iced::Border {
                    color: rgba_to_color(palette.border),
                    width: 1.0,
                    radius: self.tokens.radii.modal.into(),
                },
                ..container::Style::default()
            })
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        // Esc dismiss + click-outside dismiss (UX-27 sub-lock).
        // Click-outside-the-modal detection lands when Iced 0.14's
        // global mouse capture is available (UX-PRE unblock); for
        // now Esc is the load-bearing dismiss path.
        event::listen_with(|evt, _status, _id| match evt {
            event::Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(Named::Escape),
                ..
            }) => Some(Message::Dismiss),
            _ => None,
        })
    }

    fn theme(&self) -> iced::Theme {
        // The dark / light palette comes from mde-theme directly
        // via tokens — Iced's Theme enum is only used here as a
        // hint to the renderer.
        match self.tokens.theme {
            Theme::Dark => iced::Theme::Dark,
            Theme::Light => iced::Theme::Light,
        }
    }
}

fn rgba_to_color(c: mde_theme::Rgba) -> Color {
    c.into_iced_color()
}

fn main() -> iced::Result {
    let _ = Args::parse(); // Parse + validate; peer / dry-run wiring lands when PC-3 spawns us with --peer.

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mde_peer_card=info".into()),
        )
        .init();

    let _slide = SLIDE_DURATION_MS; // Documented chrome lock; the slide animation lands with iced_layershell 0.18 (E.2 / UX-PRE).

    iced::application("MDE Peer Connection Card", PeerCard::update, PeerCard::view)
        .subscription(PeerCard::subscription)
        .theme(PeerCard::theme)
        .window_size(Size::new(f32::from(DRAWER_WIDTH_PX), 840.0))
        .run()
}
