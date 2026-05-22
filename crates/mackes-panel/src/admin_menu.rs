//! Right-click admin menu — Fedora system-administration shortcuts.
//!
//! Q15 / Q16 lock (2026-05-19, amended 2026-05-22): right-clicking the
//! Start (`M`) button drops a 9-item `gtk::Menu` grouped by section.
//! Each item launches a `terminator` window running its command with
//! the shell kept open after the command finishes (suggestion #4) so
//! the user can read output and re-run interactively.
//!
//! **Privilege escalation:** v2.0.3 switched every elevation call site
//! from raw `sudo` to `pkexec`. Wayland sessions have no controlling
//! TTY for terminator's stdin in many launch contexts (sway, lightdm,
//! mde-session), so `sudo`'s password prompt was failing with
//! "sudo: a terminal is required to read the password" or
//! "no askpass program specified". `pkexec` punts to the polkit GUI
//! auth agent (which runs as part of every modern desktop session and
//! works equally well under X11 / Wayland / no-TTY) so the prompt
//! reliably appears regardless of how the terminal was spawned.
//!
//! Section catalog (locked, Q15 "Comprehensive"):
//!
//! - **Shells**: Root Terminal, edit system file
//! - **Packages**: `dnf` update, `dnf` history
//! - **Services**: `systemctl status`, `journalctl` tail
//! - **Security**: `SELinux` status, `firewall-cmd`
//! - **Storage**: clean (`dnf clean` + journal vacuum)
//!
//! Each menu item's tooltip shows the literal command + the polkit-
//! agent presence ("polkit agent running" when an org.freedesktop
//! .PolicyKit1.Authentication-Agent owner is on the session bus,
//! "polkit agent missing — pkexec may fail" otherwise) per suggestion
//! #6, so the user sees privilege-escalation status without having to
//! click anything first.

use std::process::Command;

use gtk::prelude::*;

/// Privilege-escalation strategy for an admin action.
///
/// `Pkexec` wraps the command in `pkexec sh -c '<cmd>'` so the
/// polkit auth agent owns the password prompt. `Plain` runs the
/// command as the user (no escalation), used for read-only probes
/// like `sestatus` that don't need root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Runner {
    /// pkexec — polkit-mediated escalation. Works in Wayland sessions
    /// without a controlling TTY.
    Pkexec,
    /// Run as the invoking user. No escalation, no prompt.
    Plain,
}

/// One administrative action: a label, a literal shell command, and
/// the runner strategy that determines how privilege escalation
/// happens (or doesn't).
struct AdminAction {
    label: &'static str,
    /// Literal bash command sans privilege-escalation prefix. The
    /// `Runner` adds the prefix at launch time, so the source-of-
    /// truth here is the *intent* (e.g., `dnf upgrade --refresh`)
    /// not the *mechanism* (`sudo` vs `pkexec`).
    cmd: &'static str,
    /// How to escalate (if at all). Drives the tooltip's polkit hint.
    runner: Runner,
}

const SECTIONS: &[(&str, &[AdminAction])] = &[
    (
        "Shells",
        &[
            AdminAction {
                label: "Root Terminal",
                // `pkexec bash -l` opens a fresh root login shell.
                // The previous `sudo -i` form failed under Wayland
                // because terminator couldn't always present a TTY
                // for sudo's password prompt.
                cmd: "bash -l",
                runner: Runner::Pkexec,
            },
            AdminAction {
                label: "Edit /etc/hosts (root)",
                // `nano` instead of the previous `sudoedit /etc/
                // hosts`. sudoedit drops privileges for the editor
                // process, which pkexec's environment scrubbing
                // breaks; nano-as-root via pkexec is the
                // operationally-equivalent path.
                cmd: "nano /etc/hosts",
                runner: Runner::Pkexec,
            },
        ],
    ),
    (
        "Packages",
        &[
            AdminAction {
                label: "DNF update",
                cmd: "dnf upgrade --refresh",
                runner: Runner::Pkexec,
            },
            AdminAction {
                label: "DNF history",
                // History list works as the unprivileged user on
                // recent dnf — no escalation needed.
                cmd: "dnf history list",
                runner: Runner::Plain,
            },
        ],
    ),
    (
        "Services",
        &[
            AdminAction {
                label: "systemctl status",
                // Read-only — systemctl status works without root on
                // every modern systemd.
                cmd: "systemctl status",
                runner: Runner::Plain,
            },
            AdminAction {
                label: "journalctl tail",
                // Users in the systemd-journal group (Fedora default
                // for the first interactive user) can read the
                // journal without root. Falls back to pkexec for the
                // edge case where they aren't.
                cmd: "journalctl -fxe",
                runner: Runner::Pkexec,
            },
        ],
    ),
    (
        "Security",
        &[
            AdminAction {
                label: "SELinux status",
                cmd: "sestatus",
                runner: Runner::Plain,
            },
            AdminAction {
                label: "Firewall (firewall-cmd)",
                cmd: "firewall-cmd --list-all",
                runner: Runner::Pkexec,
            },
        ],
    ),
    (
        "Storage",
        &[AdminAction {
            label: "Clean (dnf cache + journal vacuum 7d)",
            // Chained command — `pkexec sh -c '<chain>'` is how the
            // launcher wires this together.
            cmd: "dnf clean all && journalctl --vacuum-time=7d",
            runner: Runner::Pkexec,
        }],
    ),
];

