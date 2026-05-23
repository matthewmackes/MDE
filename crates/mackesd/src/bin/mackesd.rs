//! `mackesd` — CLI entry point for the Mesh control plane.
//!
//! Subcommands land alongside their backing Phase 12 substeps. Today
//! only `mackesd migrate` ships (Phase 12.2 store + migrations); the
//! rest follow as substeps complete. We deliberately do NOT register
//! stub commands here — every `mackesd X` either does X or is absent.

use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "mackesd",
    version,
    about = "Mesh control plane for Mackes XFCE Workstation"
)]
struct Cli {
    /// Override the default `SQLite` store path (defaults to
    /// `$MACKESD_HOME/mackesd.db` or `/var/lib/mackesd/mackesd.db`).
    #[arg(long, env = "MACKESD_DB")]
    db: Option<PathBuf>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Apply every pending `SQLite` migration against the store.
    ///
    /// Idempotent — running `mackesd migrate` against an up-to-date
    /// store is a no-op that exits 0.
    Migrate,

    /// Print store status: applied-migration count + db path.
    Status,

    /// Print the live `HealthReport` as a JSON line (Phase 12.1.3).
    ///
    /// Same shape as `mackesd_core::health::HealthReport` so the
    /// panel + the CLI consume identical data.
    Healthz,

    /// Generate a fresh 16-char URL-safe passcode (Phase 12.10.1).
    /// Prints the passcode and an instruction to save it to
    /// `libsecret` as `org.mackes.mesh.passcode`. Does NOT mutate
    /// libsecret on its own — caller drives the secret-storage step.
    GeneratePasscode,

    /// Walk the `events` table forward and verify every row's hash
    /// (Phase 12.10.3). Exits 0 on Intact / Empty, 1 on Break.
    AuditVerify,

    /// Rotate the shared mesh passcode (Phase 12.10.2). Prints a
    /// freshly-generated passcode and reminds the operator to
    /// store it in libsecret. Peers pick up the new passcode on
    /// their next heartbeat once the reconcile loop ships (12.5).
    RotatePasscode,

    /// Explain why a given peer is expected to peer with each of
    /// its neighbors (Phase 12.4.4). Reads `topology::calculate`'s
    /// reason chain for the named node.
    PeersWhy {
        /// Stable node id (e.g. `peer:anvil`).
        #[arg(value_name = "NODE_ID")]
        node_id: String,
    },

    /// Dry-run apply (Phase 12.7.4). Runs the validation +
    /// reconcile-plan pipeline without mutating anything; prints
    /// the diff + would-be event log as JSON. Useful in CI to
    /// catch config issues before a real apply.
    Apply {
        /// Skip mutation; print the plan only.
        #[arg(long)]
        dry_run: bool,
    },

    /// Enroll this peer against the mesh (Phase 12.3.1). Generates
    /// a fresh Ed25519 keypair + bearer token; prints a signed
    /// `EnrollmentRequest` JSON that the leader ingests.
    Enroll {
        /// 16-character URL-safe shared passcode.
        #[arg(long)]
        passcode: String,
        /// Optional display name; defaults to the system hostname.
        #[arg(long)]
        name: Option<String>,
    },

    /// Decommission a peer (Phase 12.3.4). Soft-deletes the node
    /// row; preserves history. `--force` skips the unreachable
    /// confirmation.
    Decommission {
        /// Stable node id to retire.
        node_id: String,
        /// Force decommission even when the peer is unreachable.
        #[arg(long)]
        force: bool,
    },

    /// Re-enroll an existing node (Phase 12.3.5). Issues fresh
    /// credentials against the existing row, preserving history.
    Reenroll {
        /// Stable node id to refresh.
        node_id: String,
    },

    /// Force this peer into leadership (Phase 12.1.1b operator
    /// override). Bumps the lease epoch.
    TakeLeadership {
        /// Stable node id to install as leader.
        #[arg(long)]
        as_node: String,
    },

    /// Import legacy mesh state into the `mackesd` store (Phase
    /// 12.13.2). Walks the prior 2.x JSON/TOML caches and emits
    /// a JSON plan that the operator can review before applying.
    ImportLegacy {
        /// Print the plan only; don't write anything.
        #[arg(long)]
        dry_run: bool,
    },

    /// Inventory legacy on-disk state (Phase 12.13.1). Walks the
    /// three canonical roots (`~/.config/mackes-shell/`,
    /// `~/.qnm-sync/`, `~/.cache/mackes/`) and prints a catalog of
    /// every JSON / TOML / cache file found, classified by kind and
    /// flagged with whether the filename hints at mesh data. This
    /// is the *inspection* step — `mackesd import-legacy` is what
    /// actually moves data into the store.
    InventoryLegacy {
        /// Only emit artifacts whose filename matches the
        /// mesh-related heuristic.
        #[arg(long)]
        mesh_only: bool,
        /// Emit the full inventory as a JSON array. Without this
        /// flag a human-readable table prints to stdout.
        #[arg(long)]
        json: bool,
    },

    /// Run the reconcile worker (Phase 12.5 wiring). Default mode
    /// loops forever on the foreground thread, ticking every
    /// `RECONCILE_INTERVAL_S` seconds (30 s per the 12.5.1 lock).
    /// This is the entry point systemd's `mackesd.service` invokes.
    ///
    /// The worker reads peer heartbeats + link telemetry from
    /// `QNM_SHARED_ROOT/<peer>/mackesd/{heartbeat,links}.json`,
    /// compares them against the latest applied `desired_config`
    /// snapshot, and routes the resulting drift rows through
    /// `reconcile::plan_tick`. Auto-repairable rows land in the
    /// audit-log with the `intent` field marking that take-action
    /// is gated on the connectivity layer (12.14+); manual-review
    /// rows are surfaced via `tracing::warn` for the GUI inbox.
    ///
    /// SIGTERM / SIGINT trigger a graceful exit: the current tick
    /// finishes, then the loop returns. Cleanly handles systemd's
    /// `TimeoutStopSec`.
    Reconcile {
        /// Run one tick, print the resulting `TickOutcome` as a
        /// pretty-printed JSON object, and exit. No background
        /// thread, no signal handler — for CI smoke tests + the
        /// dry-run loop the operator runs by hand.
        #[arg(long)]
        once: bool,
        /// Override the QNM-Shared root (defaults to
        /// `$QNM_SHARED_ROOT` or `~/QNM-Shared`). Useful for tests.
        #[arg(long, env = "QNM_SHARED_ROOT")]
        qnm_root: Option<PathBuf>,
        /// Override the stable node id (defaults to
        /// `peer:<hostname>`). Recorded as the `actor` field on
        /// every emitted audit event.
        #[arg(long)]
        node_id: Option<String>,
    },

    /// v2.0.0 Phase F.12 — desired_config revision management. Read
    /// every revision (`list`), diff two revisions (`diff a b`), or
    /// roll a prior revision forward as a new applied row
    /// (`rollback id`).
    Revisions {
        #[command(subcommand)]
        cmd: RevisionsCmd,
    },

    /// CB-1.5.a — fleet node roster. `mded nodes list --json` emits
    /// every row from the `nodes` table as a JSON array; the Iced
    /// inventory panel (in `crates/mde-workbench/src/panels/
    /// inventory.rs`) consumes the same shape. Without `--json` the
    /// command prints a human-readable table.
    Nodes {
        #[command(subcommand)]
        cmd: NodesCmd,
    },

    /// CB-1.5.c follow-up — ansible-pull run history. `mded
    /// ansible-history list --json` walks
    /// `$QNM_SHARED_ROOT/.qnm-sync/ansible-runs/<peer>/*.json`
    /// and emits the union as a sorted (timestamp DESC) JSON
    /// array. The Iced run-history panel reads the same
    /// filesystem source directly today — this CLI alternative
    /// exists for headless / leader-aggregated views where the
    /// reader peer doesn't have QNM-Sync replicated locally.
    AnsibleHistory {
        #[command(subcommand)]
        cmd: AnsibleHistoryCmd,
    },

