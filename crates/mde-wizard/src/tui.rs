//! TUI frontend — ratatui + crossterm console wizard.
//!
//! Runs the same state machine as the Iced GUI but renders to a
//! TTY. The launcher (`mde-installer-launch`) calls into this
//! path after `chvt`-ing to tty1 and clamping the framebuffer
//! resolution; this module owns the render loop, the keyboard
//! input handling, and the per-page widgets.
//!
//! ## Layout
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │ Mackes Desktop Environment — first-run wizard            │
//! │ Step 3/9 — Baseline purge                                │
//! ├──────────────────────────────────────────────────────────┤
//! │                                                          │
//! │  <page body — fills available space>                     │
//! │                                                          │
//! │                                                          │
//! ├──────────────────────────────────────────────────────────┤
//! │ ←/→ navigate · q quit · enter confirm                    │
//! └──────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Why a separate render loop, not a shared trait?
//!
//! Iced and ratatui have fundamentally different mental models:
//! Iced is a retained-mode update/view loop driven by typed
//! `Message` values; ratatui is an immediate-mode redraw on
//! every event. Trying to express both behind one trait costs
//! more in adapter code than it saves; instead each frontend
//! pulls page data directly from `crate::pages::*` (which are
//! pure data modules) and renders in its native style.

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::purge_ui::{PurgeStep, PurgeUi};
use crate::{pages, WizardPage, WizardState};

/// Run the TUI wizard. Blocks until the user reaches Apply +
/// confirms, or quits.
///
/// Returns Ok with the final `WizardState` (caller persists it),
/// or an io error if the terminal couldn't be set up.
pub fn run(initial_state: WizardState) -> io::Result<TuiOutcome> {
    let mut terminal = setup_terminal()?;
    let outcome = match drive(&mut terminal, initial_state) {
        Ok(o) => o,
        Err(e) => {
            // Always restore the terminal even if the loop blew
            // up — otherwise the user is staring at a scrambled
            // tty.
            let _ = teardown_terminal(&mut terminal);
            return Err(e);
        }
    };
    teardown_terminal(&mut terminal)?;
    Ok(outcome)
}

/// What the TUI loop produces when the user is done.
#[derive(Debug, Clone)]
pub enum TuiOutcome {
    /// User walked all 9 pages and confirmed Apply. State carries
    /// the user's choices.
    Completed(WizardState),
    /// User quit before completion (q / ctrl-c). State may be
    /// partial; caller decides whether to persist.
    Quit(WizardState),
    /// User declined the Stage-2 purge — installer must abort
    /// per the locked decision (CLAUDE.md §0 + design lock).
    /// The caller writes the /etc/motd banner + non-zero exit.
    PurgeDeclined,
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

fn teardown_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Core render-input loop. Factored out so `run` can guarantee
/// terminal restoration even on error.
fn drive(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut state: WizardState,
) -> io::Result<TuiOutcome> {
    let mut page = WizardPage::Welcome;
    let mut purge_ui: Option<PurgeUi> = None;
    // Tick at 4Hz so the screen redraws on resize / focus
    // events that wouldn't otherwise wake us. Cheap — the
    // page-body renders are O(lines on screen).
    let tick = Duration::from_millis(250);

    loop {
        // Lazy-init the purge sub-UI when the user lands on
        // WizardPage::Purge for the first time.
        if page == WizardPage::Purge && purge_ui.is_none() {
            purge_ui = Some(PurgeUi::new());
        }

        // The Scanning sub-step is one frame: render "scanning…"
        // first so the user sees it, then run the scan, then the
        // next iteration renders the list. We do this BEFORE
        // `terminal.draw` so the scan actually runs between
        // frames.
        if page == WizardPage::Purge {
            if let Some(ui) = purge_ui.as_mut() {
                if ui.step() == PurgeStep::Scanning {
                    let scan = pages::purge::scan_system()
                        .unwrap_or_else(|_| pages::purge::ScanResult::default());
                    ui.run_scan(scan);
                }
            }
        }

        terminal.draw(|frame| render(frame, page, &state, purge_ui.as_mut()))?;

        if !event::poll(tick)? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        // Purge page has its own state machine — route input
        // through it and only fall through to outer navigation
        // when the sub-flow reaches a terminal step.
        if page == WizardPage::Purge {
            if let Some(ui) = purge_ui.as_mut() {
                ui.handle_key(key);
                match ui.step() {
                    PurgeStep::Done => {
                        // Sub-flow complete; let the user press
                        // → / Enter to advance off the Purge
                        // page. We fall through to the outer
                        // match so navigation keys take effect.
                    }
                    PurgeStep::Aborted => {
                        return Ok(TuiOutcome::PurgeDeclined);
                    }
                    _ => continue, // stay on the Purge page
                }
            }
        }

        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                return Ok(TuiOutcome::Quit(state));
            }
            (KeyCode::Esc, _) => {
                return Ok(TuiOutcome::Quit(state));
            }
            (KeyCode::Left, _) | (KeyCode::Char('h'), _) => {
                if let Some(prev) = page.prev() {
                    page = prev;
                    if page != WizardPage::Purge {
                        purge_ui = None;
                    }
                }
            }
            (KeyCode::Right, _) | (KeyCode::Char('l'), _) | (KeyCode::Enter, _) => {
                if page == WizardPage::Apply {
                    pages::apply::finalize(&mut state);
                    return Ok(TuiOutcome::Completed(state));
                }
                if let Some(next) = page.next() {
                    page = next;
                }
            }
            _ => {}
        }
    }
}

