//! `mde-wizard` binary — Iced first-run provisioning wizard.
//!
//! CB-1.10 (CLI shell — page widgets ship as `pages::*` data
//! modules; the Iced layout slots them into the wizard's
//! breadcrumb + Next/Back navigation).

#![forbid(unsafe_code)]

use std::time::Instant;

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
    /// NF-7.3 — operator clicked the Refresh button on the
    /// Preview page (or the page just became active).
    PreviewRefresh,
}

struct WizardApp {
    page: WizardPage,
    state: WizardState,
    /// NF-7.3 — Preview page state. Re-populated whenever the
    /// page becomes active or the operator clicks Refresh.
    preview: pages::preview::PreviewSnapshot,
    /// NF-7.3 — wall-clock moment the operator first reached
    /// the Preview page. Drives the 30 s diagnostics-banner
    /// gate. `None` until the page has been visited at least
    /// once in the session.
    preview_landed_at: Option<Instant>,
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
            preview: pages::preview::PreviewSnapshot::default(),
            preview_landed_at: None,
        };
        iced::application(Self::title, Self::update, Self::view)
            .theme(Self::theme)
            .window_size(Size::new(720.0, 540.0))
            .run_with(move || (app, Task::none()))
    }

    fn title(&self) -> String {
        format!(
            "MDE wizard — {} ({}/{})",
            self.page.label(),
            self.page.index(),
            WizardPage::total()
        )
    }

    #[allow(clippy::unused_self)]
    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::NavNext => {
                // NF-14.4 — when leaving Apply for the first time,
                // shell out to `mackesd enroll --token` if the
                // operator supplied a v2.5 join token. The spawn
                // is detached so the wizard doesn't freeze waiting
                // for the lighthouse to sign; the Preview page
                // (NF-7.3) sees the resulting Nebula state on its
                // probe + diagnostics-timer gate.
                let leaving_apply = self.page == WizardPage::Apply;
                if let Some(next) = self.page.next() {
                    if leaving_apply {
                        if let Some(argv) =
                            pages::apply::build_enroll_argv(&self.state.mesh_passcode)
                        {
                            if let Err(e) = std::process::Command::new(&argv[0])
                                .args(&argv[1..])
                                .spawn()
                            {
                                info!(
                                    "wizard: failed to spawn `{}`: {} \
                                     (Preview will show empty roster)",
                                    argv.join(" "),
                                    e,
                                );
                            } else {
                                info!(
                                    "wizard: spawned `{}` (Preview will pick up state)",
                                    argv.join(" "),
                                );
                            }
                        }
                    }
                    self.page = next;
                    // NF-7.3 — on first landing on Preview, kick
                    // off a probe + start the diagnostics timer.
                    // Re-entries (Back then Next again) preserve
                    // the original landed_at so the 30 s gate
                    // measures total time on the page, not time
                    // since the latest re-entry.
                    if self.page == WizardPage::Preview {
                        if self.preview_landed_at.is_none() {
                            self.preview_landed_at = Some(Instant::now());
                        }
                        self.preview = pages::preview::probe();
                    }
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
            Message::PreviewRefresh => {
                // NF-7.3 — operator-driven re-probe. Doesn't
                // reset preview_landed_at — the diagnostics
                // banner stays gated on total elapsed time, not
                // refresh count.
                self.preview = pages::preview::probe();
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
            WizardPage::LegacyImport => legacy_body(),
            WizardPage::Preset => preset_body(&self.state),
            WizardPage::MeshPasscode => mesh_body(&self.state),
            WizardPage::Network => network_body(),
            WizardPage::Snapshot => snapshot_body(),
            WizardPage::Apply => apply_body(),
            WizardPage::Preview => {
                let elapsed = self
                    .preview_landed_at
                    .map(|t| t.elapsed().as_secs())
                    .unwrap_or(0);
                preview_body(&self.preview, elapsed)
            }
        };

        let mut nav = row![].spacing(12).align_y(Alignment::Center);
        if self.page.prev().is_some() {
            nav = nav.push(button(text("← Back")).on_press(Message::NavPrev));
        }
        nav = nav.push(Space::with_width(Length::Fill));
        // NF-7.3 — Preview button label distinguishes "click to
        // run the birthright steps" (Apply) from "click to exit
        // the wizard" (Finish). Refresh is a side button on the
        // Preview body itself.
        let next_label = match (self.page.next().is_some(), self.page) {
            (true, _) => "Next →",
            (false, WizardPage::Preview) => "Finish ✓",
            (false, _) => "Apply ✓",
        };
        nav = nav.push(button(text(next_label)).on_press(Message::NavNext));

        container(
            column![
                header,
                Space::with_height(Length::Fixed(16.0)),
                body,
                Space::with_height(Length::Fill),
                nav
            ]
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
    let mut col = column![
        text("Environment").size(16),
        Space::with_height(Length::Fixed(8.0))
    ];
    for line in report.lines() {
        col = col.push(text(line).size(13));
    }
    col.into()
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
        col = col.push(text(format!("  · {} — {}", preset.display_name, preset.blurb)).size(13));
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
        text(
            "First-run NetworkManager bring-up. nmcli will list active connections at Apply time."
        )
        .size(13),
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

fn preview_body<'a>(
    snap: &'a pages::preview::PreviewSnapshot,
    elapsed_secs: u64,
) -> Element<'a, Message> {
    let mut col = column![
        text("Mesh preview").size(16),
        text(pages::preview::summary_line(snap)).size(13),
        Space::with_height(Length::Fixed(8.0)),
    ];
    if !snap.error.is_empty() {
        col = col.push(text(format!("Probe error: {}", snap.error)).size(12));
        col = col.push(Space::with_height(Length::Fixed(6.0)));
    }
    if let Some(self_node) = &snap.self_node {
        col = col.push(text(format!("  node-id: {}", self_node.node_id)).size(12));
        col = col.push(text(format!("  hostname: {}", self_node.host)).size(12));
        col = col.push(text(format!("  role: {}", self_node.role)).size(12));
        col = col.push(text(format!("  cert epoch: {}", self_node.cert_epoch)).size(12));
        col = col.push(Space::with_height(Length::Fixed(6.0)));
    }
    if snap.peers.is_empty() {
        col = col.push(text("Lighthouse roster: (no peers yet)").size(13));
    } else {
        col = col.push(text(format!("Lighthouse roster ({} peers):", snap.peers.len())).size(13));
        for peer in &snap.peers {
            col = col.push(
                text(format!(
                    "  · {} ({}) @ {} — {}",
                    peer.name, peer.node_id, peer.overlay_ip, peer.reachable
                ))
                .size(12),
            );
        }
    }
    // NF-7.3 — diagnostics banner once the 30 s threshold passes
    // with an empty roster. The banner copy is context-aware
    // (see pages::preview::diagnostic_message).
    if pages::preview::should_show_diagnostics(snap, elapsed_secs) {
        col = col.push(Space::with_height(Length::Fixed(10.0)));
        col = col.push(text("⚠ Diagnostics").size(14));
        col = col.push(text(pages::preview::diagnostic_message(snap)).size(12));
    }
    // Inline Refresh button so the operator can re-poll without
    // backing out of the wizard.
    col = col.push(Space::with_height(Length::Fixed(12.0)));
    col = col.push(
        row![button(text("Refresh probe")).on_press(Message::PreviewRefresh)]
            .spacing(8)
            .align_y(Alignment::Center),
    );
    col.into()
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
