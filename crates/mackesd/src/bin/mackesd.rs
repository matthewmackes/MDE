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

    /// EPIC-MESH-PROBE — run the nmap probe engine (MESH-PROBE-2).
    Probe {
        #[command(subcommand)]
        action: ProbeCmd,
    },

    /// MESHFS-1.2 (v5.0.0) — pre-flight check for the LizardFS
    /// mesh-storage rollout. Walks the user's XDG dirs, sums
    /// on-disk bytes, queries `/var/lib/mde/meshfs` free space,
    /// prints a one-line OK / WARN / NoDataDir verdict + emits
    /// the full structured report as a JSON line to stdout.
    /// Exits 0 on OK, 1 on WARN / NoDataDir. Operators can run
    /// this before an upgrade to v5.0.0; the Workbench Mesh
    /// Storage panel (MESHFS-13.1) will surface the same report
    /// as a banner once it lands.
    PreflightMeshFsHeadroom {
        /// Override the LizardFS data parent dir. Defaults to
        /// `/var/lib/mde/meshfs` per the design lock.
        #[clap(long, default_value = "/var/lib/mde/meshfs")]
        data_dir: std::path::PathBuf,
        /// Override `$HOME` (used to locate the five XDG
        /// dirs). Defaults to the env-var `$HOME`.
        #[clap(long)]
        home: Option<std::path::PathBuf>,
    },

    /// MESHFS-13.1 (v5.0.0) — query the active LizardFS master
    /// and emit a fleet status JSON blob (peers, goal, quota cap,
    /// limiting peer). Printed to stdout as a single JSON line.
    /// Exits 0 when the master is reachable, 1 otherwise.
    MeshFsStatus {
        /// Floating overlay VIP the active master listens on.
        #[clap(long, default_value = "10.42.0.1")]
        vip: String,
        /// `mfsadmin` binary name (must be on PATH).
        #[clap(long, default_value = "mfsadmin")]
        admin_binary: String,
    },

    /// MESHFS-8.1 (v5.0.0) — list files recoverable from the LizardFS
    /// `.trash` virtual directory. Emits a JSON array of `TrashEntry`
    /// objects to stdout. Returns an empty array `[]` when the mount
    /// path is absent or `.trash` is empty.
    MeshFsTrashList {
        /// Mount path of the LizardFS client (must be mounted).
        #[clap(long, default_value = "/mnt/mesh-storage")]
        mount_path: String,
    },

    /// MESHFS-8.1 (v5.0.0) — restore one file from the LizardFS trash
    /// by invoking `mfsadmin <vip> TRASH-RECOVER <path>`. `--path` is
    /// the full trash entry path (as emitted by `meshfs-trash-list`).
    /// Exits 0 on success, 1 on failure.
    MeshFsUndelete {
        /// Full path of the `.trash` entry to recover.
        #[clap(long)]
        path: String,
        /// Floating overlay VIP the active master listens on.
        #[clap(long, default_value = "10.42.0.1")]
        vip: String,
        /// `mfsadmin` binary name (must be on PATH).
        #[clap(long, default_value = "mfsadmin")]
        admin_binary: String,
    },

    /// MESHFS-11.1 (v5.0.0) — archive a `.conflict-*` file to
    /// `~/Local/conflict-archive/<ts>/`, completing the Resolve flow
    /// in mde-files. The mde-files UI calls this with either the
    /// original or the conflict sibling, depending on which the
    /// operator chose to discard.
    MeshFsResolveConflict {
        /// Full path of the conflict file to archive (the loser).
        #[clap(long)]
        path: String,
    },

    /// MESHFS-14.1 (v5.0.0) — restore the Nebula CA + (when
    /// present) the LizardFS mesh-storage snapshot from an
    /// armored `state-backup.enc` bundle. CA rows go straight
    /// into the local SQLite store via
    /// `ca::backup::restore_to_store`; the optional LizardFS
    /// artifacts land at `<recovery-dir>/` for the operator
    /// to apply with `mfsmaster --import-metadata`. Automatic
    /// volume replay is intentionally out of scope — replaying
    /// a metadata dump against a live cluster requires careful
    /// peer-by-peer reconciliation that's an operator-driven
    /// step, not a silent CLI action.
    StateRestore {
        /// Path to the armored `state-backup.enc` bundle.
        bundle: std::path::PathBuf,
        /// Passphrase env-var. Defaults to
        /// `MDE_BACKUP_PASSPHRASE` (same as the daily backup
        /// worker's env).
        #[clap(long, default_value = "MDE_BACKUP_PASSPHRASE")]
        passphrase_env: String,
        /// Directory to write the LizardFS recovery artifacts
        /// for the operator-side manual replay. Created if
        /// missing. Default `/var/lib/mackesd/restore/meshfs`.
        #[clap(long, default_value = "/var/lib/mackesd/restore/meshfs")]
        recovery_dir: std::path::PathBuf,
    },

    /// Generate a fresh 16-char URL-safe passcode (Phase 12.10.1).
    /// Prints the passcode. With `--store` (EPIC-SEC-PASSCODE-CREDS),
    /// also encrypts it to the cred file via `systemd-creds` instead
    /// of printing the libsecret hint.
    GeneratePasscode {
        /// EPIC-SEC-PASSCODE-CREDS — encrypt the generated passcode
        /// to the cred file via `systemd-creds encrypt`.
        #[arg(long, default_value_t = false)]
        store: bool,
        /// Override the cred-file path (defaults to
        /// `/var/lib/mackesd/mesh-passcode.cred`).
        #[arg(long, value_name = "PATH")]
        cred_path: Option<PathBuf>,
    },

    /// Walk the `events` table forward and verify every row's hash
    /// (Phase 12.10.3). Exits 0 on Intact / Empty, 1 on Break.
    AuditVerify,

    /// Rotate the shared mesh passcode (Phase 12.10.2). Prints a
    /// freshly-generated passcode. With `--store`, encrypts it to
    /// the cred file via `systemd-creds`. Peers pick up the new
    /// passcode on their next heartbeat once the reconcile loop runs.
    RotatePasscode {
        /// EPIC-SEC-PASSCODE-CREDS — encrypt the rotated passcode to
        /// the cred file via `systemd-creds encrypt`.
        #[arg(long, default_value_t = false)]
        store: bool,
        /// Override the cred-file path.
        #[arg(long, value_name = "PATH")]
        cred_path: Option<PathBuf>,
    },

    /// EPIC-SEC-PASSCODE-CREDS — decrypt + print the mesh passcode
    /// stored via `systemd-creds` (the inverse of
    /// `generate-passcode --store`). Reads the cred file, runs
    /// `systemd-creds decrypt`, prints the plaintext to stdout.
    ShowPasscode {
        /// Override the cred-file path.
        #[arg(long, value_name = "PATH")]
        cred_path: Option<PathBuf>,
    },

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

    /// Enroll this peer against the mesh. Two flows:
    ///
    /// **Pre-v2.5 (passcode):** Phase 12.3.1 v1.x flow — generates
    /// an Ed25519 keypair + bearer token, prints a signed
    /// `EnrollmentRequest` JSON the leader ingests.
    ///
    /// **v2.5 Nebula (token):** NF-3.6.a — parses the
    /// `mesh:<id>@<ip>:<port>#<bearer>` join token, publishes a
    /// pending-enroll CSR to QNM-Shared, waits up to 30 s for the
    /// lighthouse to sign + write the bundle back. The
    /// `nebula_supervisor` worker materializes /etc/nebula/ once
    /// the bundle lands.
    ///
    /// `--passcode` and `--token` are mutually exclusive; exactly
    /// one must be set.
    Enroll {
        /// 16-character URL-safe shared passcode (v1.x flow).
        #[arg(long, conflicts_with = "token")]
        passcode: Option<String>,
        /// v2.5 Nebula join token —
        /// `mesh:<mesh_id>@<lighthouse_ip>:<port>#<bearer>`.
        #[arg(long, conflicts_with = "passcode")]
        token: Option<String>,
        /// Optional display name; defaults to the system hostname.
        #[arg(long)]
        name: Option<String>,
        /// Override the QNM-Shared root (defaults to
        /// `$QNM_SHARED_ROOT` or `~/QNM-Shared`). v2.5 token flow
        /// uses this to locate where the CSR + signed bundle live.
        #[arg(long, env = "QNM_SHARED_ROOT")]
        qnm_root: Option<PathBuf>,
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

    /// NF-2.6 (v2.5) — Nebula CA management subcommands.
    /// Mint / rotate / list / dump-ca the mesh-CA artifacts.
    Ca {
        /// Sub-subcommand selector — see `CaCmd` below.
        #[command(subcommand)]
        sub: CaCmd,
    },

    /// NF-18.x (v2.5) — Nebula peer + roster operations.
    /// Operator-facing reads against the live nebula_peer_certs
    /// + nodes tables.
    Nebula {
        #[command(subcommand)]
        sub: NebulaCmd,
    },

    /// VV-1 / VV-1.5 (v4.1.0) — Voice/Video stack operations.
    /// Today only `render-config` ships; VV-2 adds policy-driven
    /// reload, VV-14 adds Vitelity `uac.reg_dump`, etc.
    Voice {
        #[command(subcommand)]
        sub: VoiceCmd,
    },

    /// DEAD-2.5 (v5.1) + NF-21.2 (v1.0/1.1) — Wake-on-LAN.
    ///
    /// Default mode: fires the magic packet at the local broadcast
    /// address (works within one LAN segment). `--via-lighthouse <ip>`
    /// instead sends the magic packet as unicast over the Nebula
    /// overlay to a lighthouse, which de-encapsulates and re-broadcasts
    /// on the target's LAN — the "WoL across LANs" capability the v2.5
    /// cut enables.
    ///
    /// Replaces `mackes/mesh_wol.py::wake_peer` + `mesh_nebula.py::wol_via_lighthouse`.
    WakePeer {
        /// Target MAC in any canonical form: `aa:bb:cc:dd:ee:ff`,
        /// `aa-bb-cc-dd-ee-ff`, or `aabbccddeeff`.
        mac: String,
        /// Broadcast address to fire at. Defaults to the limited
        /// broadcast. Ignored when `--via-lighthouse` is set.
        #[clap(long, default_value = "255.255.255.255")]
        broadcast: String,
        /// Send via this lighthouse's overlay IP as unicast. The
        /// lighthouse-side relay re-broadcasts on the target LAN.
        /// Mutually exclusive with `--broadcast` (when both set,
        /// lighthouse mode wins).
        #[clap(long)]
        via_lighthouse: Option<String>,
        /// Destination UDP port. Standard ports are 7 + 9; 9 is the
        /// historical default and what every mainboard expects.
        #[clap(long, default_value_t = 9)]
        port: u16,
    },

    /// Portal-18.d (v6.0 R12, 2026-05-27) — fire `swaymsg exec <cmd>`
    /// for every entry in a preset tag's `launch_bundle`. The runtime
    /// entry point for Portal-18.d until Portal-17 Hub's tag-card
    /// click handler lands; operators (or Hub callbacks) invoke this
    /// to launch the bundle.
    ///
    /// Prints `launched <N>/<M>` summary; non-zero exit when any
    /// individual exec fails.
    PresetLaunch {
        /// Name of the preset tag to launch. Must exist in
        /// `<XDG_DATA_HOME>/mde/tags.json` with `TagFlavor::Preset`.
        tag: String,
    },
}