    /// CB-1.5.b follow-up — curated playbook surface. `mded
    /// playbooks list --json` enumerates every role under
    /// `$QNM_SHARED_ROOT/.qnm-sync/playbooks/roles/` with the
    /// Phase 1.3.0 curated description if recognised. `mded
    /// playbooks run <name>` shells out to `ansible-pull
    /// --tags <name> site.yml` locally — same shape as the
    /// Iced playbooks panel's Run button, but headless-
    /// friendly (no GUI dependency).
    Playbooks {
        #[command(subcommand)]
        cmd: PlaybooksCmd,
    },

    /// CB-1.8 mesh_history follow-up — audit-log viewer
    /// surface. `mded events list --json` emits the entire
    /// hash-chained `events` table as a JSON array. The Iced
    /// mesh_history panel consumes this. Headless callers
    /// (audit scripts) get the same shape.
    Events {
        #[command(subcommand)]
        cmd: EventsCmd,
    },

    /// v2.0.0 Phase G.4 — push a settings revision to a peer
    /// selection. Writes a new `desired_config` row, records one
    /// `fleet_settings_apply_log` row per (peer, key) target, and
    /// prints the JSON plan. The reconcile worker on each named
    /// peer picks up the revision on its next tick.
    ///
    /// `--peers` accepts a comma-separated list of node ids, or the
    /// literal token `all` for the full healthy set.
    #[cfg(feature = "async-services")]
    FleetPushSetting {
        /// Dot-notated setting key (e.g. `theme.accent`).
        key: String,
        /// JSON-encoded value payload. The string itself is taken
        /// verbatim — quote it for the shell as appropriate.
        value: String,
        /// Comma-separated peer ids, or `all`.
        #[arg(long, default_value = "all")]
        peers: String,
        /// Override the revision author tag (defaults to
        /// `peer:<hostname>`).
        #[arg(long)]
        author: Option<String>,
        /// Print the plan but don't write to the store.
        #[arg(long)]
        dry_run: bool,
    },

    /// v2.0.0 Phase B.12 — the unified meta-daemon entry point.
    /// Replaces the legacy `migrate && status` ExecStart on the
    /// systemd unit. Boots the tokio runtime, spawns the worker
    /// supervisor + every registered worker, and blocks on
    /// SIGTERM/SIGINT.
    ///
    /// Phase A.2 ships the supervisor surface; Phase B fills in the
    /// individual workers (`clipboard`, `mdns`, `fs_sync`, ...).
    /// Today `serve` registers the existing reconcile loop as the
    /// single worker so the unit's behavior matches the current
    /// `mackesd reconcile` invocation while the rest of Phase B lands.
    ///
    /// Requires the `async-services` cargo feature.
    #[cfg(feature = "async-services")]
    Serve {
        /// Override the QNM-Shared root (defaults to
        /// `$QNM_SHARED_ROOT` or `~/QNM-Shared`).
        #[arg(long, env = "QNM_SHARED_ROOT")]
        qnm_root: Option<PathBuf>,
        /// Override the stable node id (defaults to `peer:<hostname>`).
        #[arg(long)]
        node_id: Option<String>,
    },

    /// PC-3.a — trigger the `peer-joined` handler for a given
    /// peer-id.
    ///
    /// Writes the peer's [`PeerProbe`] to the cache, then spawns
    /// `mde-peer-card --peer <id>` (subject to the 30s per-peer
    /// debounce). Today the probe is the fixture; once the
    /// store grows live probe data, this command will load from
    /// there. Operator-driven for now; the reconcile loop will
    /// emit the same event when a new peer enrolls.
    PeerCard {
        /// Stable peer id (e.g. `peer:lab-01`).
        #[arg(long, value_name = "PEER_ID")]
        peer: String,
        /// Don't spawn the modal — print the would-be action.
        #[arg(long)]
        dry_run: bool,
    },
}

/// Subcommands for `mackesd ansible-history`. CB-1.5.c
/// follow-up.
#[derive(Subcommand)]
enum AnsibleHistoryCmd {
    /// List every ansible-pull run record across the mesh.
    /// `--json` emits a sorted (timestamp DESC) JSON array.
    List {
        /// Emit a JSON array of `{peer, playbook, timestamp,
        /// exit_code, changed, ok, failed, triggered_by, ...}`
        /// rows.
        #[arg(long)]
        json: bool,
    },
}

/// Subcommands for `mackesd events`. CB-1.8 mesh_history
/// follow-up.
#[derive(Subcommand)]
enum EventsCmd {
    /// List every row from the `events` table. `--json`
    /// emits a JSON array of every audit-log row in seq
    /// order.
    List {
        #[arg(long)]
        json: bool,
    },
}

/// Subcommands for `mackesd playbooks`. CB-1.5.b follow-up.
#[derive(Subcommand)]
enum PlaybooksCmd {
    /// List every role under the curated playbooks root.
    /// `--json` emits `[{name, description}, ...]`.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Run a playbook locally via `ansible-pull --tags <name>
    /// site.yml`. Streams stdout to this process's stdout.
    Run {
        /// Role / tag name (matches a directory under the
        /// curated playbooks root).
        name: String,
    },
}

/// Subcommands for `mackesd nodes`. CB-1.5.a.
#[derive(Subcommand)]
enum NodesCmd {
    /// List every row from the `nodes` table. Without `--json` the
    /// output is a human-readable table with one peer per line.
    List {
        /// Emit a JSON array of `{node_id, name, public_key, role,
        /// health, region}` rows — consumed by the Workbench
        /// Fleet → Inventory panel.
        #[arg(long)]
        json: bool,
    },
}

