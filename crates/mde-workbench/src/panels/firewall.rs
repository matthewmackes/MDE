//! Network → Firewall panel — firewalld via `firewall-cmd`.
//!
//! CB-1.8 partial: replaces the v1.x
//! `mackes/workbench/network/firewall.py`. Two controls:
//! a default-zone pick_list + a per-service toggle for the
//! enabled service set. Reads via `firewall-cmd --get-…` /
//! `--list-…`; writes via `pkexec firewall-cmd …` for the
//! state-change paths (permanent + reload).

use iced::widget::{checkbox, column, container, pick_list, row, scrollable, text};
use iced::{Element, Length, Task};
use mde_theme::Palette;

use crate::controls::{variant_button, ButtonVariant};
use tokio::process::Command;

/// Curated list of common firewalld services the panel exposes
/// as per-row toggles. Matches the canonical set the v1.x
/// Python panel rendered; users with custom services can still
/// edit them via `firewall-cmd` directly.
pub const COMMON_SERVICES: &[&str] = &[
    "ssh",
    "http",
    "https",
    "dhcpv6-client",
    "mdns",
    "samba-client",
    "cockpit",
    "vnc-server",
];

#[derive(Debug, Clone, Default)]
pub struct FirewallPanel {
    pub firewalld_available: bool,
    pub zones: Vec<String>,
    pub default_zone: String,
    pub enabled_services: Vec<String>,
    pub status: String,
    pub busy: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded {
        firewalld_available: bool,
        zones: Vec<String>,
        default_zone: String,
        enabled_services: Vec<String>,
    },
    Error(String),
    DefaultZoneSelected(String),
    ServiceToggled {
        service: String,
        enable: bool,
    },
    OperationFinished(Result<String, String>),
    RefreshClicked,
}

impl FirewallPanel {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load() -> Task<crate::Message> {
        Task::perform(
            async move {
                let version = run_firewall_cmd(&["--version"]).await;
                let firewalld_available = !version.is_empty();
                if !firewalld_available {
                    return Message::Loaded {
                        firewalld_available,
                        zones: Vec::new(),
                        default_zone: String::new(),
                        enabled_services: Vec::new(),
                    };
                }
                let zones_raw = run_firewall_cmd(&["--get-zones"]).await;
                let default_zone = run_firewall_cmd(&["--get-default-zone"]).await;
                let services_raw = run_firewall_cmd(&["--list-services"]).await;
                Message::Loaded {
                    firewalld_available,
                    zones: parse_space_separated(&zones_raw),
                    default_zone: default_zone.trim().to_string(),
                    enabled_services: parse_space_separated(&services_raw),
                }
            },
            crate::Message::Firewall,
        )
    }

    pub fn update(&mut self, message: Message) -> Task<crate::Message> {
        match message {
            Message::Loaded {
                firewalld_available,
                zones,
                default_zone,
                enabled_services,
            } => {
                self.firewalld_available = firewalld_available;
                self.zones = zones;
                self.default_zone = if self.zones.contains(&default_zone) {
                    default_zone
                } else {
                    self.zones.first().cloned().unwrap_or_default()
                };
                self.enabled_services = enabled_services;
                self.status.clear();
                self.busy = false;
                Task::none()
            }
            Message::Error(msg) => {
                self.status = msg;
                self.busy = false;
                Task::none()
            }
            Message::DefaultZoneSelected(zone) => {
                if self.busy {
                    return Task::none();
                }
                self.busy = true;
                self.default_zone = zone.clone();
                self.status = format!("Setting default zone to {zone} (polkit will prompt)…");
                Task::perform(
                    async move { Message::OperationFinished(set_default_zone(&zone).await) },
                    crate::Message::Firewall,
                )
            }
            Message::ServiceToggled { service, enable } => {
                if self.busy {
                    return Task::none();
                }
                self.busy = true;
                self.status = format!(
                    "{} {service} (polkit will prompt)…",
                    if enable { "Enabling" } else { "Disabling" },
                );
                Task::perform(
                    async move { Message::OperationFinished(toggle_service(&service, enable).await) },
                    crate::Message::Firewall,
                )
            }
            Message::OperationFinished(result) => {
                self.busy = false;
                self.status = match result {
                    Ok(msg) => msg,
                    Err(msg) => msg,
                };
                // Reload to reflect the new state.
                Self::load()
            }
            Message::RefreshClicked => {
                if self.busy {
                    return Task::none();
                }
                self.busy = true;
                self.status = "Refreshing…".into();
                Self::load()
            }
        }
    }