/// EPIC-MESH-PROBE — `mackesd probe <sub>` subcommands (MESH-PROBE-2).
/// The scheduled two-tier worker (MESH-PROBE-4) reuses the same
/// `probe_nmap` engine; this is the operator-facing manual surface.
#[derive(Subcommand)]
enum ProbeCmd {
    /// Run a one-shot nmap scan against `targets` and print the
    /// resulting inventory cards as JSON lines. Requires `nmap`
    /// (RPM `Requires: nmap`, MESH-PROBE-3); a missing binary prints
    /// nothing + exits 0 (graceful-degrade).
    Scan {
        /// Hosts / CIDRs to scan (e.g. `10.42.0.5`). At least one.
        #[clap(required = true)]
        targets: Vec<String>,
        /// Deep `-sV`/NSE identification pass (default: fast pass).
        #[clap(long)]
        deep: bool,
        /// Discovery source tag recorded on each host card:
        /// `mesh` (default) / `lan` / `arbitrary`.
        #[clap(long, default_value = "mesh")]
        source: String,
        /// Bundled-NSE script dir for the deep pass (MESH-PROBE-3).
        #[clap(long, default_value = "/usr/share/mde/nmap")]
        nse_dir: String,
    },
    /// Manual refresh (MESH-PROBE-4): run one deep probe cycle against
    /// the resolved mesh peers + write this peer's probe-inventory.json
    /// + announce probe/changed. Same engine the scheduled worker runs.
    Refresh {
        /// Mesh-home root (defaults to `$QNM_SHARED_ROOT` / `~/QNM-Shared`).
        #[clap(long, env = "QNM_SHARED_ROOT")]
        qnm_root: Option<PathBuf>,
        /// This peer's node-id (defaults to the daemon default).
        #[clap(long)]
        node_id: Option<String>,
        /// Bundled-NSE script dir for the deep pass.
        #[clap(long, default_value = "/usr/share/mde/nmap")]
        nse_dir: String,
    },
    /// List the merged mesh-wide probe inventory (MESH-PROBE-6): the
    /// union of every peer's `probe-inventory.json`. With `--service`,
    /// list only the hosts running that service kind.
    List {
        /// Mesh-home root (defaults to `$QNM_SHARED_ROOT` / `~/QNM-Shared`).
        #[clap(long, env = "QNM_SHARED_ROOT")]
        qnm_root: Option<PathBuf>,
        /// Filter to hosts running this service kind (e.g. `jellyfin`).
        #[clap(long)]
        service: Option<String>,
    },
}

/// VV-1 / VV-1.5 — `mackesd voice <sub>` subcommands.
#[derive(Subcommand)]
enum VoiceCmd {
    /// Regenerate the four kamailio-mde + rtpengine-mde config
    /// files (`kamailio.cfg`, `dispatcher.list`, `uacreg.list`,
    /// `rtpengine.conf`) from the current policy snapshot.
    ///
    /// Invoked by both `kamailio-mde.service` and
    /// `rtpengine-mde.service` as their `ExecStartPre=` hook on
    /// every (re)start, so the on-disk config is always coherent
    /// with the latest approved `voice_mesh` / `voice_public`
    /// policy revision.
    ///
    /// VV-1 ships the minimal generator: no peer routing, no
    /// Vitelity, just enough to boot Kamailio + `RTPengine`. VV-2
    /// wires the generator to mackesd's policy store so peer
    /// AORs (via `dispatcher.list`) + Vitelity sub-accounts (via
    /// `uacreg.list`) flow from approved `voice_mesh` /
    /// `voice_public` revisions.
    RenderConfig {
        /// Override the kamailio-mde output directory (defaults
        /// to `/etc/kamailio-mde/`). Used by tests + dry-runs.
        #[arg(long, value_name = "DIR", default_value = "/etc/kamailio-mde")]
        kamailio_dir: PathBuf,
        /// Override the rtpengine-mde output directory.
        #[arg(long, value_name = "DIR", default_value = "/etc/rtpengine-mde")]
        rtpengine_dir: PathBuf,
        /// VV-2 — JSON file containing a serialized `VoiceDesired`
        /// document. When the file is missing, render-config
        /// falls back to `VoiceDesired::boot_default(node_id)` and
        /// emits the minimal SIP-OPTIONS-keepalive-only config.
        /// The voice_config worker writes to this path on every
        /// policy change; operators can hand-edit during
        /// development by dropping a JSON document at the
        /// default path.
        #[arg(
            long,
            value_name = "PATH",
            default_value = "/var/lib/mackesd/voice-desired.json"
        )]
        desired_json: PathBuf,
        /// Skip the desired_json file entirely and use
        /// `boot_default` — useful for testing the bootstrap
        /// path in isolation.
        #[arg(long)]
        boot_default: bool,
        /// Print each generated file to stdout instead of
        /// writing to disk. Useful for diff'ing across policy
        /// revisions.
        #[arg(long)]
        dry_run: bool,
    },
}

/// NF-2.6 — `mackesd ca <sub>` subcommands.
#[derive(Subcommand)]
enum CaCmd {
    /// Idempotent CA mint at epoch 0. No-op when an active
    /// CA already exists for the named mesh.
    Mint {
        /// Mesh id (defaults to `mesh-<hostname>`).
        #[arg(long, value_name = "MESH_ID")]
        mesh_id: Option<String>,
    },

    /// Bump the CA epoch — retires the active CA, mints a
    /// fresh one at epoch+1, re-signs every active peer
    /// cert under the new epoch.
    Rotate {
        /// Mesh id (defaults to `mesh-<hostname>`).
        #[arg(long, value_name = "MESH_ID")]
        mesh_id: Option<String>,
        /// Cert lifetime in days for the re-signed peer
        /// certs (default 365).
        #[arg(long, default_value_t = 365)]
        cert_lifetime_days: u32,
    },

    /// Print one row per CA epoch — mesh_id, epoch,
    /// created_at, retired_at (or "active" when NULL).
    List,

    /// Print the public CA cert PEM to stdout. Used by
    /// peer-bootstrap flows that need the CA chain to
    /// validate inbound TLS.
    DumpCa {
        /// Mesh id (defaults to `mesh-<hostname>`).
        #[arg(long, value_name = "MESH_ID")]
        mesh_id: Option<String>,
    },

    /// NF-18.1 (v2.5) — export the CA + every peer cert into a
    /// passphrase-encrypted ASCII-armored bundle on stdout (or
    /// to `--output <path>`). Use for off-cluster disaster
    /// recovery — `import` reverses. Passphrase read from
    /// `MDE_BACKUP_PASSPHRASE` env var (operator must export
    /// before invoking) so it never lands in shell history.
    Export {
        /// Mesh id (defaults to `mesh-<hostname>`).
        #[arg(long, value_name = "MESH_ID")]
        mesh_id: Option<String>,
        /// Where to write the armored bundle. Default: stdout.
        #[arg(long, value_name = "PATH")]
        output: Option<PathBuf>,
        /// Sealed CA key path (defaults to
        /// `/var/lib/mackesd/nebula-ca/ca.key`).
        #[arg(long, value_name = "PATH")]
        ca_key: Option<PathBuf>,
    },

    /// NF-18.1 (v2.5) — import an exported bundle and restore
    /// the CA + peer certs into the local store. Reads the
    /// armored bundle from stdin (or `--input <path>`).
    /// Passphrase via `MDE_BACKUP_PASSPHRASE`.
    Import {
        /// Where to read the armored bundle from. Default:
        /// stdin.
        #[arg(long, value_name = "PATH")]
        input: Option<PathBuf>,
    },

    /// NF-3.6.b (v2.5) — sign a peer's pending-enroll CSR.
    /// Reads `QNM-Shared/<peer-id>/mackesd/pending-enroll.json`,
    /// signs the cert under the active CA, writes the
    /// `nebula-bundle.json` back so the peer's nebula_supervisor
    /// can materialize `/etc/nebula/`. Idempotent — re-running
    /// re-signs at the current epoch + allocates a fresh
    /// overlay IP.
    SignCsr {
        /// Peer's stable node-id (e.g. `peer:anvil`). Must match
        /// a pending-enroll.json under QNM-Shared.
        node_id: String,
        /// Override QNM-Shared root (defaults to
        /// `$QNM_SHARED_ROOT` or `~/QNM-Shared`).
        #[arg(long, env = "QNM_SHARED_ROOT")]
        qnm_root: Option<PathBuf>,
        /// Mesh id (defaults to `mesh-<hostname>`).
        #[arg(long, value_name = "MESH_ID")]
        mesh_id: Option<String>,
        /// CA cert path (defaults to `/etc/nebula/ca.crt`).
        #[arg(long, value_name = "PATH")]
        ca_crt: Option<PathBuf>,
        /// Sealed CA key path (defaults to
        /// `/var/lib/mackesd/nebula-ca/ca.key`).
        #[arg(long, value_name = "PATH")]
        ca_key: Option<PathBuf>,
        /// Scratch dir for intermediate peer cert/key files
        /// (defaults to `/var/lib/mackesd/nebula-ca/scratch`).
        #[arg(long, value_name = "PATH")]
        scratch_dir: Option<PathBuf>,
        /// Lighthouse public reachable address baked into the
        /// bundle's roster (form `host:port`). Defaults to
        /// `<hostname>:4242`; operators on multi-NIC or
        /// public-IP-different-from-hostname boxes should
        /// override.
        #[arg(long, value_name = "HOST:PORT")]
        lighthouse_addr: Option<String>,
        /// Cert lifetime in days (default 365).
        #[arg(long, default_value_t = 365)]
        cert_lifetime_days: u32,
        /// TUNE-11 — bypass the 8-peer cap (Q3 + Q22 lock). The
        /// override engages an audit-log entry. Document any
        /// real use in `docs/design/cap-overrides.md`.
        #[arg(long, default_value_t = false)]
        override_cap: bool,
    },
    /// INST-7 prerequisite (v2.7) — revoke a peer's Nebula cert.
    /// Marks every active row for `<node-id>` in `nebula_peer_certs`
    /// as revoked, adds the node-id to the local ban list (so the
    /// identity can't re-enroll even after a CA rotation), and fires
    /// a best-effort Bus event `ca/revoke/<node-id>`.
    ///
    /// This is the CLI replacement for the originally-planned
    /// `dev.mackes.MDE.Ca.Revoke` D-Bus method. D-Bus retires by 1.0
    /// per AI_GOVERNANCE §3.3; the wipe sequence in `mde-install`
    /// shells this command instead.
    ///
    /// Exits 0 on success (0 rows marked is still success — the ban
    /// list write happens regardless). Exits non-zero on DB or
    /// ban-list I/O failure.
    Revoke {
        /// Node-id to revoke (e.g. `peer:anvil`).
        node_id: String,
        /// Override QNM-Shared / mesh-home root (defaults to
        /// `$QNM_SHARED_ROOT` or `~/QNM-Shared`).
        #[clap(long, env = "QNM_SHARED_ROOT")]
        qnm_root: Option<std::path::PathBuf>,
        /// This peer's own node-id (used to locate the local
        /// ban-list file). Defaults to reading `/etc/mde/node-id`.
        #[clap(long)]
        self_node_id: Option<String>,
    },

    /// EPIC-SEC-BANLIST (Q53) — add a node-id to this peer's ban
    /// list. A banned node-id is refused enrollment mesh-wide, even
    /// with a valid passcode + across a CA rotation. GFS replication
    /// propagates the ban to every peer.
    Ban {
        /// Node-id to ban (e.g. `peer:stolen`).
        node_id: String,
        /// Override QNM-Shared / mesh-home root (defaults to
        /// `$QNM_SHARED_ROOT` or `~/QNM-Shared`).
        #[arg(long, env = "QNM_SHARED_ROOT")]
        qnm_root: Option<PathBuf>,
    },
    /// EPIC-SEC-BANLIST (Q53) — remove a node-id from this peer's
    /// ban list. Only lifts the entry THIS peer set; a ban another
    /// peer set must be lifted there (the gate checks the union).
    Unban {
        /// Node-id to unban.
        node_id: String,
        /// Override QNM-Shared / mesh-home root.
        #[arg(long, env = "QNM_SHARED_ROOT")]
        qnm_root: Option<PathBuf>,
    },
    /// EPIC-SEC-BANLIST (Q53) — print the union of every peer's ban
    /// list (the set the enrollment gate enforces).
    BanList {
        /// Override QNM-Shared / mesh-home root.
        #[arg(long, env = "QNM_SHARED_ROOT")]
        qnm_root: Option<PathBuf>,
    },
}