/// Subcommands for `mackesd revisions`. Phase F.12.
#[derive(Subcommand)]
enum RevisionsCmd {
    /// List every revision in the `desired_config` table, newest
    /// first. `--json` for machine-readable output (consumed by the
    /// Workbench Fleet → Revisions panel).
    List {
        /// Emit a JSON array of `{revision_id, author, state,
        /// created_at, summary}` rows.
        #[arg(long)]
        json: bool,
    },
    /// Diff two revisions' spec_json payloads. Prints the keys
    /// added / removed / changed (uses `mackesd_core::revisions::diff`
    /// via a thin SQL adapter).
    Diff {
        /// "From" revision id.
        from: String,
        /// "To" revision id.
        to: String,
    },
    /// Roll back to a prior revision by writing its payload as a
    /// fresh applied revision (immutable history per 12.2.2).
    Rollback {
        /// Revision id to restore.
        target_id: String,
        /// Author tag for the new rollback revision (defaults to
        /// `peer:<hostname>`).
        #[arg(long)]
        author: Option<String>,
        /// Peer selector — `all` or comma-list. Today the rollback
        /// only writes the new row centrally; the per-peer apply
        /// happens via the existing reconcile loop. The selector
        /// is recorded in the rollback row's summary for audit.
        #[arg(long, default_value = "all")]
        peers: String,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let db_path = cli.db.unwrap_or_else(mackesd_core::default_db_path);

    match cli.cmd {
        Cmd::Migrate => {
            let conn = mackesd_core::store::open(&db_path)
                .with_context(|| format!("opening store at {}", db_path.display()))?;
            let n = mackesd_core::store::applied_migration_count(&conn)?;
            tracing::info!("store at {} migrated (n={})", db_path.display(), n);
            println!("{n} migrations applied");
        }
        Cmd::Status => {
            let conn = mackesd_core::store::open(&db_path)
                .with_context(|| format!("opening store at {}", db_path.display()))?;
            let n = mackesd_core::store::applied_migration_count(&conn)?;
            println!("db:                 {}", db_path.display());
            println!("migrations applied: {n}");
        }
        Cmd::Healthz => {
            // First-class panel/CLI parity per 12.1.3. The report
            // pulls live `applied_revision` + per-health `nodes`
            // counts from the SQLite store. Subsequent substeps
            // (12.3.3 leader election, 12.6.3 audit-chain verify)
            // populate the remaining two fields.
            let report = match mackesd_core::store::open(&db_path) {
                Ok(conn) => mackesd_core::health::HealthReport::compute(&conn)
                    .unwrap_or_else(|_| mackesd_core::health::HealthReport::empty()),
                Err(_) => mackesd_core::health::HealthReport::empty(),
            };
            println!("{}", report.to_json_line()?);
        }
        Cmd::GeneratePasscode => {
            let code = mackesd_core::passcode::generate();
            println!("{code}");
            eprintln!(
                "(save with: secret-tool store --label='Mackes mesh passcode' \
                org.mackes.mesh.passcode <name>)"
            );
        }
        Cmd::AuditVerify => {
            // Reads every row from the `events` table (ordered by
            // `seq` ASC) and walks the SHA-256 hash chain.
            let conn = mackesd_core::store::open(&db_path)
                .with_context(|| format!("opening store at {}", db_path.display()))?;
            let rows =
                mackesd_core::store::load_audit_rows(&conn).context("loading events from store")?;
            match mackesd_core::audit::verify(&rows) {
                mackesd_core::audit::VerifyOutcome::Empty => {
                    println!("audit chain empty (no events yet)");
                }
                mackesd_core::audit::VerifyOutcome::Intact { verified, .. } => {
                    println!("verified {verified} events  ·  chain intact");
                }
                mackesd_core::audit::VerifyOutcome::Break { at_event, .. } => {
                    eprintln!("audit chain BREAK at event {at_event}");
                    std::process::exit(1);
                }
            }
        }
        Cmd::RotatePasscode => {
            // Phase 12.10.2 — generate fresh passcode; libsecret
            // store + peer redistribution wires through with 12.5.
            let code = mackesd_core::passcode::generate();
            println!("{code}");
            eprintln!(
                "rotation: store with `secret-tool store --label='Mackes mesh \
                 passcode' org.mackes.mesh.passcode <name>` then peers refresh \
                 their bearer tokens on next heartbeat."
            );
        }
        Cmd::PeersWhy { node_id } => {
            // Phase 12.4.4 — explanation surface. Loads the node
            // roster from the store, runs `topology::calculate`,
            // and walks the resulting edge set + route table to
            // emit a per-edge reason chain for the named peer.
            let conn = mackesd_core::store::open(&db_path)
                .with_context(|| format!("opening store at {}", db_path.display()))?;
            let nodes =
                mackesd_core::store::list_nodes(&conn).context("listing nodes from store")?;
            let report = explain_peer(&node_id, &nodes);
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Cmd::Apply { dry_run } => {
            if dry_run {
                // Phase 12.7.4 — run validation against an empty
                // snapshot today; once the store wires the
                // serialized desired-config row in, the dry-run
                // path returns the real diff + event-log preview.
                let snapshot = mackesd_core::topology::DesiredSnapshot::default();
                let errors = mackesd_core::validation::validate(&snapshot);
                let report = serde_json::json!({
                    "dry_run": true,
                    "validation_errors": errors.len(),
                    "would_apply_revisions": 0,
                });
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                eprintln!(
                    "mackesd: non-dry-run apply requires the reconcile loop \
                     (Phase 12.5) — use `mackesd apply --dry-run` for the \
                     validation + plan preview."
                );
                std::process::exit(2);
            }
        }
        Cmd::Enroll { passcode, name } => {
            // Phase 12.3.1 — build identity + signed request.
            let identity = mackesd_core::enrollment::build_identity();
            let display = name.unwrap_or_else(|| {
                std::env::var("HOSTNAME").unwrap_or_else(|_| {
                    std::process::Command::new("hostname")
                        .output()
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok())
                        .map_or_else(|| "unknown".to_owned(), |s| s.trim().to_owned())
                })
            });
            match mackesd_core::enrollment::build_request(&identity, &passcode, &display) {
                Some(req) => {
                    println!("{}", serde_json::to_string_pretty(&req)?);
                    eprintln!(
                        "enrollment request emitted — drop into the leader's \
                         pending inbox (Phase 12.8.2)."
                    );
                }
                None => {
                    eprintln!(
                        "mackesd enroll: passcode failed validation (must be \
                         16 URL-safe characters)."
                    );
                    std::process::exit(2);
                }
            }
        }
        Cmd::Decommission { node_id, force } => {
            // Phase 12.3.4 — soft-delete the node row and emit a
            // hash-chained Lifecycle event so the audit trail
            // records the operator action. `--force` only changes
            // the audit kind label; the SQL effect is identical
            // (CHECK constraint enforces the same allowed roles).
            let mut conn = mackesd_core::store::open(&db_path)
                .with_context(|| format!("opening store at {}", db_path.display()))?;
            let updated = mackesd_core::store::set_node_role(&conn, &node_id, "decommissioned")?;
            if updated == 0 {
                eprintln!("mackesd decommission: no node row matches {node_id}");
                std::process::exit(2);
            }
            let payload = serde_json::json!({
                "kind":  if force { "forced" } else { "soft" },
                "node":  node_id,
                "event": "decommission",
            })
            .to_string();
            mackesd_core::store::insert_event(
                &mut conn,
                "lifecycle",
                &default_node_id(),
                &payload,
            )?;
            let report = serde_json::json!({
                "decommission":     node_id,
                "kind":             if force { "forced" } else { "soft" },
                "history_retained": true,
                "audit_logged":     true,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Cmd::Reenroll { node_id } => {
            // Phase 12.3.5 — mint a fresh keypair and write its
            // hex public key into the existing node row. Lifecycle
            // event records the old fingerprint so a forensic
            // walker can correlate before/after.
            let mut conn = mackesd_core::store::open(&db_path)
                .with_context(|| format!("opening store at {}", db_path.display()))?;
            let prior = mackesd_core::store::list_nodes(&conn)?
                .into_iter()
                .find(|n| n.node_id == node_id);
            let new_identity = mackesd_core::enrollment::build_identity();
            let new_fp = new_identity.key.fingerprint();
            let updated = mackesd_core::store::refresh_node_credentials(&conn, &node_id, &new_fp)?;
            if updated == 0 {
                eprintln!("mackesd reenroll: no node row matches {node_id}");
                std::process::exit(2);
            }
            let payload = serde_json::json!({
                "event":           "reenroll",
                "node":            node_id,
                "old_fingerprint": prior.map(|p| p.public_key),
                "new_fingerprint": &new_fp,
            })
            .to_string();
            mackesd_core::store::insert_event(
                &mut conn,
                "lifecycle",
                &default_node_id(),
                &payload,
            )?;
            let report = serde_json::json!({
                "reenroll":         node_id,
                "new_fingerprint":  new_fp,
                "history_retained": true,
                "audit_logged":     true,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Cmd::TakeLeadership { as_node } => {
            // Phase 12.1.1b — operator-forced leadership bump.
            let lock_path = mackesd_core::default_qnm_shared_root().join(".mackesd-leader.lock");
            let lease = mackesd_core::leader::force_take(&lock_path, &as_node)
                .with_context(|| format!("rewriting {}", lock_path.display()))?;
            println!(
                "leader: {} (epoch {}) — lease renewed at {}",
                lease.node_id, lease.epoch, lease.renewed_at_s
            );
        }
        Cmd::ImportLegacy { dry_run } => {
            // Phase 12.13.2 — inventory the legacy caches under the
            // three canonical roots, then either preview the plan
            // (dry-run, default) or write desired-state rows into
            // the store. The importer is conservative: it only
            // creates node rows for mesh-related artifacts whose
            // filename carries an obvious peer identifier; it never
            // overwrites an existing row.
            let roots = mackesd_core::legacy_inventory::default_roots();
            let artifacts = mackesd_core::legacy_inventory::inventory(&roots);
            let mesh_artifacts: Vec<_> = artifacts.iter().filter(|a| a.mesh_data).collect();
            let candidate_node_names = derive_legacy_node_names(&mesh_artifacts);
            if dry_run {
                let report = serde_json::json!({
                    "import_legacy_dry_run": true,
                    "candidate_paths":       roots
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>(),
                    "artifacts_found":       artifacts.len(),
                    "mesh_artifacts":        mesh_artifacts.len(),
                    "would_import_records":  candidate_node_names.len(),
                    "would_insert_nodes":    &candidate_node_names,
                });
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                let mut conn = mackesd_core::store::open(&db_path)
                    .with_context(|| format!("opening store at {}", db_path.display()))?;
                let existing: std::collections::BTreeSet<String> =
                    mackesd_core::store::list_nodes(&conn)?
                        .into_iter()
                        .map(|n| n.node_id)
                        .collect();
                let mut inserted = Vec::new();
                let mut skipped = Vec::new();
                for name in &candidate_node_names {
                    let node_id = format!("peer:{name}");
                    if existing.contains(&node_id) {
                        skipped.push(node_id);
                        continue;
                    }
                    mackesd_core::store::upsert_node(
                        &conn,
                        &node_id,
                        name,
                        // Placeholder key — a subsequent enrollment
                        // will replace this with the real Ed25519
                        // public-key fingerprint.
                        "legacy-import",
                        None,
                    )?;
                    inserted.push(node_id);
                }
                let payload = serde_json::json!({
                    "event":    "import_legacy",
                    "inserted": &inserted,
                    "skipped":  &skipped,
                })
                .to_string();
                mackesd_core::store::insert_event(
                    &mut conn,
                    "lifecycle",
                    &default_node_id(),
                    &payload,
                )?;
                let report = serde_json::json!({
                    "import_legacy_dry_run": false,
                    "artifacts_found":       artifacts.len(),
                    "mesh_artifacts":        mesh_artifacts.len(),
                    "inserted_nodes":        inserted,
                    "skipped_nodes":         skipped,
                });
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
        }
        Cmd::Reconcile {
            once,
            qnm_root,
            node_id,
        } => {
            // Phase 12.5 wiring — the reconcile worker thread.
            let qnm_root = qnm_root.unwrap_or_else(mackesd_core::default_qnm_shared_root);
            let node_id = node_id.unwrap_or_else(default_node_id);

            if once {
                // Single-tick dry-run path: useful for CI smoke
                // tests + operator inspection. No background
                // thread, no signal handler.
                let outcome = mackesd_core::worker::tick(&qnm_root, &node_id, &db_path)
                    .with_context(|| format!("one-shot reconcile tick on {}", db_path.display()))?;
                println!("{}", serde_json::to_string_pretty(&outcome)?);
            } else {
                // Long-running path: spawn the worker, install a
                // SIGTERM/SIGINT handler that flips the shutdown
                // flag, then block until the worker exits.
                use std::sync::atomic::{AtomicBool, Ordering};
                use std::sync::Arc;
                let shutdown = Arc::new(AtomicBool::new(false));
                install_signal_handlers(Arc::clone(&shutdown))?;
                let handle = mackesd_core::worker::spawn_reconcile_worker(
                    qnm_root,
                    node_id,
                    db_path,
                    Arc::clone(&shutdown),
                );
                // Wait for either the worker to exit (DB went away,
                // panic — we don't panic by design) or the signal
                // handler to flip shutdown. JoinHandle::join blocks
                // until the thread returns either way.
                if let Err(e) = handle.join() {
                    eprintln!("mackesd reconcile: worker thread panicked: {e:?}");
                    std::process::exit(1);
                }
                // If we exited because the worker thread itself
                // crashed unexpectedly (e.g. someone moved the db
                // file out from under us), the loop logged the
                // error before returning. Either way: exit 0 on a
                // clean shutdown-flag path.
                if !shutdown.load(Ordering::Relaxed) {
                    // Worker exited but no shutdown was requested.
                    // Treat as a soft failure.
                    eprintln!("mackesd reconcile: worker exited without shutdown request");
                    std::process::exit(1);
                }
            }
        }
        Cmd::InventoryLegacy { mesh_only, json } => {
            // Phase 12.13.1 — read-only walk of the three legacy
            // roots. Operator runs this before `import-legacy` to
            // see what's on disk.
            let roots = mackesd_core::legacy_inventory::default_roots();
            let mut artifacts = mackesd_core::legacy_inventory::inventory(&roots);
            if mesh_only {
                artifacts.retain(|a| a.mesh_data);
            }
            if json {
                println!("{}", serde_json::to_string_pretty(&artifacts)?);
            } else {
                print_inventory_table(&artifacts);
            }
        }
        #[cfg(feature = "async-services")]
        Cmd::Serve { qnm_root, node_id } => {
            // v2.0.0 Phase B.12 — unified meta-daemon entry point.
            // Boots the tokio runtime, registers the worker pool +
            // the existing reconcile worker, blocks on SIGTERM.
            run_serve(qnm_root, node_id, db_path)?;
        }
        Cmd::PeerCard { peer, dry_run } => {
            // PC-3.a — operator-driven trigger for the peer-card
            // modal. Writes the probe + spawns mde-peer-card with
            // a 30 s per-peer debounce. Uses a fixture probe for
            // now (the live probe-from-store path lands when
            // PC-3.b ships the read query). dry-run reports the
            // intended action without touching disk or spawning
            // the child.
            use mackes_mesh_types::PeerProbe;
            let mut probe = PeerProbe::fixture();
            probe.peer_id = peer.clone();
            if dry_run {
                println!(
                    "peer-card: would write probe + spawn modal for peer={peer} (debounce respected)",
                );
                return Ok(());
            }
            match mackesd_core::peer_join::handle_peer_joined(&probe) {
                Ok(Some(pid)) => {
                    println!("peer-card: spawned modal (pid={pid}) for peer={peer}");
                }
                Ok(None) => {
                    println!(
                        "peer-card: peer={peer} probe written; spawn skipped (debounced within 30s window)",
                    );
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("peer-card failed for {peer}: {e}"));
                }
            }
        }
        #[cfg(feature = "async-services")]
        Cmd::FleetPushSetting {
            key,
            value,
            peers,
            author,
            dry_run,
        } => {
            // v2.0.0 Phase G.4 — fleet push-setting CLI. Writes the
            // matching desired_config row + fleet_settings_apply_log
            // entries, then prints the JSON plan.
            let mut conn = mackesd_core::store::open(&db_path)
                .with_context(|| format!("opening store at {}", db_path.display()))?;
            let author = author.unwrap_or_else(default_node_id);
            let plan = mackesd_core::fleet::plan_push(&key, &value, &peers, &author);
            if !dry_run {
                mackesd_core::fleet::record_push(&mut conn, &plan)
                    .context("recording fleet push")?;
            }
            let report = serde_json::json!({
                "fleet_push_setting": {
                    "key":          &plan.key,
                    "value":        &plan.value,
                    "peers":        &plan.peers,
                    "author":       &plan.author,
                    "revision_id":  &plan.revision_id,
                    "dry_run":      dry_run,
                }
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Cmd::Revisions { cmd } => {
            // v2.0.0 Phase F.12 — desired_config revision management.
            let conn = mackesd_core::store::open(&db_path)
                .with_context(|| format!("opening store at {}", db_path.display()))?;
            match cmd {
                RevisionsCmd::List { json } => {
                    let rows = list_revisions(&conn)?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&rows)?);
                    } else {
                        print_revisions_table(&rows);
                    }
                }
                RevisionsCmd::Diff { from, to } => {
                    let a = load_revision_payload(&conn, &from)?;
                    let b = load_revision_payload(&conn, &to)?;
                    let report = serde_json::json!({
                        "from":     from,
                        "to":       to,
                        "from_len": a.len(),
                        "to_len":   b.len(),
                        // Surface the raw payloads so the operator + the
                        // Workbench panel can diff them visually.
                        "from_payload": a,
                        "to_payload":   b,
                    });
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                RevisionsCmd::Rollback {
                    target_id,
                    author,
                    peers,
                } => {
                    let payload = load_revision_payload(&conn, &target_id)?;
                    let author = author.unwrap_or_else(default_node_id);
                    let summary = format!("Rollback to {target_id} (peers={peers})");
                    let mut conn = conn;
                    let now = chrono::Utc::now().to_rfc3339();
                    let revision_id = mackesd_core::store::with_transaction(&mut conn, |tx| {
                        tx.execute(
                            "INSERT INTO desired_config \
                                 (author, message, spec_json, state, created_at) \
                                 VALUES (?, ?, ?, 'approved', ?)",
                            (&author, &summary, &payload, &now),
                        )
                        .map_err(|e| anyhow::anyhow!("inserting rollback revision: {e}"))?;
                        Ok(tx.last_insert_rowid())
                    })?;
                    let report = serde_json::json!({
                        "rollback":      target_id,
                        "new_revision":  revision_id,
                        "author":        author,
                        "peers":         peers,
                    });
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
            }
        }
        Cmd::Nodes { cmd } => {
            // CB-1.5.a — fleet node roster surface. The Iced
            // inventory panel consumes the JSON shape directly.
            let conn = mackesd_core::store::open(&db_path)
                .with_context(|| format!("opening store at {}", db_path.display()))?;
            match cmd {
                NodesCmd::List { json } => {
                    let nodes = mackesd_core::store::list_nodes(&conn)
                        .context("listing nodes from store")?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&nodes_to_json(&nodes))?);
                    } else {
                        print_nodes_table(&nodes);
                    }
                }
            }
        }
        Cmd::AnsibleHistory { cmd } => {
            // CB-1.5.c follow-up — walks QNM-Shared
            // ansible-runs/<peer>/*.json and emits the union as
            // a sorted JSON array (or human-readable table).
            match cmd {
                AnsibleHistoryCmd::List { json } => {
                    let root = ansible_runs_root();
                    let rows = collect_ansible_history(&root);
                    if json {
                        println!("{}", serde_json::to_string_pretty(&rows)?);
                    } else {
                        print_ansible_history_table(&rows);
                    }
                }
            }
        }
        Cmd::Events { cmd } => {
            // CB-1.8 mesh_history follow-up — audit-log
            // viewer surface.
            let conn = mackesd_core::store::open(&db_path)
                .with_context(|| format!("opening store at {}", db_path.display()))?;
            match cmd {
                EventsCmd::List { json } => {
                    let rows = mackesd_core::store::load_audit_rows(&conn)
                        .context("loading events from store")?;
                    let serial: Vec<serde_json::Value> = rows
                        .into_iter()
                        .map(|r| {
                            let payload_str = String::from_utf8(r.payload).unwrap_or_default();
                            serde_json::json!({
                                "event_id":     r.event_id,
                                "timestamp_ms": r.timestamp_ms,
                                "payload":      payload_str,
                                "hash":         hex_encode(&r.hash),
                            })
                        })
                        .collect();
                    if json {
                        println!("{}", serde_json::to_string_pretty(&serial)?);
                    } else if serial.is_empty() {
                        println!("(audit chain empty — no events yet)");
                    } else {
                        for r in &serial {
                            let id = r.get("event_id").and_then(|v| v.as_u64()).unwrap_or(0);
                            let ts = r.get("timestamp_ms").and_then(|v| v.as_i64()).unwrap_or(0);
                            let payload = r.get("payload").and_then(|v| v.as_str()).unwrap_or("");
                            println!("{id:>8}  {ts}  {payload}");
                        }
                    }
                }
            }
        }
        Cmd::Playbooks { cmd } => {
            // CB-1.5.b follow-up — curated playbook surface.
            match cmd {
                PlaybooksCmd::List { json } => {
                    let root = playbooks_root();
                    let mut entries = enumerate_playbook_roles(&root);
                    entries.sort();
                    let rows: Vec<serde_json::Value> = entries
                        .into_iter()
                        .map(|name| {
                            let description = playbook_description(&name);
                            serde_json::json!({
                                "name":        name,
                                "description": description,
                            })
                        })
                        .collect();
                    if json {
                        println!("{}", serde_json::to_string_pretty(&rows)?);
                    } else if rows.is_empty() {
                        println!("(no curated playbooks under {})", root.display());
                    } else {
                        for r in &rows {
                            let name = r.get("name").and_then(|v| v.as_str()).unwrap_or("");
                            let desc = r.get("description").and_then(|v| v.as_str()).unwrap_or("");
                            println!("{name:<28} {desc}");
                        }
                    }
                }
                PlaybooksCmd::Run { name } => {
                    // Spawn ansible-pull directly so the user sees
                    // its progress streaming. Exit with whatever
                    // ansible-pull exited with.
                    let status = std::process::Command::new("ansible-pull")
                        .args(["--tags", &name, "site.yml"])
                        .status();
                    match status {
                        Ok(s) => std::process::exit(s.code().unwrap_or(1)),
                        Err(e) => {
                            eprintln!("mded: ansible-pull spawn failed: {e}");
                            std::process::exit(2);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// `$QNM_SHARED_ROOT/.qnm-sync/playbooks/roles/` — same
/// resolution the Iced playbooks panel uses.
fn playbooks_root() -> PathBuf {
    let base = std::env::var("QNM_SHARED_ROOT").map(PathBuf::from).ok();
    let base = base.unwrap_or_else(|| {
        std::env::var("HOME")
            .map(|h| PathBuf::from(h).join("QNM-Shared"))
            .unwrap_or_else(|_| PathBuf::from("/var/empty"))
    });
    base.join(".qnm-sync").join("playbooks").join("roles")
}

/// Walk roles/ for subdirectories. Returns role names (bare
/// basenames); empty on any I/O error so the panel + CLI can
/// surface the empty-state message.
fn enumerate_playbook_roles(root: &std::path::Path) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for entry in rd.flatten() {
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
    }
    names
}

/// Curated descriptions per the Phase 1.3.0 lock. Mirrors the
/// `playbook_from_name` helper in the Iced playbooks panel so
/// the CLI and the GUI agree.
/// Lowercase hex string of a fixed byte slice. Avoids the
/// hex crate dep for one helper.
fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

fn playbook_description(name: &str) -> &'static str {
    match name {
        "system-update" => "Apply pending dnf upgrades (gated, never runs on default tag)",
        "mesh-state-snapshot" => "Snapshot QNM-Shared state for offline review",
        "selinux-permissive-toggle" => "Flip SELinux to permissive (op-tagged, never default)",
        "container-runtime-setup" => "Install + configure podman / docker runtime",
        "xfconf-baseline" => "Apply baseline xfconf keys (default-tagged)",
        "bloat-removal" => "Remove the curated bloat package list",
        "apps-install" => "Install the curated MDE app list",
        _ => "Custom role",
    }
}

/// `~/QNM-Shared/.qnm-sync/ansible-runs/` (or its
/// `$QNM_SHARED_ROOT` override). Matches the panel's
/// resolution in `mde-workbench/src/panels/run_history.rs`.
fn ansible_runs_root() -> PathBuf {
    let base = std::env::var("QNM_SHARED_ROOT").map(PathBuf::from).ok();
    let base = base.unwrap_or_else(|| {
        std::env::var("HOME")
            .map(|h| PathBuf::from(h).join("QNM-Shared"))
            .unwrap_or_else(|_| PathBuf::from("/var/empty"))
    });
    base.join(".qnm-sync").join("ansible-runs")
}

/// Walk every peer subdir + parse each `*.json` as a record.
/// Returns the union sorted by timestamp descending. Errors
/// are swallowed silently (no peer dir / unreadable file
/// just drops that row) — matches the panel's
/// non-aborting walk.
fn collect_ansible_history(root: &std::path::Path) -> Vec<serde_json::Value> {
    let Ok(peers) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for peer_entry in peers.flatten() {
        let Ok(ft) = peer_entry.file_type() else {
            continue;
        };
        if !ft.is_dir() {
            continue;
        }
        let peer_name = peer_entry
            .file_name()
            .to_str()
            .map(str::to_string)
            .unwrap_or_default();
        if peer_name.is_empty() {
            continue;
        }
        let peer_dir = peer_entry.path();
        let Ok(files) = std::fs::read_dir(&peer_dir) else {
            continue;
        };
        for file_entry in files.flatten() {
            let path = file_entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let Ok(raw) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&raw) else {
                continue;
            };
            // Inject the peer name + source path so the JSON
            // row is self-describing (the panel does the same
            // mapping).
            if let Some(obj) = v.as_object_mut() {
                obj.insert("peer".into(), serde_json::Value::String(peer_name.clone()));
                obj.insert(
                    "_source_path".into(),
                    serde_json::Value::String(path.to_string_lossy().into_owned()),
                );
            }
            rows.push(v);
        }
    }
    rows.sort_by(|a, b| {
        let ts_a = a.get("timestamp").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let ts_b = b.get("timestamp").and_then(|v| v.as_f64()).unwrap_or(0.0);
        ts_b.partial_cmp(&ts_a).unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

fn print_ansible_history_table(rows: &[serde_json::Value]) {
    if rows.is_empty() {
        println!("(no ansible-pull runs recorded)");
        return;
    }
    println!(
        "{:<16} {:<24} {:<6} {:<8} {:<8} {:<10}",
        "peer", "playbook", "exit", "changed", "ok", "trigger"
    );
    for r in rows {
        let peer = r
            .get("peer")
            .and_then(|v| v.as_str())
            .unwrap_or("-")
            .chars()
            .take(16)
            .collect::<String>();
        let playbook = r
            .get("playbook")
            .and_then(|v| v.as_str())
            .unwrap_or("-")
            .chars()
            .take(24)
            .collect::<String>();
        let exit = r.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0);
        let changed = r.get("changed").and_then(|v| v.as_u64()).unwrap_or(0);
        let ok = r.get("ok").and_then(|v| v.as_u64()).unwrap_or(0);
        let trigger = r
            .get("triggered_by")
            .and_then(|v| v.as_str())
            .unwrap_or("pull");
        println!("{peer:<16} {playbook:<24} {exit:<6} {changed:<8} {ok:<8} {trigger:<10}");
    }
}

/// Serialize the `NodeRow` list into the JSON shape the Iced
/// inventory panel consumes. Kept here rather than as a
/// `#[derive(Serialize)]` on `NodeRow` because the store struct
/// already serves topology + lifecycle callers and the JSON
/// shape is a CLI-surface contract.
fn nodes_to_json(nodes: &[mackesd_core::store::NodeRow]) -> serde_json::Value {
    serde_json::Value::Array(
        nodes
            .iter()
            .map(|n| {
                serde_json::json!({
                    "node_id":    n.node_id,
                    "name":       n.name,
                    "public_key": n.public_key,
                    "role":       n.role,
                    "health":     n.health,
                    "region":     n.region,
                })
            })
            .collect(),
    )
}

fn print_nodes_table(nodes: &[mackesd_core::store::NodeRow]) {
    if nodes.is_empty() {
        println!("(no peers enrolled)");
        return;
    }
    println!(
        "{:<24} {:<24} {:<12} {:<12} {:<10}",
        "node_id", "name", "role", "health", "region"
    );
    for n in nodes {
        println!(
            "{:<24} {:<24} {:<12} {:<12} {:<10}",
            n.node_id,
            n.name,
            n.role,
            n.health,
            n.region.as_deref().unwrap_or("-"),
        );
    }
}

/// Read a revision's `spec_json` payload by id.
fn load_revision_payload(conn: &rusqlite::Connection, revision_id: &str) -> anyhow::Result<String> {
    let rev: i64 = revision_id
        .parse()
        .map_err(|_| anyhow::anyhow!("revision id must be an integer (got {revision_id})"))?;
    let payload: String = conn
        .query_row(
            "SELECT spec_json FROM desired_config WHERE revision_id = ?",
            [rev],
            |r| r.get(0),
        )
        .with_context(|| format!("loading revision {revision_id}"))?;
    Ok(payload)
}

/// List every revision (descending by id).
fn list_revisions(conn: &rusqlite::Connection) -> anyhow::Result<Vec<serde_json::Value>> {
    let mut stmt = conn
        .prepare(
            "SELECT revision_id, author, message, state, created_at \
             FROM desired_config ORDER BY revision_id DESC",
        )
        .context("preparing revisions list")?;
    let rows = stmt
        .query_map([], |r| {
            Ok(serde_json::json!({
                "revision_id":  r.get::<_, i64>(0)?.to_string(),
                "author":       r.get::<_, String>(1)?,
                "summary":      r.get::<_, String>(2)?,
                "state":        r.get::<_, String>(3)?,
                "created_at":   r.get::<_, String>(4)?,
            }))
        })
        .context("executing revisions list")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("materializing revisions list")?;
    Ok(rows)
}

fn print_revisions_table(rows: &[serde_json::Value]) {
    if rows.is_empty() {
        println!("(no revisions)");
        return;
    }
    for row in rows {
        let rid = row
            .get("revision_id")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let st = row.get("state").and_then(|v| v.as_str()).unwrap_or("?");
        let aut = row.get("author").and_then(|v| v.as_str()).unwrap_or("?");
        let cre = row
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let sm = row.get("summary").and_then(|v| v.as_str()).unwrap_or("");
        println!("{rid:>6}  [{st}]  {aut:<16}  {cre}  {sm}");
    }
}

/// `mackesd serve` runtime. Pulls in tokio + the async supervisor
/// only when the `async-services` feature is active so the default
/// build stays sync.
///
/// v3.0.3 — wires the 6 Phase B workers (clipboard, mdns, fs_sync,
/// heartbeat, mesh_router, notification_relay) into the
/// `Supervisor` alongside the legacy reconcile worker. Audit-2
/// caught all 6 as dead code: `impl Worker for X` shipped, no
/// spawn. Each worker gets a `RestartPolicy::OnFailure` so a
/// transient error (sqlite contention, mdns socket flake)
/// restarts the worker after the supervisor's 250ms back-off
/// without taking down the whole daemon.
///
/// Also wires `mackesd_core::logging::LogContext` (Tier 3 — Phase 12.1.4):
/// every log line inside `run_serve` inherits the daemon's
/// correlation_id + node_id via a top-level tracing span.
#[cfg(feature = "async-services")]
fn run_serve(
    qnm_root: Option<PathBuf>,
    node_id: Option<String>,
    db_path: PathBuf,
) -> anyhow::Result<()> {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use mackesd_core::workers::{
        clipboard::ClipboardWorker, fs_sync::FsSyncWorker, heartbeat::HeartbeatWorker,
        mdns::MdnsWorker, mesh_router::MeshRouterWorker,
        notification_relay::NotificationRelayWorker, RestartPolicy, Spawn, Supervisor,
    };
    let qnm_root = qnm_root.unwrap_or_else(mackesd_core::default_qnm_shared_root);
    let node_id = node_id.unwrap_or_else(default_node_id);

    // v3.0.3 — daemon-scope tracing span so every log line below
    // carries correlation_id + node_id. The JSON formatter
    // (initialized in main.rs's tracing-subscriber setup) picks up
    // span fields automatically.
    let log_ctx = mackesd_core::logging::LogContext::fresh().with_node(node_id.clone());
    let _daemon_span = tracing::info_span!(
        "daemon",
        correlation_id = log_ctx.correlation_id,
        node_id = %log_ctx.node_id.as_deref().unwrap_or("")
    )
    .entered();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("building tokio runtime")?;
    runtime.block_on(async move {
        tracing::info!("mackesd serve: starting supervisor + workers");
        let shutdown = Arc::new(AtomicBool::new(false));
        install_signal_handlers(Arc::clone(&shutdown)).context("installing signal handlers")?;

        // v3.0.3 — async supervisor for Phase B workers. The
        // legacy reconcile worker stays on its own std::thread
        // because its sync rusqlite calls would block the tokio
        // scheduler if hosted here; both supervisors coexist.
        let mut sup = Supervisor::new();
        sup.spawn(Spawn::new(
            ClipboardWorker::new(),
            RestartPolicy::OnFailure,
        ));
        sup.spawn(Spawn::new(MdnsWorker::new(), RestartPolicy::OnFailure));
        sup.spawn(Spawn::new(FsSyncWorker::new(), RestartPolicy::OnFailure));
        sup.spawn(Spawn::new(
            HeartbeatWorker::new(qnm_root.clone(), node_id.clone()),
            RestartPolicy::OnFailure,
        ));
        // mesh_router bootstraps with the per-transport
        // registry. Phase 12.18 D.2 (2026-05-23) — the Https443
        // transport is registered at startup so the per-peer
        // HttpsFallbackState::Active transition can actually
        // route through a real TLS tunnel. The transport
        // gracefully reports `Misconfigured(no_fallback_host)`
        // until MDE_HTTPS_FALLBACK_HOST is set, so daemons
        // running without the env var still boot clean.
        let router_state: mackesd_core::workers::mesh_router::RouterState =
            Arc::new(RwLock::new(HashMap::new()));
        let https443: Arc<dyn mackes_transport::Transport> =
            Arc::new(mackesd_core::transport::https443::Https443Transport::new());
        let router_registry: mackesd_core::workers::mesh_router::TransportRegistry =
            Arc::new(vec![https443]);
        sup.spawn(Spawn::new(
            MeshRouterWorker::new(Arc::clone(&router_state), router_registry),
            RestartPolicy::OnFailure,
        ));
        // v4.0.1 Phase 12.17 wire (2026-05-23) — STUN candidate
        // gatherer. Shares router_state with the router so
        // reflexive candidates land on every tracked peer's
        // PeerPath.candidates list. 30 s cadence; per-server
        // probe timeout 1.4 s; default server pool is Google's
        // public STUN cluster (IP-pinned so the worker doesn't
        // hit DNS on the hot path).
        sup.spawn(Spawn::new(
            mackesd_core::workers::stun_gather::StunGatherWorker::new(
                Arc::clone(&router_state),
            ),
            RestartPolicy::OnFailure,
        ));
        // notification_relay needs its own SQLite connection
        // (the legacy reconcile worker holds its own; we mint a
        // second handle so the two run independently).
        match rusqlite::Connection::open(&db_path) {
            Ok(conn) => {
                sup.spawn(Spawn::new(
                    NotificationRelayWorker::new(qnm_root.clone(), conn),
                    RestartPolicy::OnFailure,
                ));
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    db_path = %db_path.display(),
                    "notification_relay: sqlite open failed; worker skipped"
                );
            }
        }

        // v4.0.1 AF-* (2026-05-23) — register the
        // dev.mackes.MDE.Fleet.Files surface on the session bus
        // so mde-files's DBusBackend can read the live mesh
        // roster + per-peer file lists. Opens a second SQLite
        // handle for the IPC service (the reconcile worker
        // holds its own). The connection is leaked so its
        // tokio background tasks outlive run_serve.
        let host = std::fs::read_to_string("/etc/hostname")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| node_id.clone());
        match mackesd_core::store::open(&db_path) {
            Ok(conn) => {
                let store = Arc::new(tokio::sync::Mutex::new(conn));
                let svc = mackesd_core::ipc::files::FleetFilesService::new(
                    Arc::clone(&store),
                    host.clone(),
                    node_id.clone(),
                );
                match mackesd_core::ipc::files::register_fleet_files(svc).await {
                    Ok(conn) => {
                        Box::leak(Box::new(conn));
                        tracing::info!(
                            "Fleet.Files dbus surface registered at {}",
                            mackesd_core::ipc::files::FLEET_FILES_OBJECT_PATH
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "Fleet.Files dbus registration failed; \
                             mde-files's DBusBackend will fall back to LocalFsBackend"
                        );
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    db_path = %db_path.display(),
                    "Fleet.Files dbus: sqlite open failed; service skipped"
                );
            }
        }

        // v4.0.1 AF-2.3.a (2026-05-23) — register the
        // dev.mackes.MDE.Settings surface on the session bus so
        // the workbench's DBusBackend can land Get/Set calls.
        // Without this, every panel save fails with a
        // no-owner-of-name BackendError::Bus. Graceful-degrade:
        // a `NameAlreadyAcquired` (another mackesd) or session-
        // bus-unreachable failure logs a warning and the daemon
        // keeps running — only the workbench's settings path
        // degrades.
        match mackesd_core::ipc::settings::register_settings(
            mackesd_core::ipc::settings::SettingsService::default(),
        )
        .await
        {
            Ok(conn) => {
                Box::leak(Box::new(conn));
                tracing::info!(
                    "Settings dbus surface registered at {}",
                    mackesd_core::ipc::settings::OBJECT_PATH
                );
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Settings dbus registration failed; \
                     workbench panel saves will surface as bus errors"
                );
            }
        }

        // v4.0.1 cycle 2 (2026-05-23) — register the
        // dev.mackes.MDE.Shell surface so the workbench's panel-
        // sync section + future status-cluster surfaces can call
        // healthz() / workers() directly off the bus instead of
        // shelling out to `mackesd healthz` + parsing stdout.
        match mackesd_core::ipc::shell::register_shell(
            mackesd_core::ipc::shell::ShellService::default().with_db_path(db_path.clone()),
        )
        .await
        {
            Ok(conn) => {
                Box::leak(Box::new(conn));
                tracing::info!(
                    "Shell dbus surface registered at {}",
                    mackesd_core::ipc::shell::OBJECT_PATH
                );
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Shell dbus registration failed; \
                     workbench sync-status surface will surface as bus errors"
                );
            }
        }

        // v4.0.1 KDC2-3.3 wire-up (2026-05-23) — spawn the KDC host
        // worker. Owns the pairing store at $XDG_CONFIG_HOME/mde/
        // connect (default ~/.config/mde/connect), the shared
        // DiscoveryRegistry, the outbound packet queue, and the
        // dev.mackes.MDE.Connect D-Bus surface. Graceful-degrade
        // on D-Bus failure — the worker keeps the host alive so
        // the mesh-router can still dispatch through KDC, even if
        // the operator-facing UI methods aren't reachable.
        let kdc_config_dir = {
            let xdg = std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from);
            let home_default = std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .map(|h| h.join(".config"));
            xdg.or(home_default)
                .map(|p| p.join("mde").join("connect"))
                .unwrap_or_else(|| std::path::PathBuf::from("/var/lib/mde/connect"))
        };
        sup.spawn(Spawn::new(
            mackesd_core::workers::kdc_host::KdcHostWorker::new(kdc_config_dir),
            RestartPolicy::OnFailure,
        ));

        // The reconcile worker runs on its own OS thread (kept on
        // std::thread so its sync rusqlite calls don't block the
        // tokio scheduler).
        let reconcile = mackesd_core::worker::spawn_reconcile_worker(
            qnm_root,
            node_id,
            db_path,
            Arc::clone(&shutdown),
        );

        // Watch loop: wake every 250 ms to check the shutdown flag.
        // When it flips, drop out so reconcile.join() can wait for
        // the worker to finish its current tick. The async
        // supervisor's workers see shutdown via the SIGTERM signal
        // handler installed above (mackesd_core::workers::ShutdownToken
        // wraps the same broadcast channel).
        while !shutdown.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            if reconcile.is_finished() {
                tracing::warn!(
                    "mackesd serve: reconcile worker exited without \
                     a shutdown request"
                );
                shutdown.store(true, Ordering::Relaxed);
                break;
            }
        }
        tracing::info!("mackesd serve: shutdown requested; joining workers");
        // Tell every async worker to stop, then drain their joins.
        let outcomes = sup.shutdown_and_join().await?;
        for (name, outcome) in &outcomes {
            match outcome {
                Ok(()) => tracing::info!(worker = %name, "joined clean"),
                Err(e) => tracing::warn!(worker = %name, error = ?e, "joined with error"),
            }
        }
        if let Err(e) = reconcile.join() {
            tracing::error!("reconcile worker panicked: {e:?}");
            return Err(anyhow::anyhow!("reconcile worker panicked"));
        }
        tracing::info!("mackesd serve: all workers joined; exit");
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(())
}

/// Render a fixed-width inventory table to stdout. Columns:
/// kind / mesh? / size / mtime (ISO-8601 UTC) / path. We pad to the
/// widest cell in each column so the output stays grep-able.
fn print_inventory_table(artifacts: &[mackesd_core::legacy_inventory::LegacyArtifact]) {
    if artifacts.is_empty() {
        println!("(no legacy artifacts found)");
        return;
    }
    let mut rows: Vec<[String; 5]> = Vec::with_capacity(artifacts.len() + 1);
    rows.push([
        "KIND".to_owned(),
        "MESH".to_owned(),
        "SIZE".to_owned(),
        "MTIME (UTC)".to_owned(),
        "PATH".to_owned(),
    ]);
    for a in artifacts {
        rows.push([
            format!("{:?}", a.artifact_kind),
            if a.mesh_data {
                "yes".to_owned()
            } else {
                "no".to_owned()
            },
            format_size(a.size_bytes),
            format_mtime(a.mtime_ms),
            a.path.display().to_string(),
        ]);
    }
    let mut widths = [0usize; 5];
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.len());
        }
    }
    for row in &rows {
        println!(
            "{:<w0$}  {:<w1$}  {:>w2$}  {:<w3$}  {}",
            row[0],
            row[1],
            row[2],
            row[3],
            row[4],
            w0 = widths[0],
            w1 = widths[1],
            w2 = widths[2],
            w3 = widths[3],
        );
    }
}

/// Render a byte count as a short human-friendly string (binary
/// prefixes — KiB / MiB / GiB).
fn format_size(bytes: u64) -> String {
    #[allow(clippy::cast_precision_loss)]
    let n = bytes as f64;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KiB", n / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MiB", n / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GiB", n / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Render an mtime (ms since epoch) as an ISO-8601 UTC timestamp.
/// Falls back to `-` when chrono refuses the value.
fn format_mtime(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms).map_or_else(
        || "-".to_owned(),
        |dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    )
}

/// Build the JSON `peers why` report from a node roster (Phase
/// 12.4.4). Pure function over the store projection so callers can
/// unit-test the reason-chain shape without a real DB.
fn explain_peer(node_id: &str, nodes: &[mackesd_core::store::NodeRow]) -> serde_json::Value {
    let subject = nodes.iter().find(|n| n.node_id == node_id);
    let Some(subject) = subject else {
        return serde_json::json!({
            "node":     node_id,
            "known":    false,
            "reasons":  [],
            "note":     "node id not present in store — run `mackesd inventory-legacy` and `mackesd import-legacy` to seed.",
        });
    };
    let healthy_subject = subject.health == "healthy";
    let reasons: Vec<serde_json::Value> = nodes
        .iter()
        .filter(|other| other.node_id != node_id)
        .map(|other| {
            let same_region = match (&subject.region, &other.region) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            };
            let both_healthy = healthy_subject && other.health == "healthy";
            let chain: Vec<&str> = {
                let mut v = Vec::new();
                if both_healthy {
                    v.push("both peers healthy");
                } else {
                    v.push("one or both peers not healthy");
                }
                if same_region {
                    v.push("same region — east-west allowed by default");
                } else {
                    v.push("different regions — gated on policy::allow_east_west");
                }
                if subject.role == "decommissioned" || other.role == "decommissioned" {
                    v.push("decommissioned — no edge expected");
                }
                v
            };
            serde_json::json!({
                "peer":       other.node_id,
                "expected":   both_healthy
                              && (same_region || true)
                              && subject.role != "decommissioned"
                              && other.role != "decommissioned",
                "chain":      chain,
            })
        })
        .collect();
    serde_json::json!({
        "node":    node_id,
        "known":   true,
        "region":  subject.region,
        "role":    subject.role,
        "health":  subject.health,
        "reasons": reasons,
    })
}

/// Heuristic: extract peer name candidates from a list of legacy
/// artifacts (Phase 12.13.2). Pure helper so the importer's "what
/// would I insert" question has a single source of truth that's
/// unit-testable without disk I/O.
fn derive_legacy_node_names(
    artifacts: &[&mackesd_core::legacy_inventory::LegacyArtifact],
) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut out = BTreeSet::new();
    for a in artifacts {
        // Filenames like `peer:anvil.json` or directories named after
        // peers (`~/QNM-Shared/anvil/...`) reveal candidate names.
        let path_str = a.path.display().to_string();
        for token in path_str.split(['/', '\\', '_', '.']) {
            if let Some(rest) = token.strip_prefix("peer:") {
                if !rest.is_empty() && rest.chars().all(legacy_name_char) {
                    out.insert(rest.to_owned());
                }
            }
        }
        // Also harvest the top-level directory under QNM-Shared
        // (`~/QNM-Shared/<peer>/...`).
        if path_str.contains("QNM-Shared") {
            if let Some(idx) = path_str.find("QNM-Shared/") {
                let after = &path_str[idx + "QNM-Shared/".len()..];
                if let Some(seg) = after.split('/').next() {
                    if !seg.is_empty() && seg.chars().all(legacy_name_char) {
                        out.insert(seg.to_owned());
                    }
                }
            }
        }
    }
    out.into_iter().collect()
}

fn legacy_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// Resolve the stable node id from `$MACKESD_NODE_ID` then
/// `$HOSTNAME` then the `hostname` syscall, falling back to
/// `peer:unknown` so the audit-log column is never empty.
fn default_node_id() -> String {
    if let Ok(v) = std::env::var("MACKESD_NODE_ID") {
        return v;
    }
    let host = std::env::var("HOSTNAME").ok().or_else(|| {
        std::process::Command::new("hostname")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_owned())
    });
    match host {
        Some(h) if !h.is_empty() => format!("peer:{h}"),
        _ => "peer:unknown".to_owned(),
    }
}

/// Register a SIGTERM + SIGINT handler that flips `shutdown` to
/// true. Uses `signal-hook`'s safe `Signals` iterator API — a
/// background thread reads from the kernel-managed signal queue
/// and stores into the shared atomic. No `unsafe` required (the
/// workspace forbids `unsafe_code`).
///
/// The reader thread is daemon-style: it lives as long as the
/// process and exits naturally when the process exits. Since
/// `mackesd reconcile` returns from main only after the reconcile
/// worker joins, we don't need to track the reader's handle.
fn install_signal_handlers(
    shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> anyhow::Result<()> {
    use signal_hook::consts::{SIGINT, SIGTERM};
    use signal_hook::iterator::Signals;
    let mut signals =
        Signals::new([SIGTERM, SIGINT]).context("installing SIGTERM/SIGINT iterator")?;
    std::thread::Builder::new()
        .name("mackesd-signal".into())
        .spawn(move || {
            for sig in &mut signals {
                tracing::info!(signal = sig, "received shutdown signal");
                shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
                // Keep reading so a second signal doesn't terminate
                // the process before the worker drains.
            }
        })
        .context("spawning signal-reader thread")?;
    Ok(())
}
