//! Printers panel — list configured CUPS queues + default
//! picker, backed by `lpstat` + `lpoptions`.
//!
//! CB-1.4.c: there was no v1.x `mackes/workbench/devices/
//! printers.py` to port — this is a fresh build matching
//! the acceptance criterion. The alternative (zbus to
//! cups-browsed) was rejected: cups-browsed's D-Bus
//! surface isn't yet stable enough to depend on, and `lpstat`
//! / `lpoptions` ship with CUPS itself which is the
//! installed-by-default print stack on Fedora workstation.
//!
//! Acceptance covered:
//!   * lists configured printers (lpstat -p, parsed into
//!     queue names)
//!   * default picker writes the default queue
//!     (lpoptions -d <queue>)

use iced::widget::{column, pick_list, row, text};
use iced::{Element, Length, Task};
use mde_theme::Palette;

use crate::controls::{variant_button, ButtonVariant};
use tokio::process::Command;

#[derive(Debug, Clone, Default)]
pub struct PrintersPanel {
    /// Whether `lpstat -r` reported the scheduler running. When
    /// false, the empty-state body explains how to start it
    /// instead of rendering empty pickers.
    pub cups_running: bool,
    pub queues: Vec<String>,
    pub default_queue: String,
    pub status: String,
    pub busy: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded {
        cups_running: bool,
        queues: Vec<String>,
        default_queue: String,
    },
    Error(String),
    DefaultSelected(String),
    DefaultApplied,
}

impl PrintersPanel {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load() -> Task<crate::Message> {
        Task::perform(
            async move {
                let scheduler_status = run_lpstat(&["-r"]).await;
                let cups_running = scheduler_status
                    .to_ascii_lowercase()
                    .contains("scheduler is running");
                if !cups_running {
                    return Message::Loaded {
                        cups_running,
                        queues: Vec::new(),
                        default_queue: String::new(),
                    };
                }
                let queues_raw = run_lpstat(&["-p"]).await;
                let default_raw = run_lpstat(&["-d"]).await;
                Message::Loaded {
                    cups_running,
                    queues: parse_lpstat_p(&queues_raw),
                    default_queue: parse_lpstat_d(&default_raw),
                }
            },
            crate::Message::Printers,
        )
    }

    pub fn update(&mut self, message: Message) -> Task<crate::Message> {
        match message {
            Message::Loaded {
                cups_running,
                queues,
                default_queue,
            } => {
                self.cups_running = cups_running;
                self.queues = queues;
                self.default_queue = if self.queues.contains(&default_queue) {
                    default_queue
                } else {
                    self.queues.first().cloned().unwrap_or_default()
                };
                self.status.clear();
                self.busy = false;
                Task::none()
            }
            Message::Error(msg) => {
                self.status = msg;
                self.busy = false;
                Task::none()
            }
            Message::DefaultSelected(queue) => {
                if self.busy {
                    return Task::none();
                }
                self.default_queue = queue.clone();
                self.busy = true;
                self.status = "Applying…".into();
                Task::perform(
                    async move {
                        let _ = run_lpoptions(&["-d", &queue]).await;
                        Message::DefaultApplied
                    },
                    crate::Message::Printers,
                )
            }
            Message::DefaultApplied => {
                self.status = "Applied.".into();
                self.busy = false;
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, crate::Message> {
        if !self.cups_running {
            return column![
                text("CUPS scheduler unreachable").size(18),
                text(
                    "MDE talks to printers through CUPS (`lpstat`/`lpoptions`). \
                     Start the cups service (`systemctl start cups`) and \
                     reopen this panel to pick a default queue.",
                )
                .size(13),
            ]
            .spacing(8)
            .width(Length::Fill)
            .into();
        }

        if self.queues.is_empty() {
            return column![
                text("No printers configured").size(18),
                text(
                    "Add a queue from CUPS' web interface \
                     (http://localhost:631) or by running \
                     `lpadmin -p <name> -E -v <uri>`. \
                     Reopen this panel once a queue is added.",
                )
                .size(13),
            ]
            .spacing(8)
            .width(Length::Fill)
            .into();
        }

        // UX-7.a — refresh routed through the shared button variant.
        let refresh_btn = variant_button(
            "Refresh",
            ButtonVariant::Ghost,
            (!self.busy).then(|| crate::Message::PrintersRefresh),
            Palette::dark(),
        );

        let queues = self.queues.clone();
        let default_pick: pick_list::PickList<'_, String, _, _, crate::Message> = pick_list(
            queues,
            current_or_none(&self.queues, &self.default_queue),
            |v| crate::Message::Printers(Message::DefaultSelected(v)),
        );

        column![
            row![
                text("Default printer").width(Length::Fixed(180.0)),
                default_pick,
            ]
            .spacing(12),
            text(format!("Queues configured: {}", self.queues.len())).size(13),
            row![refresh_btn, text(&self.status).size(13)].spacing(12),
        ]
        .spacing(12)
        .width(Length::Fill)
        .into()
    }
}

fn current_or_none(list: &[String], value: &str) -> Option<String> {
    list.iter().find(|v| *v == value).cloned()
}

/// Parse `lpstat -p` output into a Vec of queue names.
///
/// The output format is one line per queue, of the form:
///   "printer hp-laserjet is idle.  enabled since Mon …"
///   "printer epson-l3210 disabled since …"
///
/// We only need the second word (the queue name). Lines that
/// don't start with "printer " are skipped (CUPS may emit
/// "no destinations added." or scheduler-state preambles).
#[must_use]
pub fn parse_lpstat_p(raw: &str) -> Vec<String> {
    raw.lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let rest = trimmed.strip_prefix("printer ")?;
            rest.split_whitespace().next().map(str::to_string)
        })
        .filter(|n| !n.is_empty())
        .collect()
}

