//! Maintain → System Update panel — `dnf upgrade` wrapper.
//!
//! CB-1.7 partial: replaces the v1.x
//! `mackes/workbench/maintain/system_update.py`. The Python
//! panel streamed dnf's stdout into a live TextView via a
//! GLib io watch; the Iced port drops live streaming for now
//! and ships a run-to-completion semantic (Check / Install
//! buttons → command runs → output appears when done).
//!
//! Live streaming via an `iced::Subscription` + tokio channel
//! is captured as a follow-up. Users who need real-time
//! progress can still run `dnf upgrade` in a terminal — this
//! panel is a convenience surface, not the only entry point.

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length, Padding, Task};
use tokio::process::Command;

#[derive(Debug, Clone, Default)]
pub struct SystemUpdatePanel {
    pub summary: String,
    pub output: String,
    pub busy: bool,
    pub status: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    SummaryLoaded(String),
    CheckClicked,
    InstallClicked,
    Finished {
        argv: String,
        success: bool,
        output: String,
    },
    Error(String),
}

impl SystemUpdatePanel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            summary: "(checking…)".into(),
            ..Self::default()
        }
    }

    pub fn load() -> Task<crate::Message> {
        Task::perform(
            async move { Message::SummaryLoaded(read_summary().await) },
            crate::Message::SystemUpdate,
        )
    }

    pub fn update(&mut self, message: Message) -> Task<crate::Message> {
        match message {
            Message::SummaryLoaded(s) => {
                self.summary = s;
                Task::none()
            }
            Message::Error(msg) => {
                self.status = msg;
                self.busy = false;
                Task::none()
            }
            Message::CheckClicked => {
                if self.busy {
                    return Task::none();
                }
                self.busy = true;
                self.status = "Checking for updates…".into();
                self.output.clear();
                let argv = "dnf check-update".to_string();
                Task::perform(
                    async move {
                        let (success, output) =
                            run_capture(&["dnf", "check-update", "--quiet"]).await;
                        Message::Finished {
                            argv,
                            success,
                            output,
                        }
                    },
                    crate::Message::SystemUpdate,
                )
            }
            Message::InstallClicked => {
                if self.busy {
                    return Task::none();
                }
                self.busy = true;
                self.status = "Installing updates (polkit will prompt for your password)…".into();
                self.output.clear();
                let argv = "pkexec dnf upgrade -y --refresh".to_string();
                Task::perform(
                    async move {
                        let (success, output) =
                            run_capture(&["pkexec", "dnf", "upgrade", "-y", "--refresh"]).await;
                        Message::Finished {
                            argv,
                            success,
                            output,
                        }
                    },
                    crate::Message::SystemUpdate,
                )
            }
            Message::Finished {
                argv,
                success,
                output,
            } => {
                self.busy = false;
                self.output = output;
                self.status = if success {
                    format!("{argv}: ok")
                } else {
                    format!("{argv}: failed (see output)")
                };
                // Refresh the summary line so a successful upgrade
                // shows "(up to date)" without a manual reload.
                Self::load()
            }
        }
    }

    pub fn view(&self) -> Element<'_, crate::Message> {
        let check_btn = {
            let mut b = button(text("Check for updates"));
            if !self.busy {
                b = b.on_press(crate::Message::SystemUpdate(Message::CheckClicked));
            }
            b
        };
        let install_btn = {
            let mut b = button(text("Install all updates"));
            if !self.busy {
                b = b.on_press(crate::Message::SystemUpdate(Message::InstallClicked));
            }
            b
        };

        column![
            text("System Update").size(20),
            text(
                "Install the latest fixes and updates for your machine. \
                 This may take a few minutes."
            )
            .size(13),
            text(&self.summary).size(13),
            row![check_btn, install_btn].spacing(12),
            text("Output").size(16),
            scrollable(
                container(text(&self.output).size(12))
                    .padding(Padding::new(12.0))
                    .width(Length::Fill),
            )
            .height(Length::Fixed(320.0)),
            text(&self.status).size(13),
        ]
        .spacing(12)
        .width(Length::Fill)
        .padding(Padding::new(0.0))
        .into()
    }
}

/// Cheap startup summary — runs `dnf check-update --quiet`
/// and counts the update-eligible package lines. The first
/// "==" header line + blank line are skipped; everything
/// after that is a package row.
async fn read_summary() -> String {
    let (success, out) = run_capture(&["dnf", "check-update", "--quiet"]).await;
    // dnf check-update exits with code 100 when updates are
    // available, 0 when up to date, non-{0,100} on error.
    // run_capture coalesces to (success_bool, output) — we
    // can't tell 100 vs 0 from a bool, so we count package
    // lines instead.
    let count = summarise_check_update(&out);
    if count == 0 {
        if success {
            "(up to date)".into()
        } else {
            "(could not check — dnf returned no parseable output)".into()
        }
    } else {
        format!("{count} package(s) available to update")
    }
}