    pub fn view(&self) -> Element<'_, crate::Message> {
        if !self.firewalld_available {
            return column![
                text("firewalld unavailable").size(18),
                text(
                    "MDE talks to the firewall through `firewall-cmd`. \
                     Install firewalld and ensure the service is running, \
                     then refresh this panel.",
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
            (!self.busy).then(|| crate::Message::Firewall(Message::RefreshClicked)),
            Palette::dark(),
        );

        let zone_pick: pick_list::PickList<'_, String, _, _, crate::Message> = pick_list(
            self.zones.clone(),
            current_or_none(&self.zones, &self.default_zone),
            |v| crate::Message::Firewall(Message::DefaultZoneSelected(v)),
        );

        let service_rows = COMMON_SERVICES.iter().fold(column![], |col, service| {
            let svc = (*service).to_string();
            let is_on = self.enabled_services.iter().any(|s| s == service);
            let busy = self.busy;
            let cb = checkbox(*service, is_on).on_toggle(move |enable| {
                let _ = busy;
                crate::Message::Firewall(Message::ServiceToggled {
                    service: svc.clone(),
                    enable,
                })
            });
            col.push(cb)
        });

        column![
            row![
                text("Default zone").width(Length::Fixed(180.0)),
                zone_pick,
                refresh_btn,
            ]
            .spacing(12),
            text("Services").size(16),
            scrollable(container(service_rows.spacing(4))).height(Length::Fixed(240.0)),
            text(format!(
                "{} service(s) enabled in zone {}",
                self.enabled_services.len(),
                self.default_zone,
            ))
            .size(13),
            text(&self.status).size(13),
        ]
        .spacing(12)
        .width(Length::Fill)
        .into()
    }
}

fn current_or_none(list: &[String], value: &str) -> Option<String> {
    list.iter().find(|v| *v == value).cloned()
}

/// firewall-cmd's `--get-zones` / `--list-services` output is a
/// single line of whitespace-separated tokens. Empty input
/// produces an empty Vec.
#[must_use]
pub fn parse_space_separated(raw: &str) -> Vec<String> {
    raw.split_whitespace().map(String::from).collect()
}

/// Shell out to `firewall-cmd` with the given args. Returns
/// stdout on success; empty on failure (used as the
/// "unavailable" signal in the read paths).
pub async fn run_firewall_cmd(args: &[&str]) -> String {
    let Ok(output) = Command::new("firewall-cmd").args(args).output().await else {
        return String::new();
    };
    if !output.status.success() {
        return String::new();
    }
    String::from_utf8(output.stdout).unwrap_or_default()
}

/// Set the default zone. Returns a human-readable status
/// message on success or failure.
pub async fn set_default_zone(zone: &str) -> Result<String, String> {
    let success = run_pkexec_firewall_cmd(&["--set-default-zone", zone]).await;
    if success {
        Ok(format!("Default zone set to {zone}."))
    } else {
        Err(format!(
            "Setting default zone to {zone} failed (polkit cancelled or daemon down)."
        ))
    }
}

/// Add or remove a service from the default zone, permanent +
/// reload. firewalld's `--permanent` is needed so the change
/// survives a daemon restart; the `--reload` makes it active
/// immediately.
pub async fn toggle_service(service: &str, enable: bool) -> Result<String, String> {
    let flag = if enable {
        "--add-service"
    } else {
        "--remove-service"
    };
    let ok = run_pkexec_firewall_cmd(&[flag, service, "--permanent"]).await;
    if !ok {
        return Err(format!(
            "{} {service} failed (polkit cancelled or service unknown).",
            if enable { "Enabling" } else { "Disabling" },
        ));
    }
    let reload_ok = run_pkexec_firewall_cmd(&["--reload"]).await;
    if !reload_ok {
        return Err("Service updated but firewall-cmd --reload failed.".into());
    }
    Ok(format!(
        "{} {service}.",
        if enable { "Enabled" } else { "Disabled" },
    ))
}