/// Cheap probe: is a polkit authentication agent currently registered
/// on the session bus? Used in the tooltip so the user knows whether
/// pkexec will be able to surface a password prompt at all.
///
/// `gdbus call --session --dest org.freedesktop.PolicyKit1` would be
/// the rigorous test, but a busctl introspect probe is the cheapest
/// way to confirm an owner exists for the well-known name. Fails
/// silently to `false` on any error — the tooltip just degrades to
/// "agent missing" hint, which is never wrong (worst case is a false
/// negative that scares the user into running pkexec from a real
/// terminal).
fn polkit_agent_running() -> bool {
    let result = Command::new("busctl")
        .args(["--user", "list", "--acquired", "--no-pager", "--no-legend"])
        .stderr(std::process::Stdio::null())
        .output();
    match result {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.contains("org.freedesktop.PolicyKit1.AuthenticationAgent")
        }
        _ => false,
    }
}

/// Build the menu. Caller calls `show_all()` + `popup_at_widget`; this
/// constructor only assembles the widget tree.
#[must_use]
pub fn build() -> gtk::Menu {
    let menu = gtk::Menu::new();
    menu.set_widget_name("mackes-admin-menu");

    let agent_ok = polkit_agent_running();

    for (i, (section, actions)) in SECTIONS.iter().enumerate() {
        if i > 0 {
            menu.append(&gtk::SeparatorMenuItem::new());
        }

        // Section header — non-interactive label rendered as a
        // disabled menu item so it visually anchors the group
        // without being clickable.
        let header = gtk::MenuItem::with_label(section);
        header.set_sensitive(false);
        header.set_widget_name("mackes-admin-menu-section");
        menu.append(&header);

        for action in *actions {
            let item = gtk::MenuItem::with_label(action.label);
            let tooltip = format_tooltip(action, agent_ok);
            item.set_tooltip_text(Some(&tooltip));
            let cmd = action.cmd;
            let runner = action.runner;
            item.connect_activate(move |_| {
                launch_in_terminator(cmd, runner);
            });
            menu.append(&item);
        }
    }

    menu
}

/// `terminator -x bash -c '<wrapped>; bash'` keeps the shell open
/// after the command finishes so the user can scroll output or re-
/// run without re-spawning the window.
///
/// For `Runner::Pkexec`, the inner command becomes
/// `pkexec sh -c '<cmd>'` so chained shell pipelines (`a && b`)
/// still execute as root. For `Runner::Plain`, the command runs
/// verbatim as the invoking user.
fn launch_in_terminator(cmd: &str, runner: Runner) {
    let wrapped = wrap_command(cmd, runner);
    if let Err(e) = Command::new("terminator")
        .args(["-x", "bash", "-c", &wrapped])
        .spawn()
    {
        eprintln!("mackes-panel: terminator launch failed ({cmd}): {e}");
    }
}

/// Pure helper that builds the final bash one-liner for terminator.
/// Exposed for tests.
#[must_use]
fn wrap_command(cmd: &str, runner: Runner) -> String {
    match runner {
        Runner::Pkexec => {
            // Escape any single quotes in cmd so sh -c '...' doesn't
            // break on user-content commands. The current SECTIONS
            // table has no quotes but better to be defensive against
            // future additions.
            let escaped = cmd.replace('\'', "'\\''");
            format!("pkexec sh -c '{escaped}'; bash")
        }
        Runner::Plain => format!("{cmd}; bash"),
    }
}