/// Pure helper for the summary line. Counts lines that look
/// like a package update row (3+ whitespace-separated columns:
/// `<name>.<arch>` `<version>` `<repo>`). Skips header/blank
/// lines + everything inside an `Obsoleting Packages` block
/// (those rows still parse as packages but represent the
/// obsoletion graph, not user-facing updates).
#[must_use]
pub fn summarise_check_update(output: &str) -> usize {
    let mut count = 0;
    let mut in_obsoleting = false;
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Obsoleting") {
            in_obsoleting = true;
            continue;
        }
        if in_obsoleting {
            // The obsoletion block ends at the next blank line
            // or top-level header (no leading whitespace, no
            // `.` in column 0).
            if trimmed.is_empty() {
                in_obsoleting = false;
            }
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }
        let cols: Vec<&str> = trimmed.split_whitespace().collect();
        if cols.len() >= 3 && cols[0].contains('.') {
            count += 1;
        }
    }
    count
}

/// Run a command to completion, capturing stdout+stderr. Returns
/// `(success, combined_output)`. Empty output on launch failure.
async fn run_capture(argv: &[&str]) -> (bool, String) {
    let Some((bin, args)) = argv.split_first() else {
        return (false, "empty command".into());
    };
    let Ok(output) = Command::new(bin).args(args).output().await else {
        return (false, format!("{bin} not found on PATH"));
    };
    let mut combined = String::from_utf8(output.stdout).unwrap_or_default();
    let stderr = String::from_utf8(output.stderr).unwrap_or_default();
    if !stderr.is_empty() {
        if !combined.ends_with('\n') && !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&stderr);
    }
    (output.status.success(), combined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarise_check_update_counts_update_rows() {
        let out = "\
Last metadata expiration check: 0:01:00 ago.

firefox.x86_64            128.0.1-1.fc44      updates
kernel.x86_64             6.10.0-1.fc44       updates
glibc.x86_64              2.39-5.fc44         updates
";
        assert_eq!(summarise_check_update(out), 3);
    }

    #[test]
    fn summarise_check_update_zero_when_up_to_date() {
        assert_eq!(summarise_check_update(""), 0);
        assert_eq!(
            summarise_check_update("Last metadata expiration check: now.\n"),
            0
        );
    }

    #[test]
    fn summarise_check_update_skips_obsoleting_block() {
        let out = "\
firefox.x86_64    128.0.1-1.fc44   updates

Obsoleting Packages
  oldpkg.noarch  1.0-1.fc43       updates
      replaces:  oldpkg.noarch    1.0-1.fc42
";
        // Only the real firefox row should count.
        assert_eq!(summarise_check_update(out), 1);
    }

    #[test]
    fn loaded_summary_replaces_initial_checking_placeholder() {
        let mut panel = SystemUpdatePanel::new();
        assert!(panel.summary.contains("checking"));
        let _ = panel.update(Message::SummaryLoaded("(up to date)".into()));
        assert_eq!(panel.summary, "(up to date)");
    }

    #[test]
    fn check_clicked_while_busy_is_noop() {
        let mut panel = SystemUpdatePanel::new();
        panel.busy = true;
        panel.status = "Checking…".into();
        let _ = panel.update(Message::CheckClicked);
        assert_eq!(panel.status, "Checking…");
    }

    #[test]
    fn install_clicked_while_busy_is_noop() {
        let mut panel = SystemUpdatePanel::new();
        panel.busy = true;
        panel.status = "Installing…".into();
        let _ = panel.update(Message::InstallClicked);
        assert_eq!(panel.status, "Installing…");
    }

    #[test]
    fn finished_success_records_ok_status_and_clears_busy() {
        let mut panel = SystemUpdatePanel::new();
        panel.busy = true;
        let _ = panel.update(Message::Finished {
            argv: "dnf check-update".into(),
            success: true,
            output: "firefox.x86_64    1.0    updates".into(),
        });
        assert!(!panel.busy);
        assert!(panel.status.contains("ok"));
        assert!(panel.output.contains("firefox"));
    }

    #[test]
    fn finished_failure_includes_failed_marker() {
        let mut panel = SystemUpdatePanel::new();
        panel.busy = true;
        let _ = panel.update(Message::Finished {
            argv: "pkexec dnf upgrade".into(),
            success: false,
            output: "polkit denied".into(),
        });
        assert!(panel.status.contains("failed"));
        assert!(panel.output.contains("polkit"));
    }

    #[test]
    fn error_message_clears_busy_and_stores_msg() {
        let mut panel = SystemUpdatePanel::new();
        panel.busy = true;
        let _ = panel.update(Message::Error("dnf not found".into()));
        assert_eq!(panel.status, "dnf not found");
        assert!(!panel.busy);
    }
}
