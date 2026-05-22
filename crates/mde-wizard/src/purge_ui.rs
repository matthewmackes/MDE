//! Stage-2 baseline-purge TUI sub-flow.
//!
//! The Purge page in `WizardPage` is a multi-step sub-flow with
//! its own state machine. The outer wizard delegates input +
//! rendering to this module whenever the user is on
//! `WizardPage::Purge`; control returns to the outer wizard
//! when the user finishes (the scan finds nothing, the user
//! confirms removal, or the user declines and aborts).
//!
//! ## Sub-step flow
//!
//! ```text
//! Warning  ─→  Scanning  ─→  ListView  ─→  Confirm  ─→  Executing  ─→  Done
//!                                ↓
//!                              Aborted   (q anywhere)
//! ```
//!
//! ## Why a separate module
//!
//! The Purge page has the most complex per-page state of any
//! wizard step (selection set, list cursor, filter string, scan
//! results) — keeping it out of `tui.rs` keeps that module
//! focused on the outer page-flow.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph, Wrap};
use ratatui::Frame;

use crate::pages::purge::{self, PurgeFamily, PurgeSelection, ScanResult};

/// Where we are in the Purge sub-flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurgeStep {
    /// Static rationale + "press Enter to scan."
    Warning,
    /// Synchronous scan in progress (one-frame state — the scan
    /// runs inline and we advance to `ListView` on the next
    /// render). Visible only briefly.
    Scanning,
    /// Scrollable per-package list. The user toggles whitelist
    /// membership with space, filters with `/`, advances to
    /// `Confirm` with Enter.
    ListView,
    /// "Will remove N packages — Continue? (y/n)" gate.
    Confirm,
    /// Execution placeholder — for this commit, shows the dnf
    /// argv that would run + the manifest path. INST-3 wires
    /// the real subprocess.
    Executing,
    /// User confirmed + exec finished; the outer wizard should
    /// advance to the next page.
    Done,
    /// User declined (or quit during purge) — outer wizard
    /// should bubble `TuiOutcome::PurgeDeclined`.
    Aborted,
}

/// Per-row state in the ListView. Materialized once after
/// scanning; ordered by family → name.
#[derive(Debug, Clone)]
struct Row {
    /// Package name (RPM / flatpak app-id / snap name).
    name: String,
    /// Family label for the section header. None means "same
    /// family as previous row" (header suppressed).
    family_header: Option<&'static str>,
    /// Human-readable blurb shown when this row is focused.
    blurb: String,
}

/// All of the Purge sub-flow state. Owned by the outer TUI;
/// dropped when the wizard advances past WizardPage::Purge.
pub struct PurgeUi {
    step: PurgeStep,
    scan: Option<ScanResult>,
    rows: Vec<Row>,
    selection: PurgeSelection,
    list_state: ListState,
    /// Filter prefix — case-insensitive substring match against
    /// row.name. Empty string = show everything.
    filter: String,
    /// True iff the user is currently typing into the filter
    /// field (`/` was pressed). Keystrokes go to the filter
    /// instead of the list while this is on.
    filter_editing: bool,
    /// Cached "filtered indices into `rows`" so we don't
    /// recompute every render.
    filtered: Vec<usize>,
}

impl Default for PurgeUi {
    fn default() -> Self {
        Self {
            step: PurgeStep::Warning,
            scan: None,
            rows: Vec::new(),
            selection: PurgeSelection::default(),
            list_state: ListState::default(),
            filter: String::new(),
            filter_editing: false,
            filtered: Vec::new(),
        }
    }
}

impl PurgeUi {
    /// Construct a fresh state-machine instance. Call when the
    /// user first lands on WizardPage::Purge.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Current sub-step (outer wizard checks this to decide
    /// whether to advance away from the Purge page).
    #[must_use]
    pub fn step(&self) -> PurgeStep {
        self.step
    }

    /// Take ownership of the final scan + selection — used by
    /// the outer wizard when the purge sub-flow completes so it
    /// can pass them to the executor / manifest writer.
    #[must_use]
    pub fn into_result(self) -> Option<(ScanResult, PurgeSelection)> {
        Some((self.scan?, self.selection))
    }