fn render(
    frame: &mut Frame<'_>,
    page: WizardPage,
    state: &WizardState,
    purge_ui: Option<&mut PurgeUi>,
) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(0),    // body
            Constraint::Length(3), // footer (key hints)
        ])
        .split(area);

    render_header(frame, chunks[0], page);
    if page == WizardPage::Purge {
        if let Some(ui) = purge_ui {
            ui.render(frame, chunks[1]);
        } else {
            render_body(frame, chunks[1], page, state);
        }
    } else {
        render_body(frame, chunks[1], page, state);
    }
    render_footer(frame, chunks[2], page);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, page: WizardPage) {
    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            "Mackes Desktop Environment — first-run wizard",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!(
                "Step {}/{}  —  {}",
                page.index(),
                WizardPage::total(),
                page.label()
            ),
            Style::default().fg(Color::Cyan),
        )),
    ])
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, area);
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, page: WizardPage) {
    let hint = if page == WizardPage::Apply {
        "←/h back  ·  enter apply ✓  ·  q quit"
    } else if page.prev().is_some() {
        "←/h back  ·  →/l/enter next  ·  q quit"
    } else {
        "→/l/enter next  ·  q quit"
    };
    let footer = Paragraph::new(Line::from(Span::styled(
        hint,
        Style::default().fg(Color::DarkGray),
    )))
    .block(Block::default().borders(Borders::TOP));
    frame.render_widget(footer, area);
}

fn render_body(frame: &mut Frame<'_>, area: Rect, page: WizardPage, state: &WizardState) {
    let body = match page {
        WizardPage::Welcome => welcome_body(),
        WizardPage::Scan => scan_body(),
        WizardPage::Purge => purge_body(),
        WizardPage::LegacyImport => legacy_body(),
        WizardPage::Preset => preset_body(state),
        WizardPage::MeshPasscode => mesh_body(state),
        WizardPage::Network => network_body(),
        WizardPage::Snapshot => snapshot_body(),
        WizardPage::Apply => apply_body(),
    };
    let widget = Paragraph::new(body)
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::NONE).padding(
            ratatui::widgets::Padding::new(2, 2, 1, 1),
        ));
    frame.render_widget(widget, area);
}

fn welcome_body() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            pages::welcome::HEADLINE,
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::raw(pages::welcome::SUBHEAD),
        Line::raw(""),
        Line::raw("Press → or Enter to begin."),
    ]
}

fn scan_body() -> Vec<Line<'static>> {
    let report = pages::scan::ScanReport::probe();
    let mut lines = vec![Line::from(Span::styled(
        "Environment",
        Style::default().add_modifier(Modifier::BOLD),
    ))];
    lines.push(Line::raw(""));
    for line in report.lines() {
        lines.push(Line::raw(line));
    }
    lines
}

