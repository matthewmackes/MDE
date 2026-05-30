//! `mde-install` — converge this node to a known-clean state for a
//! chosen install profile, then run birthrights.
//!
//! INST-3a / INST-4 (picker) / INST-5 (typed-`NUKE` confirm + audit
//! log). The clean Fedora-Server build-up path: pick a profile →
//! confirm → wipe MDE local state → run `mackes.birthright` for the
//! profile. Re-install extras (Nebula cert-revoke + GlusterFS brick
//! teardown) are INST-3b, blocked on a mackesd `Ca.Revoke` method;
//! on a clean box there is nothing there to tear down.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use mde_installer::confirm;
use mde_installer::profile::Profile;
use mde_installer::wipe;

#[derive(Parser, Debug)]
#[command(
    name = "mde-install",
    about = "Converge an MDE node to a known-clean state for an install profile, then run birthrights."
)]
struct Args {
    /// Install profile (skips the interactive picker): lighthouse|headless|full.
    #[arg(long)]
    profile: Option<String>,

    /// Non-interactive: skip the typed-NUKE confirm (writes an audit log instead). Requires --profile.
    #[arg(long)]
    yes: bool,

    /// Print the plan without changing anything.
    #[arg(long)]
    dry_run: bool,

    /// Tar existing MDE state to /var/lib/mde/backups/ before the wipe (recovery escape hatch).
    #[arg(long)]
    backup: bool,

    /// Skip the post-install smoke check (INST-14) — for image builds where some services aren't started yet.
    #[arg(long)]
    skip_smoke: bool,

    /// Skip cert revocation + GlusterFS brick teardown (INST-7 mesh-departure steps 2-4).
    /// Use when wiping local config but staying enrolled in the mesh — the node
    /// remains a valid mesh peer. Noted on the NUKE confirm screen.
    #[arg(long)]
    keep_mesh: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        // Ok carries the smoke-check exit code (0 = clean, 3 = a check failed).
        Ok(code) => ExitCode::from(code),
        Err(msg) => {
            eprintln!("mde-install: {msg}");
            ExitCode::from(2)
        }
    }
}

