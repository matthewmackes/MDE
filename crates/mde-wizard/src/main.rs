//! `mde-wizard` binary — Iced first-run provisioning wizard.
//!
//! CB-1.10 (CLI shell — page widgets ship as `pages::*` data
//! modules; the Iced layout slots them into the wizard's
//! breadcrumb + Next/Back navigation).

#![forbid(unsafe_code)]

use clap::Parser;
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Length, Padding, Size, Task, Theme};
use tracing::info;

use mde_wizard::{pages, WizardPage, WizardState};

#[derive(Parser, Debug)]
#[command(
    name = "mde-wizard",
    about = "Mackes Desktop Environment (MDE) first-run wizard"
)]
struct Cli {
    /// Force the wizard to re-run even if state.json says provisioned.
    #[arg(long)]
    rerun: bool,
}

#[derive(Debug, Clone)]
enum Message {
    NavNext,
    NavPrev,
    Quit,
}

struct WizardApp {
    page: WizardPage,
    state: WizardState,
}

impl WizardApp {
    fn run(rerun: bool) -> iced::Result {
        let path = WizardState::default_path();
        let mut state = WizardState::load(&path);
        if state.provisioned && !rerun {
            info!("wizard already provisioned; pass --rerun to force");
            // Still launch the UI but jump to the Apply page so
            // the user can see they're done.
        } else if state.preset.is_empty() {
            state.preset = pages::preset::DEFAULT_PRESET.into();
        }

        let app = Self {
            page: WizardPage::Welcome,
            state,
        };
        iced::application(Self::title, Self::update, Self::view)
            .theme(Self::theme)
            .window_size(Size::new(720.0, 540.0))
            .run_with(move || (app, Task::none()))
    }

    fn title(&self) -> String {
        format!("MDE wizard — {} ({}/{})", self.page.label(), self.page.index(), WizardPage::total())
    }

    #[allow(clippy::unused_self)]
    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::NavNext => {
                if let Some(next) = self.page.next() {
                    self.page = next;
                } else {
                    // Reached the end — finalize + persist + exit.
                    pages::apply::finalize(&mut self.state);
                    let _ = self.state.save(&WizardState::default_path());
                    info!("wizard complete — state.json saved + provisioned=true");
                }
            }
            Message::NavPrev => {
                if let Some(prev) = self.page.prev() {
                    self.page = prev;
                }
            }
            Message::Quit => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let header = text(format!(
            "{} ({}/{})",
            self.page.label(),
            self.page.index(),
            WizardPage::total()
        ))
        .size(22);

        let body = match self.page {
            WizardPage::Welcome => welcome_body(),
            WizardPage::Scan => scan_body(),
            WizardPage::Purge => purge_body(),
            WizardPage::LegacyImport => legacy_body(),
            WizardPage::Preset => preset_body(&self.state),
            WizardPage::MeshPasscode => mesh_body(&self.state),
            WizardPage::Network => network_body(),
            WizardPage::Snapshot => snapshot_body(),
            WizardPage::Apply => apply_body(),
        };

        let mut nav = row![].spacing(12).align_y(Alignment::Center);
        if self.page.prev().is_some() {
            nav = nav.push(button(text("← Back")).on_press(Message::NavPrev));
        }
        nav = nav.push(Space::with_width(Length::Fill));
        let next_label = if self.page.next().is_some() {
            "Next →"
        } else {
            "Apply ✓"
        };
        nav = nav.push(button(text(next_label)).on_press(Message::NavNext));

        container(
            column![header, Space::with_height(Length::Fixed(16.0)), body, Space::with_height(Length::Fill), nav]
                .padding(Padding::new(24.0))
                .spacing(8),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

fn welcome_body<'a>() -> Element<'a, Message> {
    column![
        text(pages::welcome::HEADLINE).size(20),
        Space::with_height(Length::Fixed(8.0)),
        text(pages::welcome::SUBHEAD).size(14),
    ]
    .into()
}

fn scan_body<'a>() -> Element<'a, Message> {
    let report = pages::scan::ScanReport::probe();
    let mut col = column![text("Environment").size(16), Space::with_height(Length::Fixed(8.0))];
    for line in report.lines() {
        col = col.push(text(line).size(13));
    }
    col.into()
}

fn purge_body<'a>() -> Element<'a, Message> {
    // Placeholder body — the live scan + per-app whitelist
    // ratatui+Iced widgets land in the commit that wires
    // pages::purge into the frontends. For now the page
    // renders the rationale + a stub note so the breadcrumb
    // navigation is testable end-to-end.
    column![
        text("Baseline purge").size(16),
        Space::with_height(Length::Fixed(8.0)),
        text(pages::purge::RATIONALE).size(13),
        Space::with_height(Length::Fixed(8.0)),
        text("(Live scan UI lands in the next commit.)").size(12),
    ]
    .into()
}

fn legacy_body<'a>() -> Element<'a, Message> {
    let detection = pages::legacy_import::LegacyDetection::probe();
    column![
        text("Legacy import").size(16),
        Space::with_height(Length::Fixed(8.0)),
        text(detection.summary()).size(13),
    ]
    .into()
}

fn preset_body<'a>(state: &'a WizardState) -> Element<'a, Message> {
    let mut col = column![
        text(format!("Active preset: {}", state.preset)).size(16),
        Space::with_height(Length::Fixed(8.0)),
    ];
    for preset in pages::preset::PRESETS {
        col = col.push(
            text(format!("  · {} — {}", preset.display_name, preset.blurb)).size(13),
        );
    }
    col.into()
}

fn mesh_body<'a>(state: &'a WizardState) -> Element<'a, Message> {
    column![
        text("Mesh passcode").size(16),
        text("16-character shared passcode (uppercase letters + digits).").size(13),
        Space::with_height(Length::Fixed(8.0)),
        text(format!(
            "Current: {}",
            if state.mesh_passcode.is_empty() {
                "(none — wizard will prompt at Apply)"
            } else {
                state.mesh_passcode.as_str()
            }
        ))
        .size(13),
    ]
    .into()
}

fn network_body<'a>() -> Element<'a, Message> {
    column![
        text("Network").size(16),
        text("First-run NetworkManager bring-up. nmcli will list active connections at Apply time.").size(13),
    ]
    .into()
}

fn snapshot_body<'a>() -> Element<'a, Message> {
    column![
        text("Snapshot").size(16),
        text(format!(
            "A pre-apply snapshot tagged `{}` will be created so you can roll back via `mde recover`.",
            pages::snapshot::default_tag()
        ))
        .size(13),
    ]
    .into()
}

fn apply_body<'a>() -> Element<'a, Message> {
    let mut col = column![
        text("Apply").size(16),
        text("Selected birthright steps:").size(13),
        Space::with_height(Length::Fixed(4.0)),
    ];
    for step in pages::apply::STEPS {
        let mark = if step.default_on { "[x]" } else { "[ ]" };
        col = col.push(text(format!("  {mark} {}", step.label)).size(13));
    }
    col = col.push(Space::with_height(Length::Fixed(8.0)));
    col = col.push(text("Click Apply ✓ to finalize.").size(13));
    col.into()
}

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("MDE_WIZARD_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("mde_wizard=info,warn")),
        )
        .init();
    let cli = Cli::parse();
    WizardApp::run(cli.rerun)
}