fn purge_body() -> Vec<Line<'static>> {
    // Fallback only — the live PurgeUi renderer takes over the
    // page body when the wizard reaches WizardPage::Purge. This
    // arm fires only if the outer loop hasn't initialized the
    // sub-UI yet (one frame at most, during the lazy-init race).
    vec![
        Line::from(Span::styled(
            "Baseline purge",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::raw("Initializing scan…"),
    ]
}

fn legacy_body() -> Vec<Line<'static>> {
    let detection = pages::legacy_import::LegacyDetection::probe();
    vec![
        Line::from(Span::styled(
            "Legacy import",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::raw(detection.summary()),
    ]
}

fn preset_body(state: &WizardState) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(Span::styled(
            "Pick a preset",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::raw(format!("Active: {}", state.preset)),
        Line::raw(""),
    ];
    for preset in pages::preset::PRESETS {
        lines.push(Line::raw(format!(
            "  · {}  —  {}",
            preset.display_name, preset.blurb
        )));
    }
    lines
}

fn mesh_body(state: &WizardState) -> Vec<Line<'static>> {
    let current = if state.mesh_passcode.is_empty() {
        "(none — will prompt at Apply)"
    } else {
        state.mesh_passcode.as_str()
    };
    vec![
        Line::from(Span::styled(
            "Mesh passcode",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::raw("16-character shared passcode (uppercase letters + digits)."),
        Line::raw(""),
        Line::raw(format!("Current: {current}")),
    ]
}

fn network_body() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "Network",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::raw(
            "First-run NetworkManager bring-up. nmcli will list active connections at Apply.",
        ),
    ]
}

fn snapshot_body() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "Snapshot",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::raw(format!(
            "A pre-apply snapshot tagged `{}` will be created so you can roll back via `mde recover`.",
            pages::snapshot::default_tag()
        )),
    ]
}

fn apply_body() -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(Span::styled(
            "Apply",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::raw("Selected birthright steps:"),
        Line::raw(""),
    ];
    for step in pages::apply::STEPS {
        let mark = if step.default_on { "[x]" } else { "[ ]" };
        lines.push(Line::raw(format!("  {mark}  {}", step.label)));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "Press Enter to finalize.",
        Style::default().fg(Color::Green),
    )));
    lines
}

/// Soft-wrap a paragraph at word boundaries to `cols` columns.
///
/// Used for the Purge rationale + any other prose that ratatui
/// would otherwise truncate mid-word. We don't reach for an
/// external textwrap crate because this is the only wrap
/// callsite — keeping the dep set tight matters for the RPM.
pub(crate) fn wrap_paragraph(text: &str, cols: usize) -> Vec<String> {
    if cols == 0 {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let need = if line.is_empty() { word.len() } else { line.len() + 1 + word.len() };
        if need > cols && !line.is_empty() {
            out.push(std::mem::take(&mut line));
            line.push_str(word);
        } else {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_paragraph_breaks_at_word_boundaries() {
        let wrapped = wrap_paragraph("the quick brown fox jumps over the lazy dog", 12);
        for line in &wrapped {
            assert!(line.len() <= 12, "line too long: {line:?}");
        }
        let joined: String = wrapped.join(" ");
        assert_eq!(joined, "the quick brown fox jumps over the lazy dog");
    }

    #[test]
    fn wrap_paragraph_short_input_one_line() {
        assert_eq!(wrap_paragraph("hello world", 80), vec!["hello world"]);
    }

    #[test]
    fn wrap_paragraph_zero_cols_returns_input_as_is() {
        let r = wrap_paragraph("foo bar", 0);
        assert_eq!(r, vec!["foo bar".to_string()]);
    }

    #[test]
    fn wrap_paragraph_single_long_word_kept_on_its_own_line() {
        // We don't hyphenate; an over-long token survives on its
        // own line rather than getting truncated.
        let r = wrap_paragraph("supercalifragilisticexpialidocious", 10);
        assert_eq!(r, vec!["supercalifragilisticexpialidocious"]);
    }

    #[test]
    fn welcome_body_has_at_least_three_lines() {
        let lines = welcome_body();
        assert!(lines.len() >= 3, "got {} lines", lines.len());
    }

    #[test]
    fn purge_body_fallback_mentions_baseline_purge() {
        // Live rationale lives in PurgeUi now; this fallback
        // path only renders for one frame during sub-UI init.
        let lines = purge_body();
        let blob: String = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join(" ");
        assert!(blob.contains("Baseline purge"));
    }

    #[test]
    fn apply_body_lists_every_birthright_step() {
        let lines = apply_body();
        let blob: String = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        for step in pages::apply::STEPS {
            assert!(blob.contains(step.label), "missing step: {}", step.label);
        }
    }
}