fn run(args: &Args) -> Result<u8, String> {
    let profile = resolve_profile(args)?;
    println!("Install profile: {profile} — {}", profile.describe());

    // INST-6 — is this a lossy downgrade off the previously-installed
    // profile (dropping the brick and/or desktop)? Drives the extra
    // typed-`<prev>` confirm below and the audit-log WARNING line.
    let prev_profile = wipe::read_installed_profile();
    let lossy = prev_profile
        .is_some_and(|prev| mde_installer::profile::is_lossy_downgrade(prev, profile));

    // Preflight — what the wipe will remove, with du-style size + file
    // count per path (INST-5a).
    let all = wipe::local_state_paths();
    let targets = wipe::existing(&all);
    println!("\nLocal MDE state to be wiped:");
    if targets.is_empty() {
        println!("  (none — clean machine)");
    } else {
        for p in &targets {
            let (bytes, files) = wipe::path_usage(p);
            let plural = if files == 1 { "" } else { "s" };
            println!(
                "  {}  ({}, {files} file{plural})",
                p.display(),
                wipe::human_size(bytes)
            );
        }
    }
    // INST-5 peer-impact: who will see this node leave the mesh.
    // Sourced from the converged GFS peer-files (PEERVER-3).
    let local = mde_installer::peers::local_hostname();
    let others: Vec<String> = mde_installer::peers::list_peers()
        .into_iter()
        .filter(|p| p.hostname != local)
        .map(|p| p.hostname)
        .collect();
    if others.is_empty() {
        println!("Peers affected: none (no other peers in the mesh peer registry).");
    } else {
        println!(
            "Peers that will see this node leave the mesh: {}",
            others.join(", ")
        );
    }

    if lossy {
        if let Some(prev) = prev_profile {
            println!(
                "\n(!) lossy downgrade: {prev} -> {profile} tears down the \
                 {prev}-profile pieces (brick and/or desktop)."
            );
        }
    }

    if args.dry_run {
        if let Some(prev) = prev_profile.filter(|_| lossy) {
            println!(
                "[dry-run] would require typing `{prev}` (the previous profile) \
                 after the NUKE confirm to proceed with the downgrade."
            );
        }
        if profile.needs_desktop_rpm() && !desktop_rpm_present() {
            println!("[dry-run] would `dnf install -y mde-desktop` (building up to the full desktop).");
        }
        if args.backup {
            println!("[dry-run] would tar existing MDE state to /var/lib/mde/backups/ before wiping.");
        }
        if args.keep_mesh {
            println!("[dry-run] --keep-mesh: cert revoke + LizardFS data teardown will be SKIPPED.");
        } else {
            println!(
                "[dry-run] would revoke this node's Nebula cert, wait up to 10s for peer \
                 detach, then remove the LizardFS data dir at /var/lib/mde/meshfs/."
            );
        }
        println!("\n[dry-run] would stop {:?}, wipe the paths above, write the \
                  profile marker, restart services, then run birthrights for {profile}.",
                 wipe::MANAGED_SERVICES);
        if !args.skip_smoke {
            println!("[dry-run] would then run the post-install smoke check.");
        }
        return Ok(0);
    }

    // Confirm.
    if confirm::stdin_is_tty() && !args.yes {
        if args.keep_mesh {
            println!(
                "(Mesh enrollment will be kept (--keep-mesh): cert revoke + \
                 LizardFS data teardown skipped.)"
            );
        }
        let ok = confirm::require_typed(
            "NUKE",
            "\nType NUKE to wipe the above and (re)install: ",
        )
        .map_err(|e| format!("reading confirmation: {e}"))?;
        if !ok {
            return Err("not confirmed — aborted".to_string());
        }
        // INST-6 — extra confirm on a lossy downgrade: make the operator
        // type the previous profile name so reflexive `NUKE` muscle-memory
        // can't silently demote a workstation to a routing-only lighthouse.
        if let Some(prev) = prev_profile.filter(|_| lossy) {
            let ok = confirm::require_typed(
                prev.as_str(),
                &format!(
                    "Currently `{prev}`. Type `{prev}` to confirm leaving the \
                     {prev}-profile state: "
                ),
            )
            .map_err(|e| format!("reading downgrade confirmation: {e}"))?;
            if !ok {
                return Err(format!(
                    "downgrade to {profile} not confirmed — aborted; no changes made."
                ));
            }
        }
    } else if !args.yes {
        return Err(
            "no TTY for the NUKE confirm; re-run with --yes (and --profile) for unattended installs"
                .to_string(),
        );
    }

    let mut audit = Vec::new();
    // INST-6 — record the downgrade at the top of the audit trail (the
    // only signal on the `--yes` path, which skips both confirms).
    if let Some(prev) = prev_profile.filter(|_| lossy) {
        audit.push(format!("WARNING: lossy downgrade from {prev} to {profile}"));
    }
    audit.push(format!("profile: {profile}"));

    // A4 — build up to the full desktop: pull mde-desktop if the
    // profile needs it and it isn't installed (the "build up from a
    // Fedora Server CLI" path). Done before the wipe so a missing
    // repo fails loudly before anything is destroyed.
    if profile.needs_desktop_rpm() && !desktop_rpm_present() {
        let msg = ensure_desktop_rpm()?;
        println!("{msg}");
        audit.push(msg);
    }

    // A6 — optional pre-wipe backup (recovery escape hatch; there is
    // no version history per Q25, so this is the only undo).
    if args.backup && !targets.is_empty() {
        match backup_state(&targets) {
            Ok(p) => {
                println!("backup: {}", p.display());
                audit.push(format!("backup: {}", p.display()));
            }
            Err(e) => return Err(format!("backup failed (aborting before wipe): {e}")),
        }
    }

    // Wipe sequence (clean-install scope).
    for line in wipe::stop_services(wipe::MANAGED_SERVICES) {
        audit.push(line);
    }
    // INST-7 — mesh-departure steps 2-4: cert revoke → peer-detach wait →
    // LizardFS data teardown. Skipped when --keep-mesh is set (node stays
    // a valid mesh peer with local config wiped but cert intact).
    if args.keep_mesh {
        audit.push(
            "keep-mesh: cert revoke + LizardFS data teardown skipped".to_string(),
        );
    } else {
        let local_node_id = mde_installer::peers::local_hostname();
        audit.push(wipe::revoke_own_cert(&local_node_id));
        audit.push(wipe::wait_for_peer_detach(10));
        for line in wipe::wipe_meshfs_data() {
            audit.push(line);
        }
    }
    for line in wipe::wipe_paths(&all) {
        audit.push(line);
    }
    // PEERVER-5 — leave the mesh registry: remove our own peer-file
    // (own-row authority = the file is this node's presence).
    match mde_installer::peers::remove_local_peer_file() {
        Ok(Some(p)) => audit.push(format!("removed peer-record: {}", p.display())),
        Ok(None) => audit.push("peer-record: none to remove".to_string()),
        Err(e) => audit.push(format!("peer-record remove failed: {e}")),
    }
    if let Err(e) = wipe::write_profile_marker(profile) {
        audit.push(format!("FAILED to write profile marker: {e}"));
    } else {
        audit.push(format!("wrote {}", wipe::PROFILE_MARKER));
    }
    for line in wipe::start_services(wipe::MANAGED_SERVICES) {
        audit.push(line);
    }

    // Always leave an audit trail (INST-5).
    let log_path = write_audit_log(&audit);
    match &log_path {
        Ok(p) => println!("audit log: {}", p.display()),
        Err(e) => eprintln!("mde-install: could not write audit log: {e}"),
    }

    // Birthrights for the profile.
    if let Err(e) = run_birthrights(profile) {
        return Err(format!(
            "{e}\n  recover: run `python3 -m mackes.birthright_rollback` or re-run \
             `mde-install --profile={profile}` (idempotent); the audit log above lists \
             what changed."
        ));
    }
    println!("\nmde-install: {profile} node converged.");

    // INST-14 — post-install smoke check: verify the claimed profile is
    // actually running before reporting success. A failed check exits 3.
    if args.skip_smoke {
        println!("(smoke check skipped via --skip-smoke)");
        return Ok(0);
    }
    println!("\nPost-install smoke check:");
    let results = mde_installer::smoke::run(profile);
    Ok(mde_installer::smoke::report(profile, &results))
}