async fn run_pkexec_firewall_cmd(args: &[&str]) -> bool {
    let mut argv = vec!["firewall-cmd"];
    argv.extend_from_slice(args);
    let Ok(output) = Command::new("pkexec").args(&argv).output().await else {
        return false;
    };
    output.status.success()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_services_lock_is_eight_entries() {
        assert_eq!(COMMON_SERVICES.len(), 8);
        assert!(COMMON_SERVICES.contains(&"ssh"));
        assert!(COMMON_SERVICES.contains(&"https"));
    }

    #[test]
    fn parse_space_separated_handles_typical_input() {
        let raw = "FedoraServer FedoraWorkstation block dmz drop\n";
        let zones = parse_space_separated(raw);
        assert_eq!(zones.len(), 5);
        assert_eq!(zones[0], "FedoraServer");
        assert_eq!(zones[4], "drop");
    }

    #[test]
    fn parse_space_separated_collapses_runs_of_whitespace() {
        let raw = "  ssh    http https \t mdns  ";
        let services = parse_space_separated(raw);
        assert_eq!(services, vec!["ssh", "http", "https", "mdns"]);
    }

    #[test]
    fn parse_space_separated_empty_on_empty_or_whitespace() {
        assert!(parse_space_separated("").is_empty());
        assert!(parse_space_separated("   \n  \t  ").is_empty());
    }

    #[test]
    fn loaded_records_state_and_falls_back_to_first_zone_when_default_unknown() {
        let mut panel = FirewallPanel::new();
        let _ = panel.update(Message::Loaded {
            firewalld_available: true,
            zones: vec!["public".into(), "trusted".into()],
            default_zone: "vanished".into(),
            enabled_services: vec!["ssh".into(), "http".into()],
        });
        assert!(panel.firewalld_available);
        assert_eq!(panel.default_zone, "public");
        assert_eq!(panel.enabled_services, vec!["ssh", "http"]);
    }

    #[test]
    fn loaded_preserves_known_default_zone() {
        let mut panel = FirewallPanel::new();
        let _ = panel.update(Message::Loaded {
            firewalld_available: true,
            zones: vec!["public".into(), "trusted".into()],
            default_zone: "trusted".into(),
            enabled_services: vec![],
        });
        assert_eq!(panel.default_zone, "trusted");
    }

    #[test]
    fn loaded_firewalld_unavailable_clears_state() {
        let mut panel = FirewallPanel::new();
        let _ = panel.update(Message::Loaded {
            firewalld_available: false,
            zones: Vec::new(),
            default_zone: String::new(),
            enabled_services: Vec::new(),
        });
        assert!(!panel.firewalld_available);
    }

    #[test]
    fn default_zone_selected_while_busy_is_noop() {
        let mut panel = FirewallPanel::new();
        panel.busy = true;
        panel.default_zone = "public".into();
        let _ = panel.update(Message::DefaultZoneSelected("trusted".into()));
        assert_eq!(panel.default_zone, "public");
    }

    #[test]
    fn service_toggled_while_busy_is_noop() {
        let mut panel = FirewallPanel::new();
        panel.busy = true;
        panel.status = "Applying…".into();
        let _ = panel.update(Message::ServiceToggled {
            service: "ssh".into(),
            enable: true,
        });
        assert_eq!(panel.status, "Applying…");
    }

    #[test]
    fn operation_finished_ok_carries_status_and_clears_busy() {
        let mut panel = FirewallPanel::new();
        panel.busy = true;
        let _ = panel.update(Message::OperationFinished(Ok("Enabled ssh.".into())));
        assert!(!panel.busy);
        assert_eq!(panel.status, "Enabled ssh.");
    }

    #[test]
    fn operation_finished_err_carries_error_and_clears_busy() {
        let mut panel = FirewallPanel::new();
        panel.busy = true;
        let _ = panel.update(Message::OperationFinished(Err("polkit denied".into())));
        assert!(!panel.busy);
        assert_eq!(panel.status, "polkit denied");
    }

    #[test]
    fn refresh_clicked_while_busy_is_noop() {
        let mut panel = FirewallPanel::new();
        panel.busy = true;
        panel.status = "stale".into();
        let _ = panel.update(Message::RefreshClicked);
        assert_eq!(panel.status, "stale");
    }

    #[test]
    fn error_message_clears_busy_and_stores_msg() {
        let mut panel = FirewallPanel::new();
        panel.busy = true;
        let _ = panel.update(Message::Error("firewall-cmd not found".into()));
        assert_eq!(panel.status, "firewall-cmd not found");
        assert!(!panel.busy);
    }
}