    /// Handle one keystroke. Returns true iff the outer wizard
    /// should re-render (always true in practice — kept for
    /// future "this key was ignored, don't redraw" optimization).
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return false;
        }

        if self.filter_editing {
            self.handle_filter_key(key);
            return true;
        }

        match self.step {
            PurgeStep::Warning => self.handle_warning_key(key),
            PurgeStep::Scanning => {} // no input; auto-advances
            PurgeStep::ListView => self.handle_list_key(key),
            PurgeStep::Confirm => self.handle_confirm_key(key),
            PurgeStep::Executing => self.handle_executing_key(key),
            PurgeStep::Done | PurgeStep::Aborted => {} // terminal
        }
        true
    }

    fn handle_warning_key(&mut self, key: KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => {
                self.step = PurgeStep::Aborted;
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.step = PurgeStep::Aborted;
            }
            (KeyCode::Enter, _) | (KeyCode::Char(' '), _) | (KeyCode::Right, _) => {
                self.step = PurgeStep::Scanning;
            }
            _ => {}
        }
    }

    /// Called by the outer loop after `Scanning` is entered to
    /// run the actual rpm/flatpak/snap scan. Synchronous; OK
    /// because the rpm cache populate is ~250 ms and the user
    /// is staring at a "scanning…" frame.
    pub fn run_scan(&mut self, scan: ScanResult) {
        self.rows = build_rows(&scan);
        self.filtered = (0..self.rows.len()).collect();
        self.scan = Some(scan);
        if let Some(s) = &self.scan {
            if s.is_empty() {
                // Nothing to remove — skip the list + confirm
                // entirely. Outer wizard advances on Done.
                self.step = PurgeStep::Done;
                return;
            }
        }
        if !self.rows.is_empty() {
            self.list_state.select(Some(0));
        }
        self.step = PurgeStep::ListView;
    }

    fn handle_list_key(&mut self, key: KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => {
                self.step = PurgeStep::Aborted;
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.step = PurgeStep::Aborted;
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), _) => self.cursor_down(),
            (KeyCode::Up, _) | (KeyCode::Char('k'), _) => self.cursor_up(),
            (KeyCode::PageDown, _) => {
                for _ in 0..10 {
                    self.cursor_down();
                }
            }
            (KeyCode::PageUp, _) => {
                for _ in 0..10 {
                    self.cursor_up();
                }
            }
            (KeyCode::Home, _) if !self.filtered.is_empty() => {
                self.list_state.select(Some(0));
            }
            (KeyCode::End, _) if !self.filtered.is_empty() => {
                self.list_state.select(Some(self.filtered.len() - 1));
            }
            (KeyCode::Char(' '), _) => self.toggle_focused(),
            (KeyCode::Char('w'), _) => self.toggle_focused(),
            (KeyCode::Char('a'), _) => self.clear_whitelist(),
            (KeyCode::Char('/'), _) => {
                self.filter_editing = true;
            }
            (KeyCode::Enter, _) => {
                self.step = PurgeStep::Confirm;
            }
            _ => {}
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => {
                self.filter.clear();
                self.filter_editing = false;
                self.recompute_filter();
            }
            (KeyCode::Enter, _) => {
                self.filter_editing = false;
                self.recompute_filter();
            }
            (KeyCode::Backspace, _) => {
                self.filter.pop();
                self.recompute_filter();
            }
            (KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
                self.filter.push(c);
                self.recompute_filter();
            }
            _ => {}
        }
    }

    fn handle_confirm_key(&mut self, key: KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Char('y'), _) | (KeyCode::Char('Y'), _) | (KeyCode::Enter, _) => {
                self.step = PurgeStep::Executing;
            }
            (KeyCode::Char('n'), _) | (KeyCode::Char('N'), _) | (KeyCode::Esc, _) => {
                // Return to list view — user wants to revise.
                self.step = PurgeStep::ListView;
            }
            (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.step = PurgeStep::Aborted;
            }
            _ => {}
        }
    }

    fn handle_executing_key(&mut self, key: KeyEvent) {
        // INST-3 will replace this with a real progress view +
        // subprocess pump. For now any key advances out of the
        // placeholder.
        if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
            self.step = PurgeStep::Done;
        }
    }

    fn cursor_down(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        let cur = self.list_state.selected().unwrap_or(0);
        let next = (cur + 1).min(self.filtered.len() - 1);
        self.list_state.select(Some(next));
    }

    fn cursor_up(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        let cur = self.list_state.selected().unwrap_or(0);
        let prev = cur.saturating_sub(1);
        self.list_state.select(Some(prev));
    }

    fn toggle_focused(&mut self) {
        let Some(idx) = self.list_state.selected() else { return; };
        let Some(&row_idx) = self.filtered.get(idx) else { return; };
        let name = self.rows[row_idx].name.clone();
        self.selection.toggle(&name);
    }

    fn clear_whitelist(&mut self) {
        self.selection.clear();
    }

    fn recompute_filter(&mut self) {
        let needle = self.filter.to_lowercase();
        self.filtered = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| needle.is_empty() || r.name.to_lowercase().contains(&needle))
            .map(|(i, _)| i)
            .collect();
        // Keep cursor in-range.
        if let Some(sel) = self.list_state.selected() {
            if sel >= self.filtered.len() {
                self.list_state.select(if self.filtered.is_empty() { None } else { Some(self.filtered.len() - 1) });
            }
        }
    }

    /// True iff at least one removable item remains after the
    /// user's whitelist choices (drives the "would remove
    /// nothing → just skip" path).
    fn would_remove_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| !self.selection.is_kept(&r.name))
            .count()
    }

    fn kept_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| self.selection.is_kept(&r.name))
            .count()
    }

    /// Render the active sub-step into `area`.
    pub fn render(&mut self, frame: &mut Frame<'_>, area: Rect) {
        match self.step {
            PurgeStep::Warning => self.render_warning(frame, area),
            PurgeStep::Scanning => self.render_scanning(frame, area),
            PurgeStep::ListView => self.render_list(frame, area),
            PurgeStep::Confirm => self.render_confirm(frame, area),
            PurgeStep::Executing => self.render_executing(frame, area),
            PurgeStep::Done => self.render_done(frame, area),
            PurgeStep::Aborted => self.render_aborted(frame, area),
        }
    }

    fn render_warning(&self, frame: &mut Frame<'_>, area: Rect) {
        let mut lines = vec![Line::from(Span::styled(
            "Baseline purge — required before MDE can start",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ))];
        lines.push(Line::raw(""));
        for para in purge::RATIONALE.split("\n\n") {
            for wrapped in crate::tui::wrap_paragraph(para, area.width.saturating_sub(4) as usize) {
                lines.push(Line::raw(wrapped));
            }
            lines.push(Line::raw(""));
        }
        lines.push(Line::from(Span::styled(
            "Press Enter to scan installed packages, or q to abort.",
            Style::default().fg(Color::Cyan),
        )));
        let para = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::NONE).padding(Padding::new(2, 2, 1, 1)));
        frame.render_widget(para, area);
    }

    fn render_scanning(&self, frame: &mut Frame<'_>, area: Rect) {
        let para = Paragraph::new(vec![
            Line::from(Span::styled(
                "Scanning installed packages…",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::raw("Querying the RPM database, Flatpak system installations, and Snap (if present)."),
        ])
        .block(Block::default().borders(Borders::NONE).padding(Padding::new(2, 2, 1, 1)));
        frame.render_widget(para, area);
    }

    fn render_list(&mut self, frame: &mut Frame<'_>, area: Rect) {
        // Top counter line + filter input + list + key hints.
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // counter
                Constraint::Length(2), // filter
                Constraint::Min(0),    // list
                Constraint::Length(2), // hints
            ])
            .split(area);

        // Counter.
        let kept = self.kept_count();
        let remove = self.would_remove_count();
        let total = self.rows.len();
        let counter = Paragraph::new(Line::from(vec![
            Span::styled(format!("{kept}"), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" whitelisted  ·  "),
            Span::styled(format!("{remove}"), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" to remove  ·  "),
            Span::styled(format!("{total}"), Style::default().fg(Color::DarkGray)),
            Span::raw(" total found"),
        ]))
        .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(counter, chunks[0]);

        // Filter row.
        let filter_label = if self.filter_editing {
            format!("/{}_", self.filter)
        } else if self.filter.is_empty() {
            "/  (press to filter)".to_string()
        } else {
            format!("/{}  (enter to confirm, esc to clear)", self.filter)
        };
        let filter = Paragraph::new(Line::from(Span::styled(
            filter_label,
            Style::default().fg(if self.filter_editing { Color::Cyan } else { Color::DarkGray }),
        )))
        .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(filter, chunks[1]);

        // List body.
        let blurb = self
            .list_state
            .selected()
            .and_then(|i| self.filtered.get(i))
            .and_then(|&ri| self.rows.get(ri))
            .map(|r| r.blurb.clone())
            .unwrap_or_default();

        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .map(|&i| {
                let row = &self.rows[i];
                let kept = self.selection.is_kept(&row.name);
                let mut spans = Vec::new();
                if let Some(h) = row.family_header {
                    spans.push(Span::styled(
                        format!("── {h} "),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::DIM),
                    ));
                }
                let (marker, style) = if kept {
                    ("[keep]   ", Style::default().fg(Color::Yellow))
                } else {
                    ("[remove] ", Style::default().fg(Color::Red))
                };
                let mut line = if let Some(h) = row.family_header {
                    vec![Span::styled(
                        format!("─ {h} ─\n"),
                        Style::default().fg(Color::DarkGray),
                    )]
                } else {
                    Vec::new()
                };
                let _ = spans;
                line.push(Span::styled(marker, style));
                line.push(Span::raw(row.name.clone()));
                ListItem::new(Line::from(line))
            })
            .collect();

        let list_block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Packages — {} ", blurb));
        let list = List::new(items)
            .block(list_block)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");
        frame.render_stateful_widget(list, chunks[2], &mut self.list_state);

        let hints = Paragraph::new(Line::from(Span::styled(
            "↑↓ move · space/w toggle keep · / filter · a clear-whitelist · enter confirm · q abort",
            Style::default().fg(Color::DarkGray),
        )))
        .block(Block::default().borders(Borders::TOP));
        frame.render_widget(hints, chunks[3]);
    }

    fn render_confirm(&self, frame: &mut Frame<'_>, area: Rect) {
        let remove = self.would_remove_count();
        let kept = self.kept_count();
        let mut lines = vec![Line::from(Span::styled(
            "Confirm baseline purge",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ))];
        lines.push(Line::raw(""));
        lines.push(Line::raw(format!(
            "MDE will remove {remove} package(s) and keep {kept} whitelisted package(s)."
        )));
        lines.push(Line::raw(""));
        lines.push(Line::raw(
            "A rollback manifest will be written to /var/lib/mde/installer/purge-manifest.json — \
             run `mde install --restore-purged` later to reinstall everything removed today.",
        ));
        lines.push(Line::raw(""));
        if kept > 0 {
            lines.push(Line::from(Span::styled(
                format!("⚠  {kept} kept package(s) may conflict with MDE — unsupported configuration."),
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::raw(""));
        }
        lines.push(Line::from(Span::styled(
            "Press y to proceed, n to revise the whitelist, or q to abort the entire install.",
            Style::default().fg(Color::Cyan),
        )));
        let para = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::NONE).padding(Padding::new(2, 2, 1, 1)));
        frame.render_widget(para, area);
    }

    fn render_executing(&self, frame: &mut Frame<'_>, area: Rect) {
        let mut lines = vec![Line::from(Span::styled(
            "Executing baseline purge",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ))];
        lines.push(Line::raw(""));
        lines.push(Line::raw(
            "(INST-3 wires the live `dnf remove` + flatpak uninstall + snap remove pumps. \
             For this build the executor previews the argvs.)",
        ));
        lines.push(Line::raw(""));
        if let Some(scan) = &self.scan {
            let plan = purge::build_plan(scan, &self.selection);
            if let Some(argv) = plan.dnf_remove_argv() {
                lines.push(Line::from(Span::styled(
                    format!("$ {}", argv.join(" ")),
                    Style::default().fg(Color::Green),
                )));
            }
            if let Some(argv) = plan.flatpak_system_argv() {
                lines.push(Line::from(Span::styled(
                    format!("$ {}", argv.join(" ")),
                    Style::default().fg(Color::Green),
                )));
            }
            if let Some(argv) = plan.snap_remove_argv() {
                lines.push(Line::from(Span::styled(
                    format!("$ {}", argv.join(" ")),
                    Style::default().fg(Color::Green),
                )));
            }
            if plan.is_noop() {
                lines.push(Line::raw("(Nothing to remove.)"));
            }
        }
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "Press Enter to acknowledge and continue.",
            Style::default().fg(Color::Cyan),
        )));
        let para = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::NONE).padding(Padding::new(2, 2, 1, 1)));
        frame.render_widget(para, area);
    }

    fn render_done(&self, frame: &mut Frame<'_>, area: Rect) {
        let para = Paragraph::new(vec![
            Line::from(Span::styled(
                "Baseline purge complete",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::raw("Press → to continue to the next step."),
        ])
        .block(Block::default().borders(Borders::NONE).padding(Padding::new(2, 2, 1, 1)));
        frame.render_widget(para, area);
    }

    fn render_aborted(&self, frame: &mut Frame<'_>, area: Rect) {
        let para = Paragraph::new(vec![
            Line::from(Span::styled(
                "Baseline purge declined — install aborted",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::raw(
                "MDE requires a clean desktop baseline. The wizard will exit and /etc/motd will \
                 explain how to resume the installer once you're ready to proceed.",
            ),
        ])
        .block(Block::default().borders(Borders::NONE).padding(Padding::new(2, 2, 1, 1)));
        frame.render_widget(para, area);
    }
}

/// Materialize the row vector from a scan result. Family
/// section headers are attached to the first row of each
/// family so the list visually groups without needing
/// separate widget items.
fn build_rows(scan: &ScanResult) -> Vec<Row> {
    let mut rows = Vec::new();

    let grouped = scan.by_family();
    for fam in PurgeFamily::ordered() {
        let Some(packages) = grouped.get(fam) else { continue; };
        let mut first = true;
        for pkg in packages {
            rows.push(Row {
                name: pkg.name.to_string(),
                family_header: if first { Some(fam.label()) } else { None },
                blurb: pkg.blurb.to_string(),
            });
            first = false;
        }
    }

    if !scan.extra_flatpaks.is_empty() {
        let mut first = true;
        for app in &scan.extra_flatpaks {
            rows.push(Row {
                name: app.clone(),
                family_header: if first { Some(PurgeFamily::FlatpakApp.label()) } else { None },
                blurb: format!("Flatpak application: {app}"),
            });
            first = false;
        }
    }

    if !scan.extra_snaps.is_empty() {
        let mut first = true;
        for snap in &scan.extra_snaps {
            rows.push(Row {
                name: snap.clone(),
                family_header: if first { Some(PurgeFamily::SnapApp.label()) } else { None },
                blurb: format!("Snap application: {snap}"),
            });
            first = false;
        }
    }

    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pages::purge::{scan_installed, InstalledQuery};
    use std::collections::HashSet;

    fn make_query(installed: &[&str]) -> impl InstalledQuery {
        let set: HashSet<String> = installed.iter().map(|s| (*s).to_string()).collect();
        move |pkg: &str| set.contains(pkg)
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn fresh_ui_starts_on_warning() {
        let ui = PurgeUi::new();
        assert_eq!(ui.step(), PurgeStep::Warning);
    }

    #[test]
    fn enter_on_warning_advances_to_scanning() {
        let mut ui = PurgeUi::new();
        ui.handle_key(key(KeyCode::Enter));
        assert_eq!(ui.step(), PurgeStep::Scanning);
    }

    #[test]
    fn q_on_warning_aborts() {
        let mut ui = PurgeUi::new();
        ui.handle_key(key(KeyCode::Char('q')));
        assert_eq!(ui.step(), PurgeStep::Aborted);
    }

    #[test]
    fn empty_scan_jumps_to_done() {
        let mut ui = PurgeUi::new();
        ui.handle_key(key(KeyCode::Enter));
        let q = make_query(&[]);
        ui.run_scan(scan_installed(&q, &[], &[]));
        assert_eq!(ui.step(), PurgeStep::Done);
    }

    #[test]
    fn non_empty_scan_enters_list_view() {
        let mut ui = PurgeUi::new();
        ui.handle_key(key(KeyCode::Enter));
        let q = make_query(&["gnome-shell", "kwin"]);
        ui.run_scan(scan_installed(&q, &[], &[]));
        assert_eq!(ui.step(), PurgeStep::ListView);
        assert_eq!(ui.rows.len(), 2);
        assert_eq!(ui.would_remove_count(), 2);
        assert_eq!(ui.kept_count(), 0);
    }

    #[test]
    fn space_toggles_whitelist_membership() {
        let mut ui = PurgeUi::new();
        ui.handle_key(key(KeyCode::Enter));
        let q = make_query(&["gnome-shell", "kwin", "xfwm4"]);
        ui.run_scan(scan_installed(&q, &[], &[]));

        // Cursor lands on row 0 (gnome-shell by family order).
        assert_eq!(ui.list_state.selected(), Some(0));
        let name0 = ui.rows[0].name.clone();
        ui.handle_key(key(KeyCode::Char(' ')));
        assert!(ui.selection.is_kept(&name0));
        ui.handle_key(key(KeyCode::Char(' ')));
        assert!(!ui.selection.is_kept(&name0));
    }

    #[test]
    fn down_arrow_moves_cursor() {
        let mut ui = PurgeUi::new();
        ui.handle_key(key(KeyCode::Enter));
        let q = make_query(&["gnome-shell", "kwin", "xfwm4"]);
        ui.run_scan(scan_installed(&q, &[], &[]));
        ui.handle_key(key(KeyCode::Down));
        assert_eq!(ui.list_state.selected(), Some(1));
    }

    #[test]
    fn slash_enters_filter_editing_mode() {
        let mut ui = PurgeUi::new();
        ui.handle_key(key(KeyCode::Enter));
        let q = make_query(&["gnome-shell", "kwin"]);
        ui.run_scan(scan_installed(&q, &[], &[]));
        ui.handle_key(key(KeyCode::Char('/')));
        assert!(ui.filter_editing);
        ui.handle_key(key(KeyCode::Char('g')));
        ui.handle_key(key(KeyCode::Char('n')));
        assert_eq!(ui.filter, "gn");
        // Filter should reduce to rows containing "gn".
        assert!(ui.filtered.iter().all(|&i| ui.rows[i].name.contains("gn")));
        ui.handle_key(key(KeyCode::Enter));
        assert!(!ui.filter_editing);
    }

    #[test]
    fn escape_during_filter_clears_and_exits() {
        let mut ui = PurgeUi::new();
        ui.handle_key(key(KeyCode::Enter));
        let q = make_query(&["gnome-shell", "kwin"]);
        ui.run_scan(scan_installed(&q, &[], &[]));
        ui.handle_key(key(KeyCode::Char('/')));
        ui.handle_key(key(KeyCode::Char('z'))); // matches nothing
        assert_eq!(ui.filter, "z");
        ui.handle_key(key(KeyCode::Esc));
        assert!(ui.filter.is_empty());
        assert!(!ui.filter_editing);
        // All rows visible again.
        assert_eq!(ui.filtered.len(), ui.rows.len());
    }

    #[test]
    fn a_clears_whitelist() {
        let mut ui = PurgeUi::new();
        ui.handle_key(key(KeyCode::Enter));
        let q = make_query(&["gnome-shell", "kwin"]);
        ui.run_scan(scan_installed(&q, &[], &[]));
        ui.handle_key(key(KeyCode::Char(' ')));
        assert_eq!(ui.kept_count(), 1);
        ui.handle_key(key(KeyCode::Char('a')));
        assert_eq!(ui.kept_count(), 0);
    }

    #[test]
    fn enter_in_list_advances_to_confirm() {
        let mut ui = PurgeUi::new();
        ui.handle_key(key(KeyCode::Enter));
        let q = make_query(&["gnome-shell"]);
        ui.run_scan(scan_installed(&q, &[], &[]));
        ui.handle_key(key(KeyCode::Enter));
        assert_eq!(ui.step(), PurgeStep::Confirm);
    }

    #[test]
    fn y_at_confirm_advances_to_executing() {
        let mut ui = PurgeUi::new();
        ui.handle_key(key(KeyCode::Enter));
        let q = make_query(&["gnome-shell"]);
        ui.run_scan(scan_installed(&q, &[], &[]));
        ui.handle_key(key(KeyCode::Enter));
        ui.handle_key(key(KeyCode::Char('y')));
        assert_eq!(ui.step(), PurgeStep::Executing);
    }

    #[test]
    fn n_at_confirm_returns_to_list_view() {
        let mut ui = PurgeUi::new();
        ui.handle_key(key(KeyCode::Enter));
        let q = make_query(&["gnome-shell"]);
        ui.run_scan(scan_installed(&q, &[], &[]));
        ui.handle_key(key(KeyCode::Enter));
        assert_eq!(ui.step(), PurgeStep::Confirm);
        ui.handle_key(key(KeyCode::Char('n')));
        assert_eq!(ui.step(), PurgeStep::ListView);
    }

    #[test]
    fn build_rows_attaches_family_headers_to_first_row_per_family() {
        let q = make_query(&["gnome-shell", "gnome-tweaks", "kwin"]);
        let scan = scan_installed(&q, &[], &[]);
        let rows = build_rows(&scan);
        // First gnome row has header; second doesn't. First kde
        // row has header.
        let gnome_rows: Vec<_> = rows.iter().filter(|r| r.name.starts_with("gnome")).collect();
        assert_eq!(gnome_rows.len(), 2);
        assert!(gnome_rows[0].family_header.is_some());
        assert!(gnome_rows[1].family_header.is_none());

        let kde_rows: Vec<_> = rows.iter().filter(|r| r.name == "kwin").collect();
        assert!(kde_rows[0].family_header.is_some());
    }
}