/// `dnf install -y mde-desktop` — pull the Wayland desktop addon when
/// building up to the full profile from a headless base.
fn ensure_desktop_rpm() -> Result<String, String> {
    println!("Building up to the full desktop: installing mde-desktop…");
    let status = Command::new("dnf")
        .args(["install", "-y", "mde-desktop"])
        .status()
        .map_err(|e| format!("spawning dnf: {e}"))?;
    if status.success() {
        Ok("installed mde-desktop".to_string())
    } else {
        Err(format!(
            "could not install mde-desktop (dnf exit {}). The full profile needs it — \
             configure the MDE dnf repo (or `dnf install mde-desktop` manually) and re-run.",
            status.code().unwrap_or(-1)
        ))
    }
}

/// Tar the existing MDE-state paths to `/var/lib/mde/backups/` before
/// the wipe. Returns the tarball path.
fn backup_state(paths: &[PathBuf]) -> std::io::Result<PathBuf> {
    let dir = PathBuf::from("/var/lib/mde/backups");
    fs::create_dir_all(&dir)?;
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let tarball = dir.join(format!("mde-state-{ms}.tar.gz"));
    let mut cmd = Command::new("tar");
    cmd.arg("-czf").arg(&tarball);
    for p in paths {
        cmd.arg(p);
    }
    let status = cmd.status()?;
    if status.success() {
        Ok(tarball)
    } else {
        Err(std::io::Error::other(format!(
            "tar exited {}",
            status.code().unwrap_or(-1)
        )))
    }
}

fn resolve_profile(args: &Args) -> Result<Profile, String> {
    if let Some(p) = &args.profile {
        return p.parse::<Profile>().map_err(|e| e.to_string());
    }
    if confirm::stdin_is_tty() {
        let default = desktop_rpm_present().then_some(Profile::Full);
        let stdin = std::io::stdin();
        let mut locked = stdin.lock();
        let mut out = std::io::stdout();
        return confirm::pick_profile_from(&mut locked, &mut out, default)
            .map_err(|e| format!("reading profile choice: {e}"));
    }
    Err("no --profile and no TTY for the picker; pass --profile=lighthouse|headless|full".to_string())
}

fn desktop_rpm_present() -> bool {
    Command::new("rpm")
        .args(["-q", "mde-desktop"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_birthrights(profile: Profile) -> Result<(), String> {
    println!("\nRunning birthrights for {profile}…");
    let status = Command::new("python3")
        .args([
            "-m",
            "mackes.birthright",
            "--profile",
            profile.as_str(),
            "--noninteractive",
        ])
        .status()
        .map_err(|e| format!("spawning birthrights: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "birthrights exited with {}",
            status.code().unwrap_or(-1)
        ))
    }
}

fn write_audit_log(lines: &[String]) -> std::io::Result<PathBuf> {
    let dir = PathBuf::from("/var/log/mde");
    fs::create_dir_all(&dir)?;
    // INST-5c — ULID filename: time-sortable + collision-free, matching
    // the `wipe-<ulid>.log` path INST-12's auto-trigger expects.
    let id = ulid::Ulid::new();
    let path = dir.join(format!("wipe-{id}.log"));
    let mut f = fs::File::create(&path)?;
    for line in lines {
        writeln!(f, "{line}")?;
    }
    Ok(path)
}