/// NF-18.x — `mackesd nebula <sub>` subcommands.
#[derive(Subcommand)]
enum NebulaCmd {
    /// NF-18.2 — emit a JSON array of every active peer cert
    /// (one row per active row in nebula_peer_certs, joined
    /// with the nodes table for the role field). Useful for
    /// off-cluster audit and as a human-readable backup record
    /// that complements the encrypted `ca export` bundle.
    ExportRoster,
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
            // First-class panel/CLI parity per 12.1.3. Today the
            // report is the empty baseline; subsequent substeps
            // (12.3.3 heartbeats, 12.5.1 drift detector) populate
            // the live fields.
            let report = mackesd_core::health::HealthReport::empty();
            println!("{}", report.to_json_line()?);
        }
        Cmd::Probe { action } => match action {
            ProbeCmd::Scan { targets, deep, source, nse_dir } => {
                use mackesd_core::probe_nmap::{scan, Profile};
                use mde_card::probe::HostSource;
                let src = match source.as_str() {
                    "lan" => HostSource::Lan,
                    "arbitrary" => HostSource::Arbitrary,
                    _ => HostSource::Mesh,
                };
                let profile = if deep { Profile::Deep } else { Profile::Fast };
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let cards = scan("nmap", profile, &targets, &[], &nse_dir, src, now);
                // One JSON line per host card (each carries its service
                // children). Empty output = no hosts found / nmap absent.
                for card in &cards {
                    println!("{}", serde_json::to_string(card)?);
                }
            }
            ProbeCmd::Refresh { qnm_root, node_id, nse_dir } => {
                // MESH-PROBE-4 manual refresh — one deep cycle that
                // writes probe-inventory.json + announces probe/changed.
                let qnm_root = qnm_root.unwrap_or_else(mackesd_core::default_qnm_shared_root);
                let node_id = node_id.unwrap_or_else(default_node_id);
                let home = std::env::var_os("HOME")
                    .map_or_else(|| PathBuf::from("/root"), PathBuf::from);
                let n = mackesd_core::probe_nmap::run_probe_cycle(
                    &qnm_root, &node_id, &home, "nmap", &nse_dir, true,
                );
                println!("probe refresh: {n} host(s) in inventory");
            }
            ProbeCmd::List { qnm_root, service } => {
                // MESH-PROBE-6 — read the merged mesh-wide inventory.
                let qnm_root = qnm_root.unwrap_or_else(mackesd_core::default_qnm_shared_root);
                match service {
                    Some(kind) => {
                        for hs in mackesd_core::probe_nmap::peers_with_service(&qnm_root, &kind) {
                            println!(
                                "{}\t{}\t{}:{}",
                                hs.host.ip, hs.service.service_kind, hs.host.hostname, hs.service.port
                            );
                        }
                    }
                    None => {
                        for card in &mackesd_core::probe_nmap::inventory(&qnm_root) {
                            println!("{}", serde_json::to_string(card)?);
                        }
                    }
                }
            }
        },
        Cmd::PresetLaunch { tag } => {
            // Portal-18.d (v6.0 R12, 2026-05-27) — preset launch-
            // bundle expansion. Loads the tag store, finds the
            // named preset, fires `swaymsg exec <cmd>` for each
            // entry in `launch_bundle`. Prints a one-line summary;
            // non-zero exit when any exec fails.
            let store = mackes_mesh_types::TagStore::load_default()
                .with_context(|| "loading tag store for preset-launch")?;
            let Some(tag_entry) = store.find_by_name(&tag) else {
                eprintln!("error: tag '{tag}' not found in tag store");
                std::process::exit(1);
            };
            let launch_bundle = match &tag_entry.flavor {
                mackes_mesh_types::TagFlavor::Preset { launch_bundle } => launch_bundle.clone(),
                other => {
                    eprintln!(
                        "error: tag '{tag}' is not a preset (flavor: {:?})",
                        other
                    );
                    std::process::exit(1);
                }
            };
            if launch_bundle.is_empty() {
                eprintln!("error: tag '{tag}' has an empty launch_bundle");
                std::process::exit(1);
            }
            let total = launch_bundle.len();
            let mut launched = 0usize;
            for cmd_str in &launch_bundle {
                let escaped = cmd_str.replace('\\', "\\\\").replace('"', "\\\"");
                let swayipc_cmd = format!("exec \"{escaped}\"");
                let status = std::process::Command::new("swaymsg")
                    .arg(&swayipc_cmd)
                    .status();
                match status {
                    Ok(s) if s.success() => launched += 1,
                    Ok(s) => {
                        eprintln!("warn: swaymsg exit {s} for '{cmd_str}'");
                    }
                    Err(e) => {
                        eprintln!("warn: swaymsg spawn failed for '{cmd_str}': {e}");
                    }
                }
            }
            println!("launched {launched}/{total} from preset '{tag}'");
            if launched != total {
                std::process::exit(1);
            }
        }
        Cmd::StateRestore { bundle, passphrase_env, recovery_dir } => {
            // MESHFS-14.1 — bundle decode + CA restore + LizardFS
            // metadata extraction. We deliberately do NOT replay
            // the LizardFS volume config automatically; operator
            // runs `mfsmaster --import-metadata` against the
            // extracted dump.
            let passphrase = std::env::var(&passphrase_env).with_context(|| {
                format!(
                    "passphrase env-var {passphrase_env} unset — \
                     export it before running state restore",
                )
            })?;
            let armored = std::fs::read_to_string(&bundle).with_context(|| {
                format!("reading bundle {}", bundle.display())
            })?;
            let sealed = mackesd_core::ca::backup::dearmor(&armored)
                .context("ASCII-armor decode")?;
            let plaintext = mackesd_core::ca::backup::unseal(&passphrase, &sealed)
                .context("AEAD unseal — wrong passphrase OR tampered bundle")?;

            let conn = mackesd_core::store::open(&db_path)
                .with_context(|| format!("opening store at {}", db_path.display()))?;
            mackesd_core::ca::backup::restore_to_store(&conn, &plaintext)
                .context("restoring CA + peer rows to store")?;
            eprintln!(
                "[state-restore] CA: {ca_n} cert(s) + {peer_n} peer cert(s) restored",
                ca_n = plaintext.ca_certs.len(),
                peer_n = plaintext.peer_certs.len(),
            );

            // MESHFS-14.1 — extract LizardFS snapshot if present.
            match plaintext.meshfs_snapshot.as_ref() {
                None => {
                    eprintln!(
                        "[state-restore] bundle has no meshfs snapshot (CA-only, pre-MESHFS, or Gluster-only bundle) — skipping LizardFS step",
                    );
                }
                Some(snap) => {
                    std::fs::create_dir_all(&recovery_dir).with_context(|| {
                        format!("mkdir {}", recovery_dir.display())
                    })?;
                    let mut wrote = 0usize;
                    // Write metadata dump for `mfsmaster --import-metadata`.
                    if let Some(dump) = snap.metadata_dump.as_deref() {
                        let path = recovery_dir.join("metadata.mfs.dump");
                        std::fs::write(&path, dump).with_context(|| {
                            format!("writing {}", path.display())
                        })?;
                        wrote += 1;
                        eprintln!(
                            "[state-restore] meshfs: wrote {} ({} bytes)",
                            path.display(), dump.len(),
                        );
                    }
                    // Write exports config for re-application.
                    if let Some(cfg) = snap.exports_config.as_deref() {
                        let path = recovery_dir.join("mfsexports.cfg");
                        std::fs::write(&path, cfg).with_context(|| {
                            format!("writing {}", path.display())
                        })?;
                        wrote += 1;
                        eprintln!(
                            "[state-restore] meshfs: wrote {} ({} bytes)",
                            path.display(), cfg.len(),
                        );
                    }
                    // Write CS list for topology reference.
                    if let Some(cs) = snap.cs_list.as_deref() {
                        let path = recovery_dir.join("cs-list.txt");
                        std::fs::write(&path, cs).with_context(|| {
                            format!("writing {}", path.display())
                        })?;
                        wrote += 1;
                        eprintln!(
                            "[state-restore] meshfs: wrote {} ({} bytes)",
                            path.display(), cs.len(),
                        );
                    }
                    if wrote == 0 {
                        eprintln!(
                            "[state-restore] meshfs snapshot present but every section was empty — nothing to apply",
                        );
                    } else {
                        let goal_hint = snap.goal.map_or_else(
                            || "N (re-count enrolled peers)".to_owned(),
                            |g| g.to_string(),
                        );
                        eprintln!(
                            "[state-restore] meshfs: {wrote} file(s) at {dir}; restore steps:\n\
                             1. cp {dir}/mfsexports.cfg /etc/mfs/mfsexports.cfg\n\
                             2. mfsmaster --import-metadata {dir}/metadata.mfs.dump\n\
                             3. mfsmaster start  # starts the active master\n\
                             4. mfssetgoal -r {goal_hint} /mnt/mesh-storage\n\
                             (see docs/help/mesh-recovery.md)",
                            dir = recovery_dir.display(),
                        );
                    }
                }
            }
        }
        Cmd::PreflightMeshFsHeadroom { data_dir, home } => {
            // MESHFS-1.2 — pre-flight headroom check for the
            // LizardFS mesh-storage rollout. Mirrors the gluster
            // headroom CLI; operator runs this before upgrading
            // to v5.0.0 or before mesh-storage bootstrap.
            let home_dir = home.clone().or_else(|| {
                std::env::var_os("HOME").map(std::path::PathBuf::from)
            }).context("no HOME env var; pass --home <dir>")?;
            let xdg = mackesd_core::meshfs::headroom::default_xdg_dirs(&home_dir);
            let report = mackesd_core::meshfs::headroom::check(&data_dir, &xdg);
            eprintln!("{}", report.summary());
            println!("{}", serde_json::to_string(&report).context("encode report")?);
            match report.verdict {
                mackesd_core::meshfs::headroom::Verdict::Ok => {}
                mackesd_core::meshfs::headroom::Verdict::Warn
                | mackesd_core::meshfs::headroom::Verdict::NoDataDir => std::process::exit(1),
            }
        }
        Cmd::MeshFsStatus { vip, admin_binary } => {
            let report =
                mackesd_core::workers::meshfs_worker::meshfs_status_report(&admin_binary, &vip);
            let reachable = report.master_reachable;
            println!(
                "{}",
                serde_json::to_string(&report).context("encode meshfs status")?
            );
            if !reachable {
                std::process::exit(1);
            }
        }
        Cmd::MeshFsTrashList { mount_path } => {
            let entries =
                mackesd_core::workers::meshfs_worker::list_trash_entries(&mount_path);
            println!(
                "{}",
                serde_json::to_string(&entries).context("encode trash list")?
            );
        }
        Cmd::MeshFsUndelete { vip, path, admin_binary } => {
            let argv = vec![
                admin_binary.clone(),
                vip.clone(),
                "TRASH-RECOVER".to_owned(),
                path.clone(),
            ];
            let ok = std::process::Command::new(&argv[0])
                .args(&argv[1..])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if !ok {
                anyhow::bail!("TRASH-RECOVER failed for {path}");
            }
        }
        Cmd::MeshFsResolveConflict { path } => {
            mackesd_core::workers::meshfs_worker::resolve_conflict_to_archive(&path)
                .map_err(|e| anyhow::anyhow!("resolve-conflict failed: {e}"))?;
        }
        Cmd::GeneratePasscode { store, cred_path } => {
            let code = mackesd_core::passcode::generate();
            println!("{code}");
            if store {
                let path = cred_path
                    .unwrap_or_else(mackesd_core::passcode_creds::default_cred_path);
                mackesd_core::passcode_creds::store(
                    &code,
                    &path,
                    mackesd_core::passcode_creds::CRED_NAME,
                )
                .map_err(|e| anyhow::anyhow!("generate-passcode --store: {e}"))?;
                eprintln!(
                    "stored (encrypted via systemd-creds) at {}. Share the code \
                     above with peers; the plaintext is not on disk.",
                    path.display()
                );
            } else {
                eprintln!(
                    "(encrypt at rest with: mackesd generate-passcode --store, \
                     or save to libsecret manually)"
                );
            }
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
        Cmd::RotatePasscode { store, cred_path } => {
            // Phase 12.10.2 — generate fresh passcode; peer
            // redistribution wires through the reconcile loop (12.5).
            let code = mackesd_core::passcode::generate();
            println!("{code}");
            if store {
                let path = cred_path
                    .unwrap_or_else(mackesd_core::passcode_creds::default_cred_path);
                mackesd_core::passcode_creds::store(
                    &code,
                    &path,
                    mackesd_core::passcode_creds::CRED_NAME,
                )
                .map_err(|e| anyhow::anyhow!("rotate-passcode --store: {e}"))?;
                eprintln!(
                    "rotation: stored (encrypted via systemd-creds) at {}; \
                     peers refresh their bearer tokens on next heartbeat.",
                    path.display()
                );
            } else {
                eprintln!(
                    "rotation: encrypt at rest with `mackesd rotate-passcode \
                     --store`; peers refresh their bearer tokens on next \
                     heartbeat."
                );
            }
        }
        Cmd::ShowPasscode { cred_path } => {
            // EPIC-SEC-PASSCODE-CREDS — decrypt + print the stored
            // passcode. The inverse of generate/rotate --store.
            let path = cred_path
                .unwrap_or_else(mackesd_core::passcode_creds::default_cred_path);
            let code = mackesd_core::passcode_creds::load(
                &path,
                mackesd_core::passcode_creds::CRED_NAME,
            )
            .map_err(|e| anyhow::anyhow!("show-passcode: {e}"))?;
            println!("{code}");
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
        Cmd::Enroll {
            passcode,
            token,
            name,
            qnm_root,
        } => {
            let display = name.unwrap_or_else(|| {
                std::env::var("HOSTNAME").unwrap_or_else(|_| {
                    std::process::Command::new("hostname")
                        .output()
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok())
                        .map_or_else(|| "unknown".to_owned(), |s| s.trim().to_owned())
                })
            });
            match (passcode, token) {
                (Some(_), Some(_)) => {
                    // `conflicts_with` should catch this at parse
                    // time, but belt-and-braces.
                    eprintln!(
                        "mackesd enroll: --passcode and --token are mutually \
                         exclusive; pass exactly one."
                    );
                    std::process::exit(2);
                }
                (None, None) => {
                    eprintln!(
                        "mackesd enroll: pass either --passcode (v1.x flow) or \
                         --token (v2.5 Nebula flow)."
                    );
                    std::process::exit(2);
                }
                (Some(pc), None) => {
                    // Phase 12.3.1 — v1.x build identity + signed request.
                    let identity = mackesd_core::enrollment::build_identity();
                    match mackesd_core::enrollment::build_request(&identity, &pc, &display) {
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
                (None, Some(tok)) => {
                    // NF-3.6.a — v2.5 Nebula join-token flow.
                    let qnm_root = qnm_root
                        .unwrap_or_else(mackesd_core::default_qnm_shared_root);
                    let node_id = default_node_id();
                    eprintln!(
                        "mackesd enroll: publishing CSR + waiting up to {} s \
                         for the lighthouse to sign…",
                        mackesd_core::nebula_enroll::ENROLL_WAIT_TIMEOUT.as_secs(),
                    );
                    match mackesd_core::nebula_enroll::enroll_with_token(
                        &qnm_root, &node_id, &display, &tok,
                    ) {
                        Ok(outcome) => {
                            println!(
                                "enrolled into mesh '{}' as {} (overlay {}) after {} s.",
                                outcome.mesh_id,
                                node_id,
                                outcome.overlay_ip,
                                outcome.waited.as_secs(),
                            );
                            eprintln!(
                                "nebula_supervisor will materialize /etc/nebula/ \
                                 from the bundle on its next reconcile tick."
                            );
                        }
                        Err(e) => {
                            eprintln!("mackesd enroll: {e}");
                            std::process::exit(2);
                        }
                    }
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
        Cmd::Ca { sub } => {
            // NF-2.6 (v2.5) — mackesd ca {mint, rotate, list,
            // dump-ca} subcommands. Operator surface backing the
            // CA module.
            let mut conn = mackesd_core::store::open(&db_path)?;
            let default_mesh = format!("mesh-{}", default_node_id());
            match sub {
                CaCmd::Mint { mesh_id } => {
                    let mesh = mesh_id.unwrap_or(default_mesh);
                    match mackesd_core::ca::mint::mint_ca(
                        &mackesd_core::ca::SubprocessBackend,
                        &conn,
                        &mesh,
                        None,
                        None,
                    ) {
                        Ok(mackesd_core::ca::mint::MintOutcome::Created { .. }) => {
                            println!("CA minted at epoch 0 for mesh '{mesh}'.");
                        }
                        Ok(mackesd_core::ca::mint::MintOutcome::AlreadyMinted {
                            epoch,
                            ..
                        }) => {
                            println!(
                                "CA for mesh '{mesh}' already exists at epoch {epoch} (no-op)."
                            );
                        }
                        Err(mackesd_core::ca::CaError::BinaryMissing) => {
                            return Err(anyhow::anyhow!(
                                "nebula-cert not on PATH. Install the Fedora `nebula` package + retry."
                            ));
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!("mint: {e}"));
                        }
                    }
                }
                CaCmd::Rotate {
                    mesh_id,
                    cert_lifetime_days,
                } => {
                    let mesh = mesh_id.unwrap_or(default_mesh);
                    match mackesd_core::ca::epoch::bump_epoch(
                        &mackesd_core::ca::SubprocessBackend,
                        &mut conn,
                        &mesh,
                        None,
                        None,
                        cert_lifetime_days,
                    ) {
                        Ok(o) => {
                            println!(
                                "CA rotated for mesh '{mesh}': epoch {} → {} ({} peer certs re-signed).",
                                o.retired_epoch
                                    .map(|e| e.to_string())
                                    .unwrap_or_else(|| "none".into()),
                                o.new_epoch,
                                o.re_signed,
                            );
                        }
                        Err(mackesd_core::ca::CaError::BinaryMissing) => {
                            return Err(anyhow::anyhow!(
                                "nebula-cert not on PATH. Install the Fedora `nebula` package + retry."
                            ));
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!("rotate: {e}"));
                        }
                    }
                }
                CaCmd::List => {
                    let mut stmt = conn.prepare(
                        "SELECT mesh_id, epoch, created_at, retired_at \
                         FROM nebula_ca ORDER BY mesh_id, epoch DESC",
                    )?;
                    let rows = stmt.query_map([], |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, i64>(1)?,
                            r.get::<_, i64>(2)?,
                            r.get::<_, Option<i64>>(3)?,
                        ))
                    })?;
                    println!(
                        "{:<24} {:>6} {:>12} {:>12}",
                        "MESH_ID", "EPOCH", "CREATED", "RETIRED"
                    );
                    let mut count = 0;
                    for row in rows {
                        let (mesh, epoch, created, retired) = row?;
                        let retired_disp = match retired {
                            Some(t) => t.to_string(),
                            None => "active".to_string(),
                        };
                        println!(
                            "{mesh:<24} {epoch:>6} {created:>12} {retired_disp:>12}",
                        );
                        count += 1;
                    }
                    if count == 0 {
                        println!("(no CAs minted yet — run `mackesd ca mint`)");
                    }
                }
                CaCmd::DumpCa { mesh_id } => {
                    let mesh = mesh_id.unwrap_or(default_mesh);
                    match mackesd_core::ca::mint::current_ca(&conn, &mesh) {
                        Ok(Some((_epoch, pem))) => {
                            print!("{pem}");
                        }
                        Ok(None) => {
                            return Err(anyhow::anyhow!(
                                "no active CA for mesh '{mesh}'"
                            ));
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!("dump-ca: {e}"));
                        }
                    }
                }
                CaCmd::Export { mesh_id, output, ca_key } => {
                    // NF-18.1 — encrypted CA backup. Passphrase
                    // via env var so it doesn't land in shell
                    // history. CA key path defaults to the
                    // SignCsrPaths production value.
                    let mesh = mesh_id.unwrap_or(default_mesh);
                    let passphrase = std::env::var("MDE_BACKUP_PASSPHRASE")
                        .map_err(|_| anyhow::anyhow!(
                            "export: set MDE_BACKUP_PASSPHRASE before invoking \
                             (the bundle is encrypted with this passphrase)"
                        ))?;
                    let key_path = ca_key.unwrap_or_else(|| {
                        mackesd_core::nebula_enroll::SignCsrPaths::production_defaults().ca_key
                    });
                    let ca_key_pem = mackesd_core::ca::seal::read_sealed(&key_path)
                        .map_err(|e| anyhow::anyhow!(
                            "export: read CA key {}: {e}", key_path.display(),
                        ))?;
                    let ca_key_pem_str = String::from_utf8(ca_key_pem)
                        .map_err(|e| anyhow::anyhow!("export: CA key not UTF-8: {e}"))?;
                    let plaintext = mackesd_core::ca::backup::assemble_from_store(
                        &conn, &mesh, &ca_key_pem_str,
                    ).map_err(|e| anyhow::anyhow!("export: assemble: {e}"))?;
                    let sealed = mackesd_core::ca::backup::seal(&passphrase, &plaintext)
                        .map_err(|e| anyhow::anyhow!("export: seal: {e}"))?;
                    let armored = mackesd_core::ca::backup::armor(&sealed, plaintext.exported_at);
                    match output {
                        Some(path) => {
                            std::fs::write(&path, &armored).with_context(|| {
                                format!("write {}", path.display())
                            })?;
                            eprintln!(
                                "exported {} CA rows + {} peer certs → {} ({} bytes armored)",
                                plaintext.ca_certs.len(),
                                plaintext.peer_certs.len(),
                                path.display(),
                                armored.len(),
                            );
                        }
                        None => {
                            print!("{armored}");
                        }
                    }
                }
                CaCmd::Import { input } => {
                    // NF-18.1 — encrypted CA bundle restore.
                    let passphrase = std::env::var("MDE_BACKUP_PASSPHRASE")
                        .map_err(|_| anyhow::anyhow!(
                            "import: set MDE_BACKUP_PASSPHRASE before invoking",
                        ))?;
                    let armored = match input {
                        Some(path) => std::fs::read_to_string(&path).with_context(|| {
                            format!("read {}", path.display())
                        })?,
                        None => {
                            use std::io::Read;
                            let mut s = String::new();
                            std::io::stdin().read_to_string(&mut s)?;
                            s
                        }
                    };
                    let sealed = mackesd_core::ca::backup::dearmor(&armored)
                        .map_err(|e| anyhow::anyhow!("import: dearmor: {e}"))?;
                    let plaintext = mackesd_core::ca::backup::unseal(&passphrase, &sealed)
                        .map_err(|e| anyhow::anyhow!("import: {e}"))?;
                    mackesd_core::ca::backup::restore_to_store(&conn, &plaintext)
                        .map_err(|e| anyhow::anyhow!("import: restore: {e}"))?;
                    eprintln!(
                        "imported {} CA rows + {} peer certs for mesh '{}' \
                         (exported_at = unix:{}); restart mackesd to pick up \
                         the new CA + the operator should re-write \
                         /etc/nebula/{{ca.crt,ca.key}} from the bundle.",
                        plaintext.ca_certs.len(),
                        plaintext.peer_certs.len(),
                        plaintext.mesh_id,
                        plaintext.exported_at,
                    );
                }
                CaCmd::SignCsr {
                    node_id,
                    qnm_root,
                    mesh_id,
                    ca_crt,
                    ca_key,
                    scratch_dir,
                    lighthouse_addr,
                    cert_lifetime_days,
                    override_cap,
                } => {
                    // NF-3.6.b — sign the peer's pending-enroll
                    // CSR + write the bundle back to QNM-Shared.
                    let qnm_root = qnm_root
                        .unwrap_or_else(mackesd_core::default_qnm_shared_root);
                    let mesh = mesh_id.unwrap_or(default_mesh);
                    let mut paths =
                        mackesd_core::nebula_enroll::SignCsrPaths::production_defaults();
                    if let Some(p) = ca_crt {
                        paths.ca_crt = p;
                    }
                    if let Some(p) = ca_key {
                        paths.ca_key = p;
                    }
                    if let Some(p) = scratch_dir {
                        paths.scratch_dir = p;
                    }
                    let lh_addr = lighthouse_addr.unwrap_or_else(|| {
                        let host = std::fs::read_to_string("/etc/hostname")
                            .ok()
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| default_node_id());
                        format!("{host}:4242")
                    });
                    // Self-roster: the lighthouse running this
                    // CLI is the only entry. Multi-lighthouse
                    // setups can re-sign with a different roster
                    // via a future --lighthouse flag set.
                    let local_id = default_node_id();
                    let lighthouses = vec![mackesd_core::ca::bundle::LighthouseEntry {
                        node_id: local_id.clone(),
                        // Best-choice: until the lighthouse knows
                        // its own overlay IP (it gets one only
                        // after it self-enrolls), advertise the
                        // conventional first-host address. Operator
                        // can override by re-signing post-mint or
                        // by editing the bundle directly.
                        overlay_ip: "10.42.0.1".to_string(),
                        external_addr: lh_addr,
                    }];
                    match mackesd_core::nebula_enroll::sign_pending_csr(
                        &mackesd_core::ca::SubprocessBackend,
                        &conn,
                        &qnm_root,
                        &node_id,
                        &mesh,
                        &paths,
                        lighthouses,
                        cert_lifetime_days,
                        override_cap,
                    ) {
                        Ok(outcome) => {
                            if override_cap {
                                eprintln!(
                                    "TUNE-11 OVERRIDE ENGAGED: signed {} past the {}-peer cap. \
                                     Audit-log entry written to the journal under \
                                     `mackesd::cap_override`. Document the exception in \
                                     docs/design/cap-overrides.md.",
                                    outcome.peer_id,
                                    mackesd_core::ca::sign::MAX_PEER_CAP,
                                );
                            }
                            println!(
                                "signed {} into mesh '{}' at epoch {} (overlay {}); bundle at {}.",
                                outcome.peer_id,
                                mesh,
                                outcome.epoch,
                                outcome.overlay_ip,
                                outcome.bundle_path.display(),
                            );
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!("sign-csr: {e}"));
                        }
                    }
                }
                CaCmd::Revoke {
                    node_id,
                    qnm_root,
                    self_node_id,
                } => {
                    // INST-7 prerequisite — revoke a peer's cert +
                    // ban the identity. CLI surface replaces the
                    // originally-planned D-Bus method (D-Bus retires
                    // by 1.0 per AI_GOVERNANCE §3.3).
                    let qnm_root =
                        qnm_root.unwrap_or_else(mackesd_core::default_qnm_shared_root);
                    let self_id =
                        self_node_id.unwrap_or_else(default_node_id);
                    let rows = mackesd_core::ca::revoke::revoke_peer(
                        &conn, &qnm_root, &self_id, &node_id,
                    )
                    .context("ca revoke")?;
                    println!(
                        "revoked '{node_id}': {rows} cert row(s) marked revoked; \
                         added to ban list at {self_id}'s QNM-Shared entry."
                    );
                }
                CaCmd::Ban { node_id, qnm_root } => {
                    // EPIC-SEC-BANLIST (Q53) — add node-id to this
                    // peer's ban list. GFS replication propagates it.
                    let qnm_root =
                        qnm_root.unwrap_or_else(mackesd_core::default_qnm_shared_root);
                    let self_id = default_node_id();
                    match mackesd_core::ca::ban_list::add_banned(
                        &qnm_root, &self_id, &node_id,
                    ) {
                        Ok(true) => println!(
                            "banned '{node_id}' (recorded in {}'s ban list; \
                             propagates to every peer via GFS).",
                            self_id
                        ),
                        Ok(false) => println!("'{node_id}' was already banned (no-op)."),
                        Err(e) => return Err(anyhow::anyhow!("ca ban: {e}")),
                    }
                }
                CaCmd::Unban { node_id, qnm_root } => {
                    // EPIC-SEC-BANLIST (Q53) — lift a ban THIS peer
                    // set. Bans set on other peers must be lifted
                    // there (the gate enforces the union).
                    let qnm_root =
                        qnm_root.unwrap_or_else(mackesd_core::default_qnm_shared_root);
                    let self_id = default_node_id();
                    match mackesd_core::ca::ban_list::remove_banned(
                        &qnm_root, &self_id, &node_id,
                    ) {
                        Ok(true) => println!("unbanned '{node_id}' from {self_id}'s ban list."),
                        Ok(false) => {
                            // Still surface the union state so the
                            // operator knows if another peer banned it.
                            if mackesd_core::ca::ban_list::is_banned(&qnm_root, &node_id) {
                                println!(
                                    "'{node_id}' isn't in {self_id}'s ban list, but ANOTHER \
                                     peer still bans it — unban it on that peer too."
                                );
                            } else {
                                println!("'{node_id}' isn't banned (no-op).");
                            }
                        }
                        Err(e) => return Err(anyhow::anyhow!("ca unban: {e}")),
                    }
                }
                CaCmd::BanList { qnm_root } => {
                    // EPIC-SEC-BANLIST (Q53) — print the enforced
                    // union across every peer's ban list.
                    let qnm_root =
                        qnm_root.unwrap_or_else(mackesd_core::default_qnm_shared_root);
                    let union = mackesd_core::ca::ban_list::load_union(&qnm_root);
                    if union.is_empty() {
                        println!("ban list empty (no node-ids banned across the mesh).");
                    } else {
                        println!("Banned node-ids (mesh-wide union, {} total):", union.len());
                        for id in &union {
                            println!("  {id}");
                        }
                    }
                }
            }
        }
        Cmd::Nebula { sub } => {
            // NF-18.x — mackesd nebula <sub> operator surface.
            let conn = mackesd_core::store::open(&db_path)?;
            match sub {
                NebulaCmd::ExportRoster => {
                    // NF-18.2 — JSON array of (node_id, name,
                    // overlay_ip, cert_pem, epoch, created_at,
                    // expires_at, groups). `groups` is sourced
                    // from nodes.role since the Nebula cert
                    // groups are encoded in the cert PEM body
                    // and we want a flat queryable shape.
                    let rows = mackesd_core::nebula_roster::export_roster(&conn)
                        .map_err(|e| anyhow::anyhow!("export-roster: {e}"))?;
                    println!("{}", serde_json::to_string_pretty(&rows)?);
                }
            }
        }
        Cmd::Voice { sub } => {
            // VV-1 / VV-1.5 / VV-2 (v4.1.0) — voice stack operator
            // surface. `render-config` is invoked by both
            // `kamailio-mde.service` and `rtpengine-mde.service` as
            // their ExecStartPre hook; the voice_config worker
            // writes the JSON input file when policy changes and
            // triggers `systemctl reload` to re-run this command.
            match sub {
                VoiceCmd::RenderConfig {
                    kamailio_dir,
                    rtpengine_dir,
                    desired_json,
                    boot_default,
                    dry_run,
                } => {
                    let desired = load_voice_desired(
                        &desired_json,
                        boot_default,
                        &default_node_id(),
                    )?;
                    let set = mde_voice_config::generate(&desired);
                    let kamailio_files = [
                        ("kamailio.cfg", &set.kamailio_cfg),
                        ("dispatcher.list", &set.dispatcher_list),
                        ("uacreg.list", &set.uacreg_list),
                    ];
                    let rtpengine_files = [("rtpengine.conf", &set.rtpengine_conf)];
                    if dry_run {
                        for (name, body) in kamailio_files {
                            println!("# ---- {} (would write under {}) ----", name, kamailio_dir.display());
                            print!("{body}");
                        }
                        for (name, body) in rtpengine_files {
                            println!("# ---- {} (would write under {}) ----", name, rtpengine_dir.display());
                            print!("{body}");
                        }
                    } else {
                        write_voice_config_files(&kamailio_dir, &kamailio_files)?;
                        write_voice_config_files(&rtpengine_dir, &rtpengine_files)?;
                        println!(
                            "voice render-config: wrote {} files under {} + {} under {}",
                            kamailio_files.len(),
                            kamailio_dir.display(),
                            rtpengine_files.len(),
                            rtpengine_dir.display(),
                        );
                    }
                }
            }
        }
        Cmd::WakePeer { mac, broadcast, via_lighthouse, port } => {
            // DEAD-2.5 + NF-21.2 — wire mackesd_core::workers::wol so
            // the Rust port has a runtime entry point. Replaces the
            // retired Python `mesh_wol.wake_peer` for the MAC-already-
            // known case; hostname resolution is the operator's job
            // until a PeerStore lookup helper lands. `--via-lighthouse`
            // routes through a lighthouse's overlay IP for WoL-across-
            // LANs (NF-21.2).
            let Some(mac_bytes) = mackesd_core::workers::wol::normalize_mac(&mac) else {
                anyhow::bail!("wake-peer: could not parse MAC {mac:?}");
            };
            if let Some(lighthouse_ip) = via_lighthouse.as_deref() {
                mackesd_core::workers::wol::wake_via_lighthouse(
                    mac_bytes, lighthouse_ip, port,
                )
                .context("wake-peer: send magic packet via lighthouse")?;
                println!(
                    "wake-peer: sent magic packet for {mac} via lighthouse \
                     {lighthouse_ip}:{port}"
                );
            } else {
                mackesd_core::workers::wol::wake(mac_bytes, &broadcast, port)
                    .context("wake-peer: send magic packet")?;
                println!("wake-peer: sent magic packet to {mac} via {broadcast}:{port}");
            }
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
/// v3.0.3 — wires the 5 Phase B workers (clipboard, mdns, fs_sync,
/// heartbeat, mesh_router; notification_relay retired in BUS-4.2)
/// into the
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
        clipboard::ClipboardWorker, firewall_preset::FirewallPresetWorker,
        fs_sync::FsSyncWorker, heartbeat::HeartbeatWorker,
        mdns::MdnsWorker, mesh_router::MeshRouterWorker,
        sshd_overlay_bind::SshdOverlayBindWorker, voice_config::VoiceConfigWorker,
        RestartPolicy, Spawn, Supervisor,
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

        // HYP-8.5 — load operator tag manifests on startup +
        // publish one Bus event per loaded tag. Fail-open: missing
        // dir → 0 tags loaded → no events. Per-file parse failures
        // log + skip in `load_tag_manifests`, the daemon never
        // crashes on a malformed manifest.
        if let Some(tags_dir) = mackesd_core::config::default_manifests_dir() {
            match mackesd_core::config::load_tag_manifests(&tags_dir) {
                Ok(manifests) => {
                    tracing::info!(
                        path = %tags_dir.display(),
                        count = manifests.len(),
                        "tag_manifest: loaded operator manifests",
                    );
                    for m in &manifests {
                        // Best-effort Bus publish — broker may not be
                        // up yet during the early startup phase, but
                        // the spawn-detached shell-out makes that a
                        // silent no-op rather than a daemon crash.
                        let topic = "event/config/tags/loaded".to_string();
                        let body = format!(
                            r#"{{"name":"{}","apps":{},"layout":"{}","autostart":{}}}"#,
                            m.name.replace('"', "\\\""),
                            m.apps.len(),
                            m.layout.replace('"', "\\\""),
                            m.autostart,
                        );
                        let _ = std::process::Command::new("mde-bus")
                            .arg("publish")
                            .arg(&topic)
                            .arg("--body-flag")
                            .arg(&body)
                            .spawn();
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        path = %tags_dir.display(),
                        error = %e,
                        "tag_manifest: directory load failed (expected on first boot)",
                    );
                }
            }
        }

        // v3.0.3 — async supervisor for Phase B workers. The
        // legacy reconcile worker stays on its own std::thread
        // because its sync rusqlite calls would block the tokio
        // scheduler if hosted here; both supervisors coexist.
        let mut sup = Supervisor::new();
        // v4.1 — track spawned worker names so Shell.Workers can
        // surface them via D-Bus. Strings get pushed alongside
        // each sup.spawn(); skipped workers (sqlite open failure)
        // don't get added so the report matches reality. The
        // Mutex<Vec<String>> is shared with ShellService so
        // post-registration spawns (KDC + reconcile, which come
        // after IPC registration) still appear in the roster.
        let worker_names: Arc<std::sync::Mutex<Vec<String>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        // OV-7.a/c (v2.6) — shared signal-sender slot for every
        // worker that emits dev.mackes.MDE.Nebula.Status.*
        // signals. Workers receive a clone before they spawn;
        // the IPC bootstrap fills the slot once the Nebula
        // status surface is registered. Empty slot → silent
        // emission; SQL + bundle writes still land.
        let nebula_signal_slot = mackesd_core::ipc::nebula::new_signal_sender_slot();
        sup.spawn(Spawn::new(
            ClipboardWorker::new(),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("clipboard".into());
        sup.spawn(Spawn::new(MdnsWorker::new(), RestartPolicy::OnFailure));
        worker_names.lock().expect("worker_names mutex").push("mdns".into());
        sup.spawn(Spawn::new(FsSyncWorker::new(), RestartPolicy::OnFailure));
        worker_names.lock().expect("worker_names mutex").push("fs_sync".into());
        sup.spawn(Spawn::new(
            HeartbeatWorker::new(qnm_root.clone(), node_id.clone()),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("heartbeat".into());
        // OV-7.a (v2.6) — health reconciler. Polls each known
        // peer's QNM-Shared heartbeat.json every 5 s, applies the
        // telemetry::health_state_from_age threshold table, writes
        // back into nodes.health, and fires PeerStateChanged on
        // transitions. Closes the gap between live heartbeats and
        // the SQLite column that NebulaStatusService::build_peer_list
        // projects. Spawn order: after HeartbeatWorker so peers
        // have at least one observable heartbeat by the first
        // reconcile tick.
        sup.spawn(Spawn::new(
            mackesd_core::workers::health_reconciler::HealthReconcilerWorker::new(
                qnm_root.clone(),
                db_path.clone(),
                node_id.clone(),
                std::sync::Arc::clone(&nebula_signal_slot),
            ),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("health_reconciler".into());
        // VV-2 (v4.1.0) — voice_config worker. Seeds the
        // /var/lib/mackesd/voice-desired.json document on first
        // tick + triggers `systemctl try-reload-or-restart` on
        // kamailio-mde + rtpengine-mde when the file changes.
        // try-reload-or-restart is a no-op while the units are
        // disabled (v4.1.0 ships them disabled per the spec
        // %post comment until VV-4 + VV-14 are green), so the
        // worker is harmless to run on a fresh peer.
        sup.spawn(Spawn::new(
            VoiceConfigWorker::new(node_id.clone()),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("voice_config".into());
        // NF-21.1 — sshd overlay-bind worker. Polls
        // /var/lib/mackesd/nebula/overlay-ip every 5 s; on change,
        // writes the /etc/ssh/sshd_config.d/mackes-mesh.conf drop-in
        // + reloads sshd so the daemon binds to the new overlay
        // address. Quiet no-op on pre-enrollment peers (missing
        // publish file). Replaces mesh_nebula.py::write_sshd_overlay_bind
        // so the Python module can fully retire (DEAD-2.14 plan).
        sup.spawn(Spawn::new(
            SshdOverlayBindWorker::new(),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("sshd_overlay_bind".into());
        // NF-21.3 — firewall_preset worker. Applies the Nebula
        // firewalld preset (UDP/4242 inbound on all peers; TCP/443
        // inbound additionally on lighthouses) on first tick + on
        // every role-flip via the /var/lib/mackesd/nebula/role.host
        // marker. Idempotent — firewall-cmd's ALREADY_ENABLED is
        // treated as success. Replaces mesh_nebula.py::apply_nebula_firewall_preset
        // so the Python helper can retire (DEAD-2.14 plan).
        sup.spawn(Spawn::new(
            FirewallPresetWorker::new(),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("firewall_preset".into());
        // mesh_router bootstraps with the per-transport
        // registry. Phase 12.18 D.2 (2026-05-23) — the NebulaHttps443
        // transport is registered at startup so the per-peer
        // HttpsFallbackState::Active transition can actually
        // route through a real TLS tunnel. The transport
        // gracefully reports `Misconfigured(no_fallback_host)`
        // until MDE_HTTPS_FALLBACK_HOST is set, so daemons
        // running without the env var still boot clean.
        let router_state: mackesd_core::workers::mesh_router::RouterState =
            Arc::new(RwLock::new(HashMap::new()));
        let https443: Arc<dyn mackes_transport::Transport> =
            Arc::new(mackesd_core::transport::https443::NebulaHttps443Transport::new());
        let router_registry: mackesd_core::workers::mesh_router::TransportRegistry =
            Arc::new(vec![https443]);
        sup.spawn(Spawn::new(
            MeshRouterWorker::new(Arc::clone(&router_state), router_registry),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("mesh_router".into());
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
        worker_names.lock().expect("worker_names mutex").push("stun_gather".into());
        // BUS-4.2 (2026-05-26) — `notification_relay` retired.
        // Cross-peer notification routing is now a side-effect of
        // BUS-4.4's FDO bridge: every Notify call publishes to
        // `fdo/<app>` on the Mackes Bus, and every peer subscribes
        // to `fdo/#` via the standard Bus subscription. The
        // legacy `~/QNM-Shared/<peer>/.qnm-notifications/` JSON
        // file convention is replaced by `<bus_root>/<topic>/
        // <ulid>.json` (BUS-1.4 file tree on GFS).

        // v2.5 NF-3.4 (2026-05-23) — Nebula supervisor.
        // Watches the leader-election state + the QNM-Shared
        // nebula-bundle.json mtime; on leader-promotion mints
        // the CA, writes the role.host marker, starts the
        // lighthouse + tunnel units. On bundle change, re-
        // materializes the on-disk Nebula config + reloads.
        match mackesd_core::store::open(&db_path) {
            Ok(conn) => {
                let sup_store = Arc::new(tokio::sync::Mutex::new(conn));
                // Bundle path mirrors the existing heartbeat
                // convention: QNM-Shared/<self>/mackesd/...
                let bundle_path = qnm_root
                    .join(&node_id)
                    .join("mackesd")
                    .join(mackesd_core::ca::bundle::BUNDLE_FILENAME);
                // mesh_id defaults to the configured node-id
                // namespace when the wizard hasn't named a
                // mesh yet. NF-7.x's wizard will overwrite the
                // record once the operator types a name.
                let mesh_id = std::env::var("MDE_MESH_ID")
                    .unwrap_or_else(|_| format!("mesh-{node_id}"));
                sup.spawn(Spawn::new(
                    mackesd_core::workers::nebula_supervisor::NebulaSupervisor::new(
                        sup_store,
                        node_id.clone(),
                        mesh_id,
                        bundle_path,
                    ),
                    RestartPolicy::OnFailure,
                ));
                worker_names.lock().expect("worker_names mutex").push("nebula_supervisor".into());
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    db_path = %db_path.display(),
                    "nebula-supervisor: sqlite open failed; worker skipped"
                );
            }
        }

        // NF-3.6.c (v2.5) — auto-signer worker. Polls QNM-Shared
        // for pending-enroll CSRs every 30 s + auto-signs each
        // one via nebula_enroll::sign_pending_csr. Runs on every
        // node — on peer-role boxes (no active CA), sign_pending_csr
        // returns SignFailed and the worker logs at debug + moves
        // on. On lighthouse-role boxes with an active CA, this
        // closes the manual `mackesd ca sign-csr` operator step
        // for the common case. Spawned outside the nebula-supervisor
        // Ok arm so the watcher runs even if the supervisor's
        // SQLite open failed (the watcher opens its own per-tick
        // connection).
        let csr_watcher_mesh_id = std::env::var("MDE_MESH_ID")
            .unwrap_or_else(|_| format!("mesh-{node_id}"));
        let csr_watcher_lighthouse_addr = {
            let host = std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| node_id.clone());
            format!("{host}:4242")
        };
        sup.spawn(Spawn::new(
            mackesd_core::workers::nebula_csr_watcher::NebulaCsrWatcher::new(
                qnm_root.clone(),
                db_path.clone(),
                csr_watcher_mesh_id,
                node_id.clone(),
                csr_watcher_lighthouse_addr,
            )
            .with_signal_slot(std::sync::Arc::clone(&nebula_signal_slot)),
            RestartPolicy::OnFailure,
        ));
        worker_names
            .lock()
            .expect("worker_names mutex")
            .push("nebula_csr_watcher".into());

        // NF-18.4 (v2.5) — automated CA backup worker.
        // Opens its own SQLite handle for the per-tick
        // assemble_from_store read. Skips silently on peer-role
        // boxes (no CA key file). Requires MDE_BACKUP_PASSPHRASE
        // env var — operators opt in via the systemd unit's
        // Environment= line.
        match mackesd_core::store::open(&db_path) {
            Ok(conn) => {
                let backup_store = Arc::new(tokio::sync::Mutex::new(conn));
                let backup_mesh = std::env::var("MDE_MESH_ID")
                    .unwrap_or_else(|_| format!("mesh-{node_id}"));
                sup.spawn(Spawn::new(
                    mackesd_core::workers::nebula_ca_backup::NebulaCaBackup::new(
                        qnm_root.clone(),
                        node_id.clone(),
                        backup_mesh,
                        backup_store,
                    ),
                    RestartPolicy::OnFailure,
                ));
                worker_names
                    .lock()
                    .expect("worker_names mutex")
                    .push("nebula_ca_backup".into());
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    db_path = %db_path.display(),
                    "nebula-ca-backup: sqlite open failed; worker skipped"
                );
            }
        }

        // MON-4 (v2.6) — alert relay worker. Polls
        // ~/.local/share/mde/alerts/*.json for events
        // written by mde-alert-emit (MON-3) via Netdata's
        // health_alarm_notify.conf custom-sender hook + fires
        // an FDO desktop notification via notify-send per
        // new event. Deduplicates by deterministic ULID.
        // RestartPolicy::Always since the tick is passive +
        // operator outage detection is the failure-tolerance
        // goal.
        //
        // v6.0 Portal-1 — attach a PortalClient so CRITICAL
        // alerts also navigate Portal-full to the Control
        // (mesh-health) layer. Graceful-degrade: if the session
        // bus or mde-portal aren't running at daemon startup
        // the relay skips the portal call and surfaces the
        // FDO notification alone.
        let alert_relay = {
            let mut worker = mackesd_core::workers::alert_relay::AlertRelayWorker::new();
            match zbus::Connection::session().await {
                Ok(bus) => {
                    worker = worker.with_portal_client(
                        mackesd_core::ipc::portal::PortalClient::new(bus),
                    );
                    tracing::info!(
                        "alert_relay: PortalClient attached \
                         (CRITICAL alerts will navigate Portal-full → Control)"
                    );
                }
                Err(e) => {
                    tracing::debug!(
                        error = %e,
                        "alert_relay: session bus unavailable at startup; \
                         portal goto(control) disabled"
                    );
                }
            }
            worker
        };
        sup.spawn(Spawn::new(alert_relay, RestartPolicy::Always));
        worker_names
            .lock()
            .expect("worker_names mutex")
            .push("alert_relay".into());

        // MESHFS-2.1 (v5.0.0) — LizardFS mesh-storage fleet supervisor.
        // Silent no-op when mfsmaster/mfschunkserver binaries are absent.
        sup.spawn(Spawn::new(
            mackesd_core::workers::meshfs_worker::MeshFsWorker::new()
                .with_qnm_peer_discovery(qnm_root.clone(), node_id.clone()),
            RestartPolicy::Always,
        ));
        worker_names
            .lock()
            .expect("worker_names mutex")
            .push("meshfs_worker".into());

        // INST-11 + INST-12 + INST-13 (v2.7) — fleet upgrade-barrier
        // worker. Runs on every peer; silently no-ops until a
        // `mde-update --coordinate <ver>` writes an intent file into
        // `<mesh-home>/upgrade-intent/`. Then it runs `dnf upgrade
        // mde-core` on its own schedule, marks itself ready, fires
        // `mde-install --yes` once quorum + grace are met, and — when
        // it holds the leader lease — cleans up fully-complete intent
        // files after the +24h grace. No SQLite handle needed: the
        // barrier state lives in the GFS-replicated intent files and
        // the peer roster in the PEERVER peers dir.
        sup.spawn(Spawn::new(
            mackesd_core::workers::upgrade_intent_watcher::UpgradeIntentWatcher::new(
                qnm_root.clone(),
                node_id.clone(),
            ),
            RestartPolicy::Always,
        ));
        worker_names
            .lock()
            .expect("worker_names mutex")
            .push("upgrade_intent_watcher".into());

        // PRINT-2..PRINT-6 + PRINT-8 (v5.0.0) — auto CUPS print
        // sharing + sync. Spawned on headless + full; SKIPPED on
        // lighthouse (routing-only, no printers — Q8 lock). The
        // profile is read from the installed-profile marker
        // `mde-install` writes; missing marker → assume a printing
        // profile (full/headless) and spawn. The worker itself is a
        // silent no-op without cups/lpadmin, so an over-spawn on a
        // box that happens to lack cups is harmless.
        let print_profile = std::fs::read_to_string("/var/lib/mde/installed-profile")
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if print_profile != "lighthouse" {
            sup.spawn(Spawn::new(
                mackesd_core::workers::cups_sync::CupsSyncWorker::new(),
                RestartPolicy::Always,
            ));
            worker_names
                .lock()
                .expect("worker_names mutex")
                .push("cups_sync".into());
        } else {
            tracing::info!("cups_sync: skipped (lighthouse profile)");
        }

        // FWMON-2..4 (v5.0.0) — firewall-denied event monitor.
        // Reads kernel journal entries logged by firewalld's
        // LogDenied=all setting (enabled by birthright's
        // apply_firewall_log_denied step), filters overlay +
        // established traffic, appends denials to
        // <mesh-storage>/firewall/<host>.jsonl, and fires a Bus
        // alert when one source crosses the threshold.
        let fw_host = std::fs::read_to_string("/etc/hostname")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| node_id.clone());
        sup.spawn(Spawn::new(
            mackesd_core::workers::firewall_monitor::FirewallMonitorWorker::new(fw_host),
            RestartPolicy::Always,
        ));
        worker_names
            .lock()
            .expect("worker_names mutex")
            .push("firewall_monitor".into());

        // EPIC-MESH-PROBE (MESH-PROBE-4) — scheduled two-tier nmap
        // probe worker. Resolves mesh-peer overlay IPs, scans them
        // (fast 60s / deep 10min), writes this peer's
        // probe-inventory.json into mesh-home, and announces
        // probe/changed on the Bus when the inventory changes. The
        // `mackesd probe scan/refresh` CLI shares the same engine.
        sup.spawn(Spawn::new(
            mackesd_core::workers::probe::ProbeWorker::new(qnm_root.clone(), node_id.clone()),
            RestartPolicy::Always,
        ));
        worker_names
            .lock()
            .expect("worker_names mutex")
            .push("probe".into());

        // MON-1.b (v2.6) — Netdata aggregator-IP publisher.
        // Pairs with `apply_netdata_monitor`'s baseline
        // /etc/netdata/netdata.conf: when this peer wins
        // leader-election it publishes its overlay IP to
        // QNM-Shared so every other peer picks the same
        // aggregator; on demote it stops publishing and the
        // freshest pointer wins. Every tick re-reads the
        // freshest pointer + rewrites the local netdata.conf
        // `[stream]` block + reloads netdata when the
        // aggregator IP changes. Fail-soft per the v2.6
        // design lock: missing aggregator strips the
        // `[stream]` block so netdata stays local-only with
        // the 7-day dbengine retention. API key defaults to
        // `mesh-${MDE_MESH_ID}-netdata` so every peer in the
        // same mesh shares the value automatically without
        // an extra wizard step (operators can override via
        // MDE_NETDATA_API_KEY if they want a custom value).
        match mackesd_core::store::open(&db_path) {
            Ok(conn) => {
                let netdata_store = Arc::new(tokio::sync::Mutex::new(conn));
                let mesh_id_for_netdata = std::env::var("MDE_MESH_ID")
                    .unwrap_or_else(|_| format!("mesh-{node_id}"));
                let api_key = std::env::var("MDE_NETDATA_API_KEY")
                    .unwrap_or_else(|_| format!("{mesh_id_for_netdata}-netdata"));
                sup.spawn(Spawn::new(
                    mackesd_core::workers::netdata_aggregator::NetdataAggregator::new(
                        netdata_store,
                        node_id.clone(),
                        qnm_root.clone(),
                        api_key,
                    ),
                    RestartPolicy::Always,
                ));
                worker_names
                    .lock()
                    .expect("worker_names mutex")
                    .push("netdata_aggregator".into());
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    db_path = %db_path.display(),
                    "netdata_aggregator: sqlite open failed; worker skipped"
                );
            }
        }

        // NF-1.5 (v2.5) — TCP/443 covert listener. Binds the
        // TLS 1.3 listener on :443 (default; env-overrideable),
        // spawns the per-stream demux pump per accepted peer
        // tunnel. Cert + key paths default to
        // /etc/nebula/lighthouse.{crt,key}; overridable via
        // MDE_HTTPS_TUNNEL_{CERT,KEY} env vars so operators
        // running Let's-Encrypt-issued certs can point to the
        // existing PEM chain. On peer-role boxes (no cert
        // files), the worker fails its bind + the supervisor's
        // OnFailure backoff effectively quarantines it.
        match mackesd_core::workers::nebula_https_listener::NebulaHttpsListener::new() {
            Ok(mut w) => {
                if let Ok(p) = std::env::var("MDE_HTTPS_TUNNEL_CERT") {
                    w = w.with_cert(PathBuf::from(p));
                }
                if let Ok(p) = std::env::var("MDE_HTTPS_TUNNEL_KEY") {
                    w = w.with_key(PathBuf::from(p));
                }
                if let Ok(addr) = std::env::var("MDE_HTTPS_TUNNEL_BIND") {
                    if let Ok(parsed) = addr.parse() {
                        w = w.with_bind_addr(parsed);
                    } else {
                        tracing::warn!(
                            value = %addr,
                            "nebula-https-listener: MDE_HTTPS_TUNNEL_BIND parse failed; using default",
                        );
                    }
                }
                sup.spawn(Spawn::new(w, RestartPolicy::OnFailure));
                worker_names
                    .lock()
                    .expect("worker_names mutex")
                    .push("nebula_https_listener".into());
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "nebula-https-listener: construction failed; skipped",
                );
            }
        }

        // v4.0.1 AF-NET-2 (2026-05-23) — mesh-latency sniffer.
        // Pings every enrolled non-local peer every 30 s and
        // writes the result to ~/.cache/mde/mesh-latency.json.
        // The WB-2.k.a Cairo topology canvas + panel Mesh-status
        // tray badge both consume the file. Best-choice
        // deviation from the TransportRegistry-routed approach
        // — see worker doc-comment.
        match mackesd_core::store::open(&db_path) {
            Ok(conn) => {
                let lat_store = Arc::new(tokio::sync::Mutex::new(conn));
                let cache =
                    mackesd_core::workers::mesh_latency::default_cache_path();
                sup.spawn(Spawn::new(
                    mackesd_core::workers::mesh_latency::MeshLatencyWorker::new(
                        lat_store,
                        node_id.clone(),
                        cache,
                    ),
                    RestartPolicy::OnFailure,
                ));
                worker_names.lock().expect("worker_names mutex").push("mesh_latency".into());
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    db_path = %db_path.display(),
                    "mesh-latency: sqlite open failed; worker skipped"
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
                        tracing::info!(
                            "Fleet.Files dbus surface registered at {}",
                            mackesd_core::ipc::files::FLEET_FILES_OBJECT_PATH
                        );
                        // NF-Bundle-0 (v2.5) — hang the Nebula
                        // status surface on the same
                        // connection so NF-10..NF-18
                        // consumers (applets / workbench /
                        // mde-files / wizard) can call it
                        // without claiming a second bus name.
                        let nebula = mackesd_core::ipc::nebula::NebulaStatusService::new(
                            Arc::clone(&store),
                            node_id.clone(),
                            host.clone(),
                        )
                        .with_qnm_root(qnm_root.clone());
                        match mackesd_core::ipc::nebula::register_nebula_status_on(
                            &conn, nebula,
                        )
                        .await
                        {
                            Ok(()) => {
                                tracing::info!(
                                    "Nebula.Status dbus surface registered at {}",
                                    mackesd_core::ipc::nebula::NEBULA_STATUS_OBJECT_PATH
                                );
                                // OV-7.a/c (v2.6) — spawn the
                                // signal dispatcher loop now that
                                // the interface is registered.
                                // Fills nebula_signal_slot so the
                                // health_reconciler + CSR watcher
                                // workers (already spawned above)
                                // pick up the sender on their
                                // next tick.
                                match mackesd_core::ipc::nebula::spawn_signal_dispatcher(
                                    conn.clone(),
                                    &nebula_signal_slot,
                                )
                                .await
                                {
                                    Ok(_sender) => {
                                        tracing::info!(
                                            "Nebula signal dispatcher spawned; \
                                             health_reconciler + nebula_csr_watcher \
                                             will emit on next state transition"
                                        );
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            error = %e,
                                            "Nebula signal dispatcher spawn failed; \
                                             signals will not fire until next mackesd restart"
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    "Nebula.Status dbus registration failed; \
                                     NF-10..NF-18 consumers will see no peer data"
                                );
                            }
                        }
                        // v4.1 (2026-05-24) — Shell.{Healthz,Workers}
                        // surface on the same shared connection.
                        // Workers list is the shared Arc<Mutex<>>
                        // so post-IPC spawns (KDC + reconcile)
                        // still appear in the roster.
                        let shell_state = mackesd_core::ipc::shell::ShellState {
                            db_path: db_path.clone(),
                            worker_names: Arc::clone(&worker_names),
                        };
                        match mackesd_core::ipc::shell::register_shell_on(
                            &conn, shell_state,
                        )
                        .await
                        {
                            Ok(()) => {
                                tracing::info!(
                                    "Shell dbus surface registered at {}",
                                    mackesd_core::ipc::shell::OBJECT_PATH
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    "Shell dbus registration failed; \
                                     mackes-panel status cluster will fall back \
                                     to subprocess invocation of `mackesd healthz`"
                                );
                            }
                        }
                        Box::leak(Box::new(conn));
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
        worker_names.lock().expect("worker_names mutex").push("kdc_host".into());

        // BUS-1.1 (v6.x Mackes Bus) — supervise the `mde-bus` daemon
        // subprocess. Gracefully degrades when the binary is absent
        // (dev box, RPM not yet installed) — the worker loops on a
        // 30s tick waiting for the binary to appear. Once the BUS-1
        // sub-epic ships, every mackesd peer carries the bus.
        sup.spawn(Spawn::new(
            mackesd_core::workers::bus_supervisor::BusSupervisor::new(),
            RestartPolicy::Always,
        ));
        worker_names.lock().expect("worker_names mutex").push("bus_supervisor".into());

        // BUS-5.1 — clipboard daemon. Spawns one `mde-clipd` process per
        // Wayland session. Idles gracefully when $WAYLAND_DISPLAY is unset
        // (e.g., early in the boot sequence or on a headless peer).
        sup.spawn(Spawn::new(
            mackesd_core::workers::clipd_supervisor::ClipdSupervisor::new(),
            RestartPolicy::Always,
        ));
        worker_names.lock().expect("worker_names mutex").push("clipd_supervisor".into());

        // Portal-41 (v6.0 R12-Q1) — auto-derived workspace names.
        // Subscribes to sway's window-event stream, debounces 200 ms,
        // and renames the focused workspace to `<num>: <app_id>`
        // whenever the focused window changes. Operator-set names
        // (anything not matching the `<num>` or `<num>: …` pattern)
        // are preserved. Backs off + retries on swayipc connect
        // failure (sway not running yet at mackesd boot, sway
        // restarted, $SWAYSOCK missing); the supervisor's
        // OnFailure policy still wraps the outer Err path for
        // completeness.
        sup.spawn(Spawn::new(
            mackesd_core::workers::workspace_namer::WorkspaceNamerWorker::new(),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("workspace_namer".into());

        // Portal-48 (v6.0 R12-Q8 + R12-Q10) — auto-mark daemon.
        // Subscribes to sway window::new events; classifies app_id
        // against a 25-entry taxonomy table (editor / web / shell /
        // mail / chat); fires `[con_id=N] mark --add <category>` for
        // matched windows that don't already carry an operator
        // mark. Marks are sway-session-ephemeral; no GFS sync.
        // Drives Portal-49's running-zone mark-pill render.
        sup.spawn(Spawn::new(
            mackesd_core::workers::auto_mark::AutoMarkWorker::new(),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("auto_mark".into());

        // SWAY-4 (v6.0 Q89/Q94) — per-window mark state. Bridges the
        // sway-native mark API to the Mackes Bus: serves
        // action/marks/{add,remove,list,match} topics + publishes
        // event/marks/<con_id> deltas. Snapshots to
        // ~/.local/share/mde/marks/<peer>.toml on 60 s tick + shutdown.
        // OnFailure — swayipc reconnect loop is internal; supervisor
        // only sees an Err when the reconnect itself fails repeatedly.
        sup.spawn(Spawn::new(
            mackesd_core::workers::marks_state::MarksStateWorker::new(),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("marks_state".into());

        // Portal-42 (v6.0 R12-Q2) — tag-driven workspace output
        // assignment. Subscribes to sway workspace::init events;
        // looks up the owning tag from Portal-18.a's tag store;
        // fires `move workspace to output <name>` when the tag
        // has a `preferred_output` set. Untagged workspaces +
        // tags without preferred_output keep sway's natural
        // placement.
        sup.spawn(Spawn::new(
            mackesd_core::workers::workspace_router::WorkspaceRouterWorker::new(),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("workspace_router".into());

        // Portal-44 (v6.0 R12-Q4) — per-tag default_layout
        // enforcement. Subscribes to sway window::new events;
        // flips a single-window tag-owned workspace to the
        // owning tag's `default_layout` (splith / splitv /
        // tabbed / stacked).
        sup.spawn(Spawn::new(
            mackesd_core::workers::tag_layout::TagLayoutWorker::new(),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("tag_layout".into());

        // Portal-54 (v6.0 R12-Q16) — per-tag autostart.
        // Subscribes to sway workspace::init events; fires
        // `exec <cmd>` for each app_id in the owning tag's
        // `autostart` list, once per workspace per mded-lifetime.
        sup.spawn(Spawn::new(
            mackesd_core::workers::tag_autostart::TagAutostartWorker::new(),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("tag_autostart".into());

        // Portal-56 (v6.0 R12-Q21) — per-workspace focused-border
        // tinting. Subscribes to sway workspace::focus events;
        // fires `client.focused` with the owning tag's
        // group_color (or the platform Carbon blue when no tag
        // owns the workspace). Tagless workspaces default to
        // `#2b9af3`.
        sup.spawn(Spawn::new(
            mackesd_core::workers::border_tinter::BorderTinterWorker::new(),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("border_tinter".into());

        // Portal-57.a (v6.0 R12-Q22) — channel 1 of the urgent-
        // window cascade. Subscribes to sway window::urgent events;
        // spawns `mde-bus publish bus/mbadge/pulse` with the
        // urgent payload. Gracefully degrades when the mde-bus
        // binary isn't installed (logs a single warning, continues).
        sup.spawn(Spawn::new(
            mackesd_core::workers::urgency_router::UrgencyRouterWorker::new(),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("urgency_router".into());

        // TUNE-3.b (2026-05-26) — wire the v1.3.0 Fleet ansible-pull
        // worker. `crates/mackesd/src/workers/ansible_pull.rs::build`
        // has shipped since v2.0.0 Phase B.6 but stayed dead;
        // [[project_v1_3_0_fleet]] keeps the feature in scope so
        // wiring is the right cleanup. The worker invokes
        // `ansible-pull -U <MDE_ANSIBLE_PULL_URL> -i localhost,` on
        // a 15-min cadence (matches the retired
        // `mackes-ansible-pull.timer`). With MDE_ANSIBLE_PULL_URL
        // unset the ansible-pull binary fails fast + the supervisor
        // logs the error — the worker stays cheap to host.
        sup.spawn(Spawn::new(
            mackesd_core::workers::ansible_pull::build(),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("ansible-pull".into());

        // EPIC-SYNC-APP-CONFIG (Q26, 2026-05-28) — app-config sync is
        // now a native-Rust worker (`workers::app_sync`); it discovers
        // mesh media servers + writes Sublime Music / Delfin configs +
        // the `~/Mackes Media/` launcher view directly, retiring the
        // `python3 -m mackes.media_sync_daemon` subprocess (advances
        // §11 #6). `OnFailure` keeps the 60 s tick alive across a
        // transient write/probe error.
        sup.spawn(Spawn::new(
            mackesd_core::workers::app_sync::build(),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("app-sync".into());
        // TUNE-3.b — remmina_sync is still a subprocess-tick worker
        // driving `python3 -m mackes.remmina_sync`; its native port is
        // tracked under EPIC-SYNC-APP-CONFIG's sibling note +
        // EPIC-RETIRE-PY-DAEMONS.
        sup.spawn(Spawn::new(
            mackesd_core::workers::remmina_sync::build(),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("remmina-sync".into());

        // Portal-52.a (v6.0 R12-Q13) — sway session-restore
        // worker (workspace-structure half). 5s snapshot of
        // workspaces + outputs + layouts to
        // <XDG_DATA_HOME>/mde/session.json; first-run-after-start
        // restore replays the structure via swayipc. Window-
        // placeholder swallows ship as Portal-52.b once
        // append_layout reference is locked.
        sup.spawn(Spawn::new(
            mackesd_core::workers::session_persist::SessionPersistWorker::new(),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("session_persist".into());

        // Portal-53.a (v6.0 R12-Q14) — window-rules subsystem
        // backend. Reads `~/.config/mde/window-rules.toml` on
        // startup, applies each rule via swayipc `for_window`
        // registrations. 5 s mtime-poll watches the file for
        // operator edits + re-applies on change. Hub right-click
        // modal + Control panel CRUD UIs ship as Portal-53.b
        // + Portal-53.c once Portal-17 / Portal-20 are ready.
        sup.spawn(Spawn::new(
            mackesd_core::workers::window_rules::WindowRulesWorker::new(),
            RestartPolicy::OnFailure,
        ));
        worker_names.lock().expect("worker_names mutex").push("window_rules".into());

        // HYP-8.5.watch (v6.5) — tag-manifest watcher. Polls
        // `~/.config/mde/tags/` every 5 s; publishes
        // `event/config/tags/loaded` on new + edited files and
        // `event/config/tags/unloaded` on removed files. Pairs
        // with the run_serve startup load (HYP-8.5) to give
        // operators runtime reload of tag policy without
        // restarting mackesd. Skips entirely when neither
        // $XDG_CONFIG_HOME nor $HOME is set (vanishingly rare).
        if let Some(watcher) =
            mackesd_core::workers::tag_manifest_watcher::TagManifestWatcherWorker::from_default_dir()
        {
            sup.spawn(Spawn::new(watcher, RestartPolicy::OnFailure));
            worker_names
                .lock()
                .expect("worker_names mutex")
                .push("tag_manifest_watcher".into());
        }

        // The reconcile worker runs on its own OS thread (kept on
        // std::thread so its sync rusqlite calls don't block the
        // tokio scheduler). Still surfaced via Shell.Workers so
        // the operator sees the legacy worker alongside the async
        // supervisor children.
        worker_names.lock().expect("worker_names mutex").push("reconcile".into());
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
/// VV-2 helper — load `VoiceDesired` from the operator's JSON
/// override file at `desired_json`, falling back to
/// `boot_default(node_id)` when the file is absent or `force_boot`
/// is set.
///
/// `force_boot=true` is the explicit `--boot-default` CLI flag —
/// useful for testing the bootstrap path without removing the
/// override file. A missing override file is the steady-state on a
/// fresh peer (no voice policies have been approved yet), so it's
/// a silent fall-through rather than a hard error. Parse errors
/// on a present file *are* hard errors — the operator's
/// hand-edited / worker-written file is bad and we should not
/// silently fall back to defaults that hide the bug.
fn load_voice_desired(
    desired_json: &std::path::Path,
    force_boot: bool,
    node_id: &str,
) -> anyhow::Result<mde_voice_config::VoiceDesired> {
    if force_boot {
        return Ok(mde_voice_config::VoiceDesired::boot_default(node_id));
    }
    match std::fs::read_to_string(desired_json) {
        Ok(body) => serde_json::from_str(&body).map_err(|e| {
            anyhow::anyhow!(
                "voice render-config: parse {}: {e}",
                desired_json.display()
            )
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(mde_voice_config::VoiceDesired::boot_default(node_id))
        }
        Err(e) => Err(anyhow::anyhow!(
            "voice render-config: read {}: {e}",
            desired_json.display()
        )),
    }
}

/// VV-1 helper — atomic write-and-rename of the generated voice
/// configs. The directory is `mkdir -p`'d; each file is written
/// to a hidden `.tmp` sibling and renamed into place so a
/// partial render never leaves Kamailio / `RTPengine` reading a
/// half-written file.
fn write_voice_config_files(
    out_dir: &std::path::Path,
    files: &[(&str, &String)],
) -> anyhow::Result<()> {
    std::fs::create_dir_all(out_dir).map_err(|e| {
        anyhow::anyhow!("voice render-config: mkdir {}: {e}", out_dir.display())
    })?;
    for (name, body) in files {
        let final_path = out_dir.join(name);
        let tmp_path = out_dir.join(format!(".{name}.tmp"));
        std::fs::write(&tmp_path, body.as_bytes()).map_err(|e| {
            anyhow::anyhow!(
                "voice render-config: write {}: {e}",
                tmp_path.display()
            )
        })?;
        std::fs::rename(&tmp_path, &final_path).map_err(|e| {
            anyhow::anyhow!(
                "voice render-config: rename {} → {}: {e}",
                tmp_path.display(),
                final_path.display()
            )
        })?;
    }
    Ok(())
}

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
