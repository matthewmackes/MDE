//! SWAY-5 — workspace template → swaymsg batch emitter.
//!
//! A workspace template ([`TemplateSpec`]: one sway workspace number +
//! an ordered list of app launch commands) is applied in a single
//! `swaymsg` call so the whole layout lands in one compositor pass with
//! no intermediate flicker.
//!
//! [`batch_payload`] turns a `TemplateSpec` into the argument string
//! that gets handed to `swaymsg "<payload>"`. Commands are separated by
//! `"; "` which sway's IPC parser treats identically to a newline in
//! config. The actual one-shot shell-out is the caller's responsibility
//! (bench-gated per SWAY-5); the payload string is the pure testable
//! contract.

use mde_card::TemplateSpec;

/// Build the swaymsg argument that applies `spec` in one IPC call:
///
/// ```text
/// workspace number <id>; exec <app1>; exec <app2>; …
/// ```
///
/// Returns just `workspace number <id>` when the template has no apps,
/// so applying an empty template still focuses the workspace.
#[must_use]
pub fn batch_payload(spec: &TemplateSpec) -> String {
    let mut cmds: Vec<String> = Vec::with_capacity(spec.apps.len() + 1);
    cmds.push(format!("workspace number {}", spec.workspace));
    for app in &spec.apps {
        cmds.push(format!("exec {app}"));
    }
    cmds.join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(workspace: i32, apps: &[&str]) -> TemplateSpec {
        TemplateSpec {
            workspace,
            apps: apps.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn empty_template_emits_workspace_switch_only() {
        assert_eq!(batch_payload(&spec(3, &[])), "workspace number 3");
    }

    #[test]
    fn single_app_appends_one_exec() {
        assert_eq!(
            batch_payload(&spec(3, &["firefox"])),
            "workspace number 3; exec firefox"
        );
    }

    #[test]
    fn multi_app_preserves_order() {
        assert_eq!(
            batch_payload(&spec(5, &["foot", "firefox", "org.mde.voice.hud"])),
            "workspace number 5; exec foot; exec firefox; exec org.mde.voice.hud"
        );
    }

    #[test]
    fn workspace_switch_is_always_first() {
        let payload = batch_payload(&spec(2, &["kitty"]));
        assert!(payload.starts_with("workspace number 2; "));
    }

    #[test]
    fn exec_commands_pass_through_verbatim() {
        assert_eq!(
            batch_payload(&spec(1, &["/usr/bin/foo --flag bar"])),
            "workspace number 1; exec /usr/bin/foo --flag bar"
        );
    }

    #[test]
    fn workspace_number_one() {
        assert_eq!(batch_payload(&spec(1, &[])), "workspace number 1");
    }

    #[test]
    fn swaymsg_separator_is_semicolon_space() {
        let payload = batch_payload(&spec(4, &["app-a", "app-b"]));
        // Verify the join produces "; " not " ; " (hyprctl format).
        assert_eq!(payload, "workspace number 4; exec app-a; exec app-b");
        assert!(!payload.contains(" ; "), "should not use hyprctl format");
    }
}
