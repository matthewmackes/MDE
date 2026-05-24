//! Apply — runs every selected birthright step + finalizes the
//! wizard state.
//!
//! Birthright steps are the v1.x `mackes/birthright.py`
//! functions: theme install, font install, app install, panel
//! layout, fleet enrol, etc. The Apply page presents the locked
//! list, lets the user toggle individual steps, then invokes
//! `mackes/birthright.py apply <steps>` (which the v2.0.0 cut
//! keeps as a Python library callable from the Iced wizard via
//! subprocess, per the CB-1.10 lock).

use crate::WizardState;

/// One birthright step the wizard can run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BirthrightStep {
    pub id: &'static str,
    pub label: &'static str,
    pub default_on: bool,
}

/// The 10 birthright steps from v1.3.0 fleet lock.
pub const STEPS: &[BirthrightStep] = &[
    BirthrightStep {
        id: "themes",
        label: "Install themes (Carbon · PatternFly · GNOME)",
        default_on: true,
    },
    BirthrightStep {
        id: "fonts",
        label: "Install fonts (Red Hat Display/Text/Mono)",
        default_on: true,
    },
    BirthrightStep {
        id: "apps",
        label: "Install default applications",
        default_on: true,
    },
    BirthrightStep {
        id: "panel-layout",
        label: "Configure MDE panel layout",
        default_on: true,
    },
    BirthrightStep {
        id: "shortcuts",
        label: "Install keyboard shortcuts",
        default_on: true,
    },
    BirthrightStep {
        id: "wallpaper",
        label: "Set default wallpaper",
        default_on: true,
    },
    BirthrightStep {
        id: "tweaks",
        label: "Apply system tweaks",
        default_on: true,
    },
    BirthrightStep {
        id: "fleet-enrol",
        label: "Enrol into mesh fleet",
        default_on: false,
    },
    BirthrightStep {
        id: "ansible-pull",
        label: "Run initial Ansible pull",
        default_on: false,
    },
    BirthrightStep {
        id: "plymouth",
        label: "Configure Plymouth boot splash",
        default_on: true,
    },
];

/// Default selection — every `default_on=true` step.
#[must_use]
pub fn default_selection() -> Vec<&'static str> {
    STEPS
        .iter()
        .filter(|s| s.default_on)
        .map(|s| s.id)
        .collect()
}

/// Build the argv for `mackes/birthright.py apply <selected steps>`.
#[must_use]
pub fn build_apply_argv(steps: &[&str]) -> Vec<String> {
    let mut argv = vec![
        "python3".into(),
        "-m".into(),
        "mackes.birthright".into(),
        "apply".into(),
    ];
    for step in steps {
        argv.push((*step).to_string());
    }
    argv
}

/// NF-14.4 (v2.5) — build the argv for `mackesd enroll --token`
/// when the operator supplied a join token in the MeshPasscode
/// page. Returns `None` when the input doesn't look like a v2.5
/// join token (operator either skipped the page or typed a
/// pre-v2.5 16-char passcode — both are handled by the
/// fallthrough path: NavNext-from-Apply silently skips enroll).
///
/// The Preview page (NF-7.3) probes `dev.mackes.MDE.Nebula.Status`
/// on land, so a successful enroll surfaces automatically; a
/// failed one leaves the diagnostics banner to fire after 30 s
/// with the daemon-down hint.
#[must_use]
pub fn build_enroll_argv(passcode_or_token: &str) -> Option<Vec<String>> {
    let trimmed = passcode_or_token.trim();
    if !trimmed.starts_with("mesh:") {
        return None;
    }
    Some(vec![
        "mackesd".to_string(),
        "enroll".to_string(),
        "--token".to_string(),
        trimmed.to_string(),
    ])
}

/// Mark the wizard provisioned and persist.
pub fn finalize(state: &mut WizardState) {
    state.provisioned = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_birthright_steps() {
        assert_eq!(STEPS.len(), 10);
    }

    #[test]
    fn step_ids_are_distinct() {
        let ids: std::collections::HashSet<_> = STEPS.iter().map(|s| s.id).collect();
        assert_eq!(ids.len(), 10);
    }

    #[test]
    fn default_selection_includes_themes_and_fonts() {
        let sel = default_selection();
        assert!(sel.contains(&"themes"));
        assert!(sel.contains(&"fonts"));
        assert!(!sel.contains(&"fleet-enrol")); // off by default
    }

    #[test]
    fn default_selection_omits_off_by_default_steps() {
        let sel = default_selection();
        for step in STEPS {
            if step.default_on {
                assert!(sel.contains(&step.id), "missing {}", step.id);
            } else {
                assert!(!sel.contains(&step.id), "should not contain {}", step.id);
            }
        }
    }

    #[test]
    fn build_apply_argv_invokes_python_birthright_module() {
        let argv = build_apply_argv(&["themes", "fonts"]);
        assert_eq!(
            argv,
            vec![
                "python3",
                "-m",
                "mackes.birthright",
                "apply",
                "themes",
                "fonts"
            ]
        );
    }

    #[test]
    fn build_apply_argv_with_empty_selection_still_includes_apply() {
        let argv = build_apply_argv(&[]);
        assert_eq!(argv, vec!["python3", "-m", "mackes.birthright", "apply"]);
    }

    #[test]
    fn finalize_marks_state_provisioned() {
        let mut state = WizardState::default();
        assert!(!state.provisioned);
        finalize(&mut state);
        assert!(state.provisioned);
    }

    #[test]
    fn every_step_has_non_empty_label() {
        for step in STEPS {
            assert!(!step.label.is_empty());
        }
    }

    // ---- NF-14.4 build_enroll_argv -----------------------

    #[test]
    fn build_enroll_argv_emits_mackesd_enroll_for_join_token() {
        let argv = build_enroll_argv("mesh:m@10.0.0.5:4242#bearer").expect("yes");
        assert_eq!(argv[0], "mackesd");
        assert_eq!(argv[1], "enroll");
        assert_eq!(argv[2], "--token");
        assert_eq!(argv[3], "mesh:m@10.0.0.5:4242#bearer");
    }

    #[test]
    fn build_enroll_argv_trims_whitespace_around_token() {
        let argv = build_enroll_argv("  mesh:m@10.0.0.5:4242#b  ").expect("yes");
        assert_eq!(argv[3], "mesh:m@10.0.0.5:4242#b");
    }

    #[test]
    fn build_enroll_argv_returns_none_for_legacy_passcode() {
        // v1.x 16-char passcode doesn't have the mesh: prefix —
        // wizard skips the enroll step in the Apply handler.
        assert!(build_enroll_argv("ABCDEFGHIJKLMNOP").is_none());
    }

    #[test]
    fn build_enroll_argv_returns_none_for_empty() {
        assert!(build_enroll_argv("").is_none());
        assert!(build_enroll_argv("   ").is_none());
    }

    #[test]
    fn build_enroll_argv_returns_none_for_other_schemes() {
        // Anything that doesn't start with `mesh:` is not a v2.5
        // join token; the operator may have typed a URL or a
        // passcode-by-accident — wizard skips silently.
        assert!(build_enroll_argv("https://example.com").is_none());
        assert!(build_enroll_argv("MESH:m@10.0.0.5:4242#b").is_none());
    }
}