/// Parse `lpstat -d` output into a single default-queue name.
///
/// Output forms:
///   "system default destination: hp-laserjet"
///   "no system default destination"
///
/// Returns the empty string when no default is set, matching
/// the panel's "fall back to first listed" behaviour.
#[must_use]
pub fn parse_lpstat_d(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("system default destination:") {
        rest.trim().to_string()
    } else {
        String::new()
    }
}

/// Shell out to `lpstat` with the given args. Returns `""`
/// on any error (binary missing, non-zero exit, decode
/// failure) so callers can use empty as the "unavailable"
/// signal without bubbling Result.
pub async fn run_lpstat(args: &[&str]) -> String {
    let Ok(output) = Command::new("lpstat").args(args).output().await else {
        return String::new();
    };
    if !output.status.success() {
        // lpstat returns non-zero when the scheduler is down,
        // but it still writes a useful "scheduler is not
        // running" line on stderr. Surface stderr in that
        // case so parse_lpstat_p / the cups_running check
        // can see the actual state.
        return String::from_utf8(output.stderr).unwrap_or_default();
    }
    String::from_utf8(output.stdout).unwrap_or_default()
}

/// Shell out to `lpoptions` with the given args. Same
/// error-swallowing contract as [`run_lpstat`].
pub async fn run_lpoptions(args: &[&str]) -> String {
    let Ok(output) = Command::new("lpoptions").args(args).output().await else {
        return String::new();
    };
    if !output.status.success() {
        return String::from_utf8(output.stderr).unwrap_or_default();
    }
    String::from_utf8(output.stdout).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lpstat_p_extracts_queue_names_from_typical_output() {
        let raw = "\
printer hp-laserjet is idle.  enabled since Mon 01 Jan 2024 12:00:00 AM EST
printer epson-l3210 disabled since Tue 02 Jan 2024 02:30:00 PM EST -
\tFiltering started but no incoming jobs accepted
";
        assert_eq!(parse_lpstat_p(raw), vec!["hp-laserjet", "epson-l3210"]);
    }

    #[test]
    fn parse_lpstat_p_skips_non_printer_lines() {
        let raw = "\
no destinations added.
scheduler is running
printer real-queue is idle.
";
        assert_eq!(parse_lpstat_p(raw), vec!["real-queue"]);
    }

    #[test]
    fn parse_lpstat_p_empty_on_empty_or_no_destinations() {
        assert!(parse_lpstat_p("").is_empty());
        assert!(parse_lpstat_p("no destinations added.\n").is_empty());
    }

    #[test]
    fn parse_lpstat_d_extracts_default_queue() {
        assert_eq!(
            parse_lpstat_d("system default destination: hp-laserjet\n"),
            "hp-laserjet"
        );
    }

    #[test]
    fn parse_lpstat_d_empty_when_no_default() {
        assert_eq!(parse_lpstat_d("no system default destination\n"), "");
        assert_eq!(parse_lpstat_d(""), "");
    }

    #[test]
    fn loaded_with_cups_down_clears_lists_and_busy() {
        let mut panel = PrintersPanel::new();
        panel.busy = true;
        let _ = panel.update(Message::Loaded {
            cups_running: false,
            queues: Vec::new(),
            default_queue: String::new(),
        });
        assert!(!panel.cups_running);
        assert!(!panel.busy);
        assert!(panel.queues.is_empty());
    }

    #[test]
    fn loaded_with_unknown_default_falls_back_to_first_listed() {
        let mut panel = PrintersPanel::new();
        let _ = panel.update(Message::Loaded {
            cups_running: true,
            queues: vec!["alpha".into(), "beta".into()],
            default_queue: "vanished".into(),
        });
        assert_eq!(panel.default_queue, "alpha");
    }

    #[test]
    fn loaded_with_known_default_preserves_selection() {
        let mut panel = PrintersPanel::new();
        let _ = panel.update(Message::Loaded {
            cups_running: true,
            queues: vec!["alpha".into(), "beta".into()],
            default_queue: "beta".into(),
        });
        assert_eq!(panel.default_queue, "beta");
    }

    #[test]
    fn default_selected_while_busy_is_noop() {
        let mut panel = PrintersPanel::new();
        panel.busy = true;
        panel.default_queue = "alpha".into();
        let _ = panel.update(Message::DefaultSelected("beta".into()));
        assert_eq!(panel.default_queue, "alpha");
    }

    #[test]
    fn applied_clears_busy_and_records_status() {
        let mut panel = PrintersPanel::new();
        panel.busy = true;
        panel.status = "Applying…".into();
        let _ = panel.update(Message::DefaultApplied);
        assert!(!panel.busy);
        assert_eq!(panel.status, "Applied.");
    }

    #[test]
    fn error_message_clears_busy_and_stores_msg() {
        let mut panel = PrintersPanel::new();
        panel.busy = true;
        let _ = panel.update(Message::Error("lpadmin: not found".into()));
        assert_eq!(panel.status, "lpadmin: not found");
        assert!(!panel.busy);
    }
}