fn format_tooltip(action: &AdminAction, agent_running: bool) -> String {
    let cmd_line = format!("Runs: {}", action.cmd);
    match action.runner {
        Runner::Pkexec => {
            let hint = if agent_running {
                "polkit agent running — password prompt will appear"
            } else {
                "polkit agent missing — pkexec may fail; start xfce-polkit or polkit-gnome-authentication-agent-1"
            };
            format!("{cmd_line}\n{hint}")
        }
        Runner::Plain => cmd_line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_have_at_least_one_action_each() {
        for (name, actions) in SECTIONS {
            assert!(!actions.is_empty(), "section {name} has no actions");
        }
    }

    #[test]
    fn total_action_count_matches_q15_lock() {
        // Q15 locked the "Comprehensive — 9 items grouped by section"
        // option. Guard the count so future edits don't silently
        // expand/shrink the lock.
        let total: usize = SECTIONS.iter().map(|(_, a)| a.len()).sum();
        assert_eq!(total, 9, "Q15 locked exactly 9 actions across all sections");
    }

    #[test]
    fn format_tooltip_marks_polkit_agent_state() {
        let action = AdminAction {
            label: "x",
            cmd: "y",
            runner: Runner::Pkexec,
        };
        assert!(format_tooltip(&action, true).contains("polkit agent running"));
        assert!(format_tooltip(&action, false).contains("polkit agent missing"));

        let plain = AdminAction {
            label: "x",
            cmd: "y",
            runner: Runner::Plain,
        };
        // Plain runners never mention polkit in their tooltip — they
        // don't escalate, so the agent state is irrelevant.
        assert!(!format_tooltip(&plain, true).contains("polkit"));
        assert!(!format_tooltip(&plain, false).contains("polkit"));
    }

    #[test]
    fn wrap_command_uses_pkexec_for_escalated_actions() {
        let wrapped = wrap_command("dnf upgrade --refresh", Runner::Pkexec);
        // Must invoke pkexec, must wrap in `sh -c '…'` so pipelines
        // work, must trail with `; bash` so terminator stays open
        // after the command exits.
        assert!(wrapped.starts_with("pkexec sh -c '"));
        assert!(wrapped.contains("dnf upgrade --refresh"));
        assert!(wrapped.ends_with("; bash"));
        // Must NOT contain raw sudo — that's the v2.0.3 fix.
        assert!(!wrapped.contains("sudo "));
    }

    #[test]
    fn wrap_command_passes_plain_actions_through() {
        let wrapped = wrap_command("sestatus", Runner::Plain);
        // Plain runners just trail `; bash` — no pkexec, no sh -c.
        assert_eq!(wrapped, "sestatus; bash");
        assert!(!wrapped.contains("pkexec"));
    }

    #[test]
    fn wrap_command_escapes_single_quotes_inside_pkexec_payload() {
        // Defensive — current SECTIONS has no single quotes, but if
        // a future admin action does, the pkexec wrapping must not
        // break the surrounding `sh -c '…'` quote balance.
        let wrapped = wrap_command("echo 'hello world'", Runner::Pkexec);
        // Re-parsing the wrapper through `sh -c` must yield the
        // original `echo 'hello world'`. The cheap correctness
        // check: the embedded single-quote sequence is the standard
        // bash quote-escape (`'\''`) for closing the outer quote,
        // appending an escaped quote, and reopening.
        assert!(wrapped.contains("echo '\\''hello world'\\''"));
    }

    #[test]
    fn no_section_command_uses_raw_sudo() {
        // Hard lock: the v2.0.3 fix replaced every sudo call site
        // with pkexec. If a future edit re-introduces sudo, this
        // test catches the regression at CI time before the bench
        // user finds it on right-click.
        for (_, actions) in SECTIONS {
            for action in *actions {
                assert!(
                    !action.cmd.contains("sudo "),
                    "command `{}` uses raw sudo — wrap with Runner::Pkexec instead",
                    action.cmd,
                );
            }
        }
    }
}
