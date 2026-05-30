//! Post-install smoke check (INST-14) — the last step before
//! `mde-install` reports success.
//!
//! Verifies the claimed profile is actually running: services up,
//! mesh-home replicated where applicable, the desktop session live for
//! `full`. Each decision is a **pure classifier** (`classify_*`) taking
//! already-gathered facts, so the logic is unit-tested without touching
//! the system; [`run`] gathers the facts (systemctl / mfsmaster / env)
//! and applies the classifiers.

use std::process::Command;

use crate::peers;
use crate::profile::Profile;

/// One check's verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Passed.
    Ok,
    /// Not applicable in this context (with a reason).
    Skip(String),
    /// Failed (with details).
    Fail(String),
}

/// A named check result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    /// Short check name (e.g. `mackesd`, `mesh-home`).
    pub name: String,
    /// Verdict.
    pub outcome: Outcome,
}

impl CheckResult {
    fn new(name: &str, outcome: Outcome) -> Self {
        Self {
            name: name.to_string(),
            outcome,
        }
    }
}

// ── pure classifiers (unit-tested) ──────────────────────────────────

/// A required systemd unit: `Ok` when active, else `Fail`.
#[must_use]
pub fn classify_service(active: bool) -> Outcome {
    if active {
        Outcome::Ok
    } else {
        Outcome::Fail("service not active".to_string())
    }
}

/// LizardFS mesh-storage: `Ok` when `/mnt/mesh-storage` is mounted and
/// accessible, else `Fail`.
#[must_use]
pub fn classify_meshfs(mounted: bool) -> Outcome {
    if mounted {
        Outcome::Ok
    } else {
        Outcome::Fail("mesh-storage not mounted at /mnt/mesh-storage".to_string())
    }
}

/// Peer-registry presence: `Skip` when no peers are enrolled yet
/// (first-ever node), else `Ok`. (True overlay-ping reachability needs
/// per-peer overlay IPs, which PeerRecord doesn't carry — registry
/// presence is the honest proxy; see INST-PEERVER-REACH follow-up.)
#[must_use]
pub fn classify_peers(other_peer_count: usize) -> Outcome {
    if other_peer_count == 0 {
        Outcome::Skip("no other peers enrolled yet".to_string())
    } else {
        Outcome::Ok
    }
}

/// `full`-profile desktop session: `Ok` when sway is the current
/// session, else `Skip` (the operator must log out + back in to start
/// the freshly-installed session). Non-full profiles never reach here.
#[must_use]
pub fn classify_session(current_desktop: Option<&str>) -> Outcome {
    match current_desktop {
        Some(d) if d.eq_ignore_ascii_case("sway") => Outcome::Ok,
        _ => Outcome::Skip("log out and back in to start the sway session".to_string()),
    }
}

// ── runner (gathers facts + applies classifiers) ────────────────────

/// Run the smoke checks for `profile`, gathering live system facts.
#[must_use]
pub fn run(profile: Profile) -> Vec<CheckResult> {
    let mut out = vec![
        CheckResult::new("mackesd", classify_service(service_active("mackesd.service"))),
        CheckResult::new("nebula", classify_service(service_active("nebula.service"))),
        CheckResult::new(
            "peers",
            classify_peers(other_peer_count()),
        ),
    ];
    if matches!(profile, Profile::Headless | Profile::Full) {
        out.push(CheckResult::new(
            "mfschunkserver",
            classify_service(service_active("mfschunkserver.service")),
        ));
        out.push(CheckResult::new(
            "mesh-storage",
            classify_meshfs(meshfs_mounted()),
        ));
    }
    if matches!(profile, Profile::Full) {
        out.push(CheckResult::new(
            "session",
            classify_session(std::env::var("XDG_SESSION_DESKTOP").ok().as_deref()),
        ));
    }
    out
}

/// Print the results + return the process exit code: 0 when no check
/// failed (skips are fine), 3 when any check failed (INST-14 contract).
#[must_use]
pub fn report(profile: Profile, results: &[CheckResult]) -> u8 {
    let mut failed = false;
    let mut up = 0usize;
    let mut total = 0usize;
    for r in results {
        match &r.outcome {
            Outcome::Ok => {
                up += 1;
                total += 1;
            }
            Outcome::Skip(reason) => {
                println!("    - {} skipped: {reason}", r.name);
            }
            Outcome::Fail(reason) => {
                total += 1;
                failed = true;
                println!("(!) check failed: {} — {reason}", r.name);
            }
        }
    }
    if failed {
        3
    } else {
        println!(">>> mde-install complete: profile={profile}, services={up}/{total} up.");
        0
    }
}

fn service_active(unit: &str) -> bool {
    Command::new("systemctl")
        .args(["is-active", "--quiet", unit])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn meshfs_mounted() -> bool {
    std::path::Path::new("/mnt/mesh-storage").exists()
}

fn other_peer_count() -> usize {
    let local = peers::local_hostname();
    peers::list_peers()
        .into_iter()
        .filter(|p| p.hostname != local)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_active_ok_inactive_fail() {
        assert_eq!(classify_service(true), Outcome::Ok);
        assert!(matches!(classify_service(false), Outcome::Fail(_)));
    }

    #[test]
    fn meshfs_mounted_ok_unmounted_fail() {
        assert_eq!(classify_meshfs(true), Outcome::Ok);
        assert!(matches!(classify_meshfs(false), Outcome::Fail(_)));
    }

    #[test]
    fn peers_skip_when_alone_ok_when_present() {
        assert!(matches!(classify_peers(0), Outcome::Skip(_)));
        assert_eq!(classify_peers(2), Outcome::Ok);
    }

    #[test]
    fn session_ok_for_sway_skip_otherwise() {
        assert_eq!(classify_session(Some("sway")), Outcome::Ok);
        assert!(matches!(classify_session(Some("gnome")), Outcome::Skip(_)));
        assert!(matches!(classify_session(None), Outcome::Skip(_)));
    }

    #[test]
    fn report_exit_3_on_any_fail() {
        let results = vec![
            CheckResult::new("a", Outcome::Ok),
            CheckResult::new("b", Outcome::Fail("down".into())),
        ];
        assert_eq!(report(Profile::Full, &results), 3);
    }

    #[test]
    fn report_exit_0_when_no_fail() {
        let results = vec![
            CheckResult::new("a", Outcome::Ok),
            CheckResult::new("b", Outcome::Skip("n/a".into())),
        ];
        assert_eq!(report(Profile::Lighthouse, &results), 0);
    }
}
