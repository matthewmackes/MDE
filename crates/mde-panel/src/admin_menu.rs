//! Phase E.13 — right-click Start admin menu (Iced port).
//!
//! The GTK version (mackes-panel/src/admin_menu.rs) shipped a 9-item
//! Fedora admin menu grouped into 5 sections (Shells / Packages /
//! Services / Security / Storage). This Iced port reuses the locked
//! action set verbatim and replaces the GTK `popup_at_widget` flow
//! with a `foot --hold` subprocess spawn per action.
//!
//! On Wayland `foot` is the locked terminal (per CB-3.2). Each
//! action runs in a `foot --hold` so the user can read the output
//! after the command exits. `--hold` retains the window until
//! manual close.

use std::process::Command;

/// A single admin-menu entry.
#[derive(Debug, Clone, Copy)]
pub struct AdminAction {
    pub label: &'static str,
    pub cmd: &'static str,
    pub needs_sudo: bool,
}

/// Section catalog — Q15-locked "Comprehensive 9-item" set.
pub const SECTIONS: &[(&str, &[AdminAction])] = &[
    (
        "Shells",
        &[
            AdminAction {
                label: "Root Terminal",
                cmd: "sudo -i",
                needs_sudo: true,
            },
            AdminAction {
                label: "Edit system file (sudoedit)",
                cmd: "sudoedit /etc/hosts",
                needs_sudo: true,
            },
        ],
    ),
    (
        "Packages",
        &[
            AdminAction {
                label: "DNF update",
                cmd: "sudo dnf upgrade --refresh",
                needs_sudo: true,
            },
            AdminAction {
                label: "DNF history",
                cmd: "sudo dnf history list",
                needs_sudo: true,
            },
        ],
    ),
    (
        "Services",
        &[
            AdminAction {
                label: "systemctl status",
                cmd: "sudo systemctl status",
                needs_sudo: true,
            },
            AdminAction {
                label: "journalctl tail",
                cmd: "sudo journalctl -fxe",
                needs_sudo: true,
            },
        ],
    ),
    (
        "Security",
        &[
            AdminAction {
                label: "SELinux status",
                cmd: "sestatus",
                needs_sudo: false,
            },
            AdminAction {
                label: "Firewall (firewall-cmd)",
                cmd: "sudo firewall-cmd --list-all",
                needs_sudo: true,
            },
        ],
    ),
    (
        "Storage",
        &[AdminAction {
            label: "Clean (dnf cache + journal vacuum 7d)",
            cmd: "sudo dnf clean all && sudo journalctl --vacuum-time=7d",
            needs_sudo: true,
        }],
    ),
];

/// Total action count — Q15 locks this at exactly 9.
#[must_use]
pub fn action_count() -> usize {
    SECTIONS.iter().map(|(_, actions)| actions.len()).sum()
}

/// Build the argv that would spawn a single admin action under
/// `foot --hold`. Pure — no subprocess invocation, ideal for tests.
#[must_use]
pub fn build_foot_argv(action: &AdminAction) -> Vec<String> {
    vec![
        "foot".into(),
        "--hold".into(),
        "--title".into(),
        format!("MDE admin · {}", action.label),
        "sh".into(),
        "-c".into(),
        action.cmd.into(),
    ]
}

/// Spawn the action via `foot --hold`. Non-blocking. Returns the
/// Child handle so callers can adopt it (or drop it to detach).
pub fn spawn_action(action: &AdminAction) -> std::io::Result<std::process::Child> {
    let argv = build_foot_argv(action);
    Command::new(&argv[0]).args(&argv[1..]).spawn()
}

/// Probe whether sudo is currently cached. Drives the UI hint
/// next to actions that `needs_sudo`. Falls through to `false` on
/// any error.
#[must_use]
pub fn sudo_cached() -> bool {
    Command::new("sudo")
        .args(["-n", "-v"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_count_is_locked_at_nine() {
        assert_eq!(action_count(), 9);
    }

    #[test]
    fn five_sections_exactly() {
        assert_eq!(SECTIONS.len(), 5);
    }

    #[test]
    fn section_names_match_lock() {
        let names: Vec<&str> = SECTIONS.iter().map(|(n, _)| *n).collect();
        assert_eq!(names, vec!["Shells", "Packages", "Services", "Security", "Storage"]);
    }

    #[test]
    fn every_label_is_non_empty() {
        for (_, actions) in SECTIONS {
            for action in *actions {
                assert!(!action.label.is_empty());
                assert!(!action.cmd.is_empty());
            }
        }
    }

    #[test]
    fn root_terminal_needs_sudo() {
        let root_term = SECTIONS
            .iter()
            .flat_map(|(_, acts)| acts.iter())
            .find(|a| a.label == "Root Terminal")
            .unwrap();
        assert!(root_term.needs_sudo);
        assert_eq!(root_term.cmd, "sudo -i");
    }

    #[test]
    fn selinux_does_not_need_sudo() {
        let selinux = SECTIONS
            .iter()
            .flat_map(|(_, acts)| acts.iter())
            .find(|a| a.label == "SELinux status")
            .unwrap();
        assert!(!selinux.needs_sudo);
    }

    #[test]
    fn foot_argv_wraps_in_hold_and_titles() {
        let action = AdminAction {
            label: "Root Terminal",
            cmd: "sudo -i",
            needs_sudo: true,
        };
        let argv = build_foot_argv(&action);
        assert_eq!(argv[0], "foot");
        assert_eq!(argv[1], "--hold");
        assert_eq!(argv[2], "--title");
        assert_eq!(argv[3], "MDE admin · Root Terminal");
        assert_eq!(argv[4], "sh");
        assert_eq!(argv[5], "-c");
        assert_eq!(argv[6], "sudo -i");
    }

    #[test]
    fn foot_argv_preserves_compound_commands() {
        let clean_action = SECTIONS
            .iter()
            .flat_map(|(_, acts)| acts.iter())
            .find(|a| a.label.starts_with("Clean"))
            .unwrap();
        let argv = build_foot_argv(clean_action);
        assert!(argv.last().unwrap().contains("&&"));
    }
}
