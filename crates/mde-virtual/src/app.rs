//! `mde-virtual` application core (VIRT-13 + VIRT-14.a).
//!
//! Holds the Fleet/Local viewer, the 5 s Bus poll, the Local tab's direct
//! `virsh`/`podman` control + offline fallback, and the VM detail panel
//! (state, stats, full lifecycle actions, console). Every visual value
//! flows from `mde-theme` tokens — no hardcoded colors or sizes.
//!
//! Per the §13 design (M4/M5/M11): the Fleet tab is read-only for remote
//! peers; the Local tab controls *this* peer's compute directly via
//! `virsh -c qemu:///system <verb>` / `podman <verb>` (no Bus round-trip),
//! the VM console launches `virt-viewer --connect qemu:///system`, and a
//! direct `virsh list` / `podman ps` read backs the Local tab when the
//! Bus is unavailable.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_input, Space};
use iced::{Background, Border, Color, Element, Length, Subscription, Task};
use mde_theme::{Density, Rgba, Theme, Tokens, TypeRole};
use serde::{Deserialize, Serialize};

use crate::sparkline::{self, push_sample};
use crate::wizard::{Template, WizardAction, WizardMsg, WizardState};

// ── Inventory data model ────────────────────────────────────────────────
//
// A read-only mirror of the document `mded`'s `compute_registry` worker
// publishes to `compute/inventory/<peer>`. We own a local copy rather
// than depend on that crate (its worker is `async-services`-gated). Every
// field is `#[serde(default)]` so a future schema addition can't break
// deserialization.

/// One VM row from a peer's published inventory.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct VmEntry {
    /// libvirt domain UUID (the migrate `vm_id`).
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub cpu_pct: f64,
    #[serde(default)]
    pub ram_mb: u64,
    #[serde(default)]
    pub disk_path: String,
    #[serde(default)]
    pub nebula_ip: String,
    #[serde(default)]
    pub meshfs_available: bool,
}

/// One container row from a peer's published inventory.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ContainerEntry {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub cpu_pct: f64,
    #[serde(default)]
    pub ram_mb: u64,
    /// Pod this container belongs to (`""` if standalone). VIRT-18.b.
    #[serde(default)]
    pub pod: String,
}

/// A single peer's compute inventory document.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Inventory {
    /// Publisher's Nebula overlay IP (the topic suffix).
    #[serde(default)]
    pub peer: String,
    /// Publisher's hostname — the display label + local-peer match key.
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub vms: Vec<VmEntry>,
    #[serde(default)]
    pub containers: Vec<ContainerEntry>,
}

/// Whether a resource row is a VM (KVM) or a container (Podman).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResourceKind {
    Vm,
    Container,
}

impl ResourceKind {
    /// The type-badge label.
    pub(crate) fn badge(self) -> &'static str {
        match self {
            ResourceKind::Vm => "KVM",
            ResourceKind::Container => "Podman",
        }
    }
}

/// A normalized display row unifying VMs + containers.
#[derive(Debug, Clone)]
pub(crate) struct ResourceRow {
    /// libvirt UUID for VMs (empty for containers); the migrate `vm_id`.
    pub id: String,
    pub name: String,
    pub kind: ResourceKind,
    pub state: String,
    pub cpu_pct: f64,
    pub ram_mb: u64,
    /// Empty for containers (Podman rows carry no overlay IP).
    pub nebula_ip: String,
    /// Empty for containers.
    pub disk_path: String,
    /// `false` for containers.
    pub meshfs_available: bool,
    /// Container image ref (empty for VMs).
    pub image: String,
    /// Pod name for containers (`""` for VMs + standalone containers). VIRT-18.b.
    pub pod: String,
}

/// Flatten an inventory's VMs (first) then containers into display rows.
pub(crate) fn rows_for(inv: &Inventory) -> Vec<ResourceRow> {
    let mut out = Vec::with_capacity(inv.vms.len() + inv.containers.len());
    for vm in &inv.vms {
        out.push(ResourceRow {
            id: vm.id.clone(),
            name: vm.name.clone(),
            kind: ResourceKind::Vm,
            state: vm.state.clone(),
            cpu_pct: vm.cpu_pct,
            ram_mb: vm.ram_mb,
            nebula_ip: vm.nebula_ip.clone(),
            disk_path: vm.disk_path.clone(),
            meshfs_available: vm.meshfs_available,
            image: String::new(),
            pod: String::new(),
        });
    }
    for c in &inv.containers {
        out.push(ResourceRow {
            id: String::new(),
            name: c.name.clone(),
            kind: ResourceKind::Container,
            state: c.state.clone(),
            cpu_pct: c.cpu_pct,
            ram_mb: c.ram_mb,
            nebula_ip: String::new(),
            disk_path: String::new(),
            meshfs_available: false,
            image: c.image.clone(),
            pod: c.pod.clone(),
        });
    }
    out
}

/// Partition display rows for the grouped Local view (VIRT-18.b): VMs and
/// pod-less containers stay flat (original order); containers sharing a pod
/// are grouped under that pod name (groups sorted by name). Returns
/// `(flat, pod_groups)`.
pub(crate) fn group_by_pod(
    rows: &[ResourceRow],
) -> (Vec<&ResourceRow>, Vec<(String, Vec<&ResourceRow>)>) {
    use std::collections::BTreeMap;
    let mut flat = Vec::new();
    let mut pods: BTreeMap<String, Vec<&ResourceRow>> = BTreeMap::new();
    for r in rows {
        if r.kind == ResourceKind::Container && !r.pod.is_empty() {
            pods.entry(r.pod.clone()).or_default().push(r);
        } else {
            flat.push(r);
        }
    }
    (flat, pods.into_iter().collect())
}

/// True when a libvirt/podman state string reads as actively running.
pub(crate) fn state_is_running(state: &str) -> bool {
    state.to_ascii_lowercase().contains("running")
}

/// True when a state string reads as paused/suspended.
pub(crate) fn state_is_paused(state: &str) -> bool {
    let s = state.to_ascii_lowercase();
    s.contains("paused") || s.contains("suspended")
}

// ── Local control (VIRT-13.b + 14.a) ────────────────────────────────────

/// A lifecycle action applied directly to this peer's compute (the Local
/// tab + the VM detail panel are always local) via `virsh` / `podman`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionVerb {
    Start,
    Stop,
    ForceOff,
    Suspend,
    Resume,
}

impl ActionVerb {
    fn label(self) -> &'static str {
        match self {
            ActionVerb::Start => "Start",
            ActionVerb::Stop => "Stop",
            ActionVerb::ForceOff => "Force off",
            ActionVerb::Suspend => "Suspend",
            ActionVerb::Resume => "Resume",
        }
    }
}

/// The five VM detail-panel actions, in display order.
const DETAIL_ACTIONS: [ActionVerb; 5] = [
    ActionVerb::Start,
    ActionVerb::Stop,
    ActionVerb::ForceOff,
    ActionVerb::Suspend,
    ActionVerb::Resume,
];

/// The contextual quick-action set for a Local-tab row.
pub(crate) fn actions_for_state(state: &str) -> Vec<ActionVerb> {
    if state_is_running(state) {
        vec![ActionVerb::Stop, ActionVerb::Suspend]
    } else if state_is_paused(state) {
        vec![ActionVerb::Resume]
    } else {
        vec![ActionVerb::Start]
    }
}

/// VIRT-20: a stable selection key for a Local-tab row.
pub(crate) fn row_key(kind: ResourceKind, name: &str) -> String {
    let tag = match kind {
        ResourceKind::Vm => "vm",
        ResourceKind::Container => "ct",
    };
    format!("{tag}:{name}")
}

/// Both VMs and containers support bulk Start/Stop (the only verbs the
/// bulk bar exposes).
fn kind_supports_bulk(_kind: ResourceKind, verb: ActionVerb) -> bool {
    matches!(verb, ActionVerb::Start | ActionVerb::Stop)
}

/// VIRT-20: the bulk verbs valid for a selection — the intersection of
/// what every selected kind supports, limited to the bar's Start/Stop.
/// Empty selection ⇒ no actions.
pub(crate) fn bulk_actions_for(kinds: &[ResourceKind]) -> Vec<ActionVerb> {
    if kinds.is_empty() {
        return vec![];
    }
    [ActionVerb::Start, ActionVerb::Stop]
        .into_iter()
        .filter(|v| kinds.iter().all(|k| kind_supports_bulk(*k, *v)))
        .collect()
}

/// VIRT-20: resolve selected keys to `(kind, name)` targets against the
/// current rows — one entry per selected row.
pub(crate) fn bulk_targets(
    selected: &HashSet<String>,
    rows: &[ResourceRow],
) -> Vec<(ResourceKind, String)> {
    rows.iter()
        .filter(|r| selected.contains(&row_key(r.kind, &r.name)))
        .map(|r| (r.kind, r.name.clone()))
        .collect()
}

/// Whether a verb is meaningful for a given state (used to enable/disable
/// the detail-panel buttons): start a stopped resource, stop/force/suspend
/// a running one, resume a paused one.
pub(crate) fn verb_applies(verb: ActionVerb, state: &str) -> bool {
    match verb {
        ActionVerb::Start => !state_is_running(state) && !state_is_paused(state),
        ActionVerb::Stop | ActionVerb::ForceOff | ActionVerb::Suspend => state_is_running(state),
        ActionVerb::Resume => state_is_paused(state),
    }
}

/// Resolve `(program, argv)` for a lifecycle action. VMs go through the
/// system libvirtd (`-c qemu:///system`); containers through `podman`.
pub(crate) fn command_for(kind: ResourceKind, verb: ActionVerb, name: &str) -> (&'static str, Vec<String>) {
    match kind {
        ResourceKind::Vm => {
            let v = match verb {
                ActionVerb::Start => "start",
                ActionVerb::Stop => "shutdown",
                ActionVerb::ForceOff => "destroy",
                ActionVerb::Suspend => "suspend",
                ActionVerb::Resume => "resume",
            };
            (
                "virsh",
                vec![
                    "-c".to_string(),
                    "qemu:///system".to_string(),
                    v.to_string(),
                    name.to_string(),
                ],
            )
        }
        ResourceKind::Container => {
            let v = match verb {
                ActionVerb::Start => "start",
                ActionVerb::Stop => "stop",
                ActionVerb::ForceOff => "kill",
                ActionVerb::Suspend => "pause",
                ActionVerb::Resume => "unpause",
            };
            ("podman", vec![v.to_string(), name.to_string()])
        }
    }
}

/// Resolve `(program, argv)` for launching a VM's graphical console
/// (§13 M5 — `virt-viewer --connect qemu:///system <domain>`).
pub(crate) fn console_command(name: &str) -> (&'static str, Vec<String>) {
    (
        "virt-viewer",
        vec!["--connect".to_string(), "qemu:///system".to_string(), name.to_string()],
    )
}

/// Resolve `(program, argv)` for taking a snapshot of a local VM
/// (libvirt auto-names it).
pub(crate) fn snapshot_create_command(vm: &str) -> (&'static str, Vec<String>) {
    (
        "virsh",
        vec![
            "-c".to_string(),
            "qemu:///system".to_string(),
            "snapshot-create-as".to_string(),
            vm.to_string(),
        ],
    )
}

/// Resolve `(program, argv)` for deleting a named snapshot from a VM.
pub(crate) fn snapshot_delete_command(vm: &str, snap: &str) -> (&'static str, Vec<String>) {
    (
        "virsh",
        vec![
            "-c".to_string(),
            "qemu:///system".to_string(),
            "snapshot-delete".to_string(),
            vm.to_string(),
            snap.to_string(),
        ],
    )
}

/// Parse `virsh snapshot-list <vm> --name` output (one name per line;
/// tolerates the full-table form by skipping the header/separator rows).
/// Pure.
pub(crate) fn parse_virsh_snapshot_list(stdout: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("---") {
            continue;
        }
        let first = t.split_whitespace().next().unwrap_or("");
        if first.is_empty() || first == "Name" {
            continue;
        }
        out.push(first.to_string());
    }
    out
}

/// Read a VM's snapshot names directly via virsh.
fn snapshot_list(vm: &str) -> Vec<String> {
    parse_virsh_snapshot_list(&run_cmd(
        "virsh",
        &["-c", "qemu:///system", "snapshot-list", vm, "--name"],
    ))
}

/// Run a command + return its stdout (empty on missing binary / failure).
/// Mirrors `compute_registry::run_virsh`.
fn run_cmd(program: &str, args: &[&str]) -> String {
    std::process::Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

/// Parse `virsh list --all` table output into `(name, state)` pairs.
/// State is free-form and may contain a space (`shut off`). Pure.
pub(crate) fn parse_virsh_list_state(stdout: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("---") {
            continue;
        }
        let cols: Vec<&str> = t.split_whitespace().collect();
        if cols.first().copied() == Some("Id") {
            continue; // header row
        }
        if cols.len() < 3 {
            continue;
        }
        let name = cols[1].to_string();
        let state = cols[2..].join(" ");
        out.push((name, state));
    }
    out
}

/// Argv for force-removing a local container (VIRT-18.b.2).
pub(crate) fn container_delete_command(name: &str) -> (&'static str, Vec<String>) {
    ("podman", vec!["rm".into(), "-f".into(), name.into()])
}

/// Argv to start/stop every container in a pod (VIRT-18.b.3).
pub(crate) fn pod_lifecycle_command(pod: &str, start: bool) -> (&'static str, Vec<String>) {
    let verb = if start { "start" } else { "stop" };
    ("podman", vec!["pod".into(), verb.into(), pod.into()])
}

/// Argv for force-removing a pod and its containers (VIRT-18.b.3).
pub(crate) fn pod_delete_command(pod: &str) -> (&'static str, Vec<String>) {
    ("podman", vec!["pod".into(), "rm".into(), "-f".into(), pod.into()])
}

/// Argv for removing a local image (VIRT-19.b).
pub(crate) fn image_delete_command(image_ref: &str) -> (&'static str, Vec<String>) {
    ("podman", vec!["rmi".into(), image_ref.into()])
}

/// True when `image_ref` is the image of a running container (VIRT-19.b
/// in-use guard — podman would refuse the removal, so the UI refuses up
/// front with an inline message instead of opening the confirmation).
pub(crate) fn image_in_use(image_ref: &str, containers: &[ContainerEntry]) -> bool {
    containers
        .iter()
        .any(|c| c.image == image_ref && c.state == "running")
}

/// Parse `podman ps --all --format json` into container rows (name +
/// state only). Mirrors `compute_registry::parse_podman_ps_json`. Pure.
pub(crate) fn parse_podman_ps_local(stdout: &str) -> Vec<ContainerEntry> {
    let Ok(rows) = serde_json::from_str::<Vec<serde_json::Value>>(stdout) else {
        return vec![];
    };
    rows.into_iter()
        .filter_map(|row| {
            let name = row
                .get("Names")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let state = row
                .get("State")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let image = row
                .get("Image")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return None;
            }
            let pod = row
                .get("PodName")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Some(ContainerEntry {
                name,
                image,
                state,
                cpu_pct: 0.0,
                ram_mb: 0,
                pod,
            })
        })
        .collect()
}

/// One local Podman image row (VIRT-19.a) — read-only browse view.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ImageEntry {
    pub repository: String,
    pub tag: String,
    pub size: String,
    pub created: String,
}

/// One local Podman volume row (VIRT-19.a) — read-only.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VolumeEntry {
    pub name: String,
    pub mountpoint: String,
    pub driver: String,
}

/// Humanize a byte count (`podman images` reports `Size` as an int on
/// some versions, a pre-formatted string on others — see [`json_size`]).
fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut s = bytes as f64;
    let mut i = 0;
    while s >= 1024.0 && i < UNITS.len() - 1 {
        s /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{bytes} B")
    } else {
        format!("{s:.1} {}", UNITS[i])
    }
}

/// Repository + tag for an image row, tolerating both the `Names`
/// (`["repo:tag"]`) and the `Repository`/`Tag` JSON shapes podman emits.
fn image_repo_tag(row: &serde_json::Value) -> (String, String) {
    if let Some(name) = row
        .get("Names")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
    {
        // Split on the LAST colon so a registry `host:port/repo:tag`
        // keeps its port with the repository.
        return match name.rsplit_once(':') {
            Some((repo, tag)) => (repo.to_string(), tag.to_string()),
            None => (name.to_string(), "latest".to_string()),
        };
    }
    let repo = row.get("Repository").and_then(|v| v.as_str()).unwrap_or("<none>").to_string();
    let tag = row.get("Tag").and_then(|v| v.as_str()).unwrap_or("<none>").to_string();
    (repo, tag)
}

fn json_size(row: &serde_json::Value) -> String {
    match row.get("Size") {
        Some(v) if v.is_u64() => human_size(v.as_u64().unwrap_or(0)),
        Some(v) if v.is_string() => v.as_str().unwrap_or("").to_string(),
        _ => String::new(),
    }
}

fn json_created(row: &serde_json::Value) -> String {
    if let Some(s) = row.get("CreatedAt").and_then(|v| v.as_str()) {
        return s.to_string();
    }
    match row.get("Created") {
        Some(v) if v.is_string() => v.as_str().unwrap_or("").to_string(),
        Some(v) if v.is_u64() => v.as_u64().unwrap_or(0).to_string(),
        _ => String::new(),
    }
}

/// Parse `podman images --format json` into image rows. Pure; tolerant
/// of the field-shape differences across podman versions.
pub(crate) fn parse_podman_images(stdout: &str) -> Vec<ImageEntry> {
    let Ok(rows) = serde_json::from_str::<Vec<serde_json::Value>>(stdout) else {
        return vec![];
    };
    rows.into_iter()
        .map(|row| {
            let (repository, tag) = image_repo_tag(&row);
            ImageEntry {
                repository,
                tag,
                size: json_size(&row),
                created: json_created(&row),
            }
        })
        .collect()
}

/// Parse `podman volume ls --format json` into volume rows. Pure.
pub(crate) fn parse_podman_volumes(stdout: &str) -> Vec<VolumeEntry> {
    let Ok(rows) = serde_json::from_str::<Vec<serde_json::Value>>(stdout) else {
        return vec![];
    };
    rows.into_iter()
        .filter_map(|row| {
            let name = row.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if name.is_empty() {
                return None;
            }
            Some(VolumeEntry {
                name,
                mountpoint: row
                    .get("Mountpoint")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                driver: row
                    .get("Driver")
                    .and_then(|v| v.as_str())
                    .unwrap_or("local")
                    .to_string(),
            })
        })
        .collect()
}

/// Read this peer's local Podman images + volumes (VIRT-19.a). Empty
/// vectors when podman is absent.
pub(crate) fn read_local_images_volumes() -> (Vec<ImageEntry>, Vec<VolumeEntry>) {
    let images = parse_podman_images(&run_cmd("podman", &["images", "--format", "json"]));
    let volumes = parse_podman_volumes(&run_cmd("podman", &["volume", "ls", "--format", "json"]));
    (images, volumes)
}

/// Read this peer's compute directly off libvirt + podman — the Bus-
/// independent fallback the Local tab uses when the mesh is unavailable
/// (§13 M11). CPU/RAM/disk are left empty (degraded offline view).
fn read_local_direct() -> Inventory {
    let vms = parse_virsh_list_state(&run_cmd("virsh", &["-c", "qemu:///system", "list", "--all"]))
        .into_iter()
        .map(|(name, state)| VmEntry {
            id: String::new(),
            name,
            state,
            cpu_pct: 0.0,
            ram_mb: 0,
            disk_path: String::new(),
            nebula_ip: String::new(),
            meshfs_available: false,
        })
        .collect();
    let containers = parse_podman_ps_local(&run_cmd("podman", &["ps", "--all", "--format", "json"]));
    Inventory {
        peer: String::new(),
        hostname: local_hostname(),
        vms,
        containers,
    }
}

// ── Bus read ────────────────────────────────────────────────────────────

/// Resolve the Mackes Bus on-disk root (`$XDG_DATA_HOME/mde/bus`).
/// Mirrors `mde_bus::default_data_dir()`; replicated as a one-liner so the
/// GUI doesn't pull in the whole broker crate just to learn one directory.
fn bus_root() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("mde").join("bus"))
}

/// Lexically-max ULID from an iterator (`None` if empty). ULIDs sort
/// lexicographically by creation time, so the max is the newest.
fn newest_ulid(ulids: impl Iterator<Item = String>) -> Option<String> {
    ulids.max()
}

/// Newest `compute/event/<peer>/<ulid>.json` filename-ULID across every
/// peer on the Bus (VIRT-21.b), or `None` when the Bus / topic is absent.
/// Used as a high-water mark so the Fleet tab can spot a fresh VM
/// lifecycle event and re-poll inventory immediately.
fn newest_compute_event_ulid() -> Option<String> {
    let dir = bus_root()?.join("compute").join("event");
    let mut ulids = Vec::new();
    for peer in std::fs::read_dir(&dir).ok()?.flatten() {
        let pp = peer.path();
        if !pp.is_dir() {
            continue;
        }
        if let Ok(files) = std::fs::read_dir(&pp) {
            for f in files.flatten() {
                let p = f.path();
                if p.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                        ulids.push(stem.to_string());
                    }
                }
            }
        }
    }
    newest_ulid(ulids.into_iter())
}

/// This machine's hostname (`/etc/hostname`), used to pick the local peer.
pub(crate) fn local_hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// True when `inv` is this peer's own inventory (hostname match).
pub(crate) fn is_local(inv: &Inventory, local_host: &str) -> bool {
    !local_host.is_empty() && inv.hostname == local_host
}

/// One poll's Bus result. `Unavailable` → the Fleet tab renders the
/// "Mesh unavailable" banner. An empty `Available` = Bus up, no compute.
#[derive(Debug, Clone)]
pub(crate) enum FleetState {
    Unavailable,
    Available(Vec<Inventory>),
}

impl FleetState {
    fn inventories(&self) -> &[Inventory] {
        match self {
            FleetState::Available(v) => v,
            FleetState::Unavailable => &[],
        }
    }

    fn is_unavailable(&self) -> bool {
        matches!(self, FleetState::Unavailable)
    }
}

/// The full result of one poll: the Bus fleet, plus a direct local read
/// when (and only when) the Bus is unavailable (§13 M11 fallback).
#[derive(Debug, Clone)]
pub(crate) struct PollResult {
    pub fleet: FleetState,
    pub local_direct: Option<Inventory>,
}

/// Poll the Bus; when it's unavailable, also read the local compute
/// directly so the Local tab keeps working offline.
pub(crate) fn poll() -> PollResult {
    let fleet = read_fleet();
    let local_direct = if fleet.is_unavailable() {
        Some(read_local_direct())
    } else {
        None
    };
    PollResult { fleet, local_direct }
}

/// Read the current fleet inventory off the Bus tree.
fn read_fleet() -> FleetState {
    let Some(root) = bus_root() else {
        return FleetState::Unavailable;
    };
    if !root.is_dir() {
        return FleetState::Unavailable;
    }
    let inv_dir = root.join("compute").join("inventory");
    let entries = collect_inventory_files(&inv_dir);
    FleetState::Available(pick_latest_per_peer(entries))
}

/// Collect `(ulid_filename, Inventory)` pairs from the inventory topic
/// tree: `<inv_dir>/<peer>/<ulid>.json` (and tolerating flat files).
fn collect_inventory_files(inv_dir: &Path) -> Vec<(String, Inventory)> {
    let mut out = Vec::new();
    let Ok(top) = std::fs::read_dir(inv_dir) else {
        return out;
    };
    for top_entry in top.flatten() {
        let p = top_entry.path();
        if p.is_dir() {
            if let Ok(inner) = std::fs::read_dir(&p) {
                for e in inner.flatten() {
                    push_if_json(&e.path(), &mut out);
                }
            }
        } else {
            push_if_json(&p, &mut out);
        }
    }
    out
}

fn push_if_json(path: &Path, out: &mut Vec<(String, Inventory)>) {
    if path.extension().and_then(|e| e.to_str()) != Some("json") {
        return;
    }
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return;
    };
    let Ok(body) = std::fs::read_to_string(path) else {
        return;
    };
    if let Ok(inv) = serde_json::from_str::<Inventory>(&body) {
        out.push((stem.to_string(), inv));
    }
}

/// Keep only the newest inventory per peer (ULID filenames sort by time).
/// Output is sorted by hostname (then peer) for a stable display order.
pub(crate) fn pick_latest_per_peer(entries: Vec<(String, Inventory)>) -> Vec<Inventory> {
    let mut best: BTreeMap<String, (String, Inventory)> = BTreeMap::new();
    for (fname, inv) in entries {
        let replace = match best.get(&inv.peer) {
            Some((cur, _)) => fname > *cur,
            None => true,
        };
        if replace {
            best.insert(inv.peer.clone(), (fname, inv));
        }
    }
    let mut out: Vec<Inventory> = best.into_values().map(|(_, inv)| inv).collect();
    out.sort_by(|a, b| a.hostname.cmp(&b.hostname).then(a.peer.cmp(&b.peer)));
    out
}

// ── Exposed ports (VIRT-14.c) ───────────────────────────────────────────
//
// Read-only mirror of `compute_expose`'s published `compute/exposed/<peer>`
// document, used to list a VM's active port-forwards in the detail panel.

/// Which network a forward lives on (mirror of `compute_expose::Network`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Network {
    Mesh,
    Lan,
    Wan,
}

impl Network {
    fn label(self) -> &'static str {
        match self {
            Network::Mesh => "Mesh",
            Network::Lan => "LAN",
            Network::Wan => "WAN",
        }
    }
}

/// One active forwarding rule (mirror of `compute_expose::ActiveRule`).
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ActiveRule {
    pub network: Network,
    #[serde(default)]
    pub vm_nebula_ip: String,
    #[serde(default)]
    pub host_port: u16,
    #[serde(default)]
    pub proto: String,
}

/// The published `compute/exposed/<peer>` payload.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ExposedState {
    #[serde(default)]
    pub peer: String,
    #[serde(default)]
    pub rules: Vec<ActiveRule>,
}

/// Flatten per-peer exposed-state docs to the rules matching one VM's
/// overlay IP. Pure; an empty IP matches nothing.
pub(crate) fn rules_for_vm(states: Vec<ExposedState>, vm_nebula_ip: &str) -> Vec<ActiveRule> {
    if vm_nebula_ip.is_empty() {
        return vec![];
    }
    states
        .into_iter()
        .flat_map(|s| s.rules)
        .filter(|r| r.vm_nebula_ip == vm_nebula_ip)
        .collect()
}

/// Deserialize the freshest `*.json` doc per peer subdir under a topic
/// tree (newest ULID wins), tolerating flat files too.
fn latest_docs_in_topic<T: serde::de::DeserializeOwned>(topic_dir: &Path) -> Vec<T> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(topic_dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        let body = if p.is_dir() {
            latest_json_in(&p)
        } else if p.extension().and_then(|x| x.to_str()) == Some("json") {
            std::fs::read_to_string(&p).ok()
        } else {
            None
        };
        if let Some(b) = body {
            if let Ok(doc) = serde_json::from_str::<T>(&b) {
                out.push(doc);
            }
        }
    }
    out
}

/// The lexically-newest `*.json` body in a directory (newest ULID wins).
fn latest_json_in(dir: &Path) -> Option<String> {
    let mut best: Option<(String, String)> = None;
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = p.file_stem().and_then(|s| s.to_str()).map(String::from) else {
            continue;
        };
        let better = best.as_ref().map_or(true, |(s, _)| stem > *s);
        if better {
            if let Ok(body) = std::fs::read_to_string(&p) {
                best = Some((stem, body));
            }
        }
    }
    best.map(|(_, body)| body)
}

/// Read the active exposed forwards for one VM across all peers' published
/// `compute/exposed/*` state.
fn read_exposed_for_vm(vm_nebula_ip: &str) -> Vec<ActiveRule> {
    if vm_nebula_ip.is_empty() {
        return vec![];
    }
    let Some(root) = bus_root() else {
        return vec![];
    };
    let states: Vec<ExposedState> =
        latest_docs_in_topic(&root.join("compute").join("exposed"));
    rules_for_vm(states, vm_nebula_ip)
}

/// The `[Expose port…]` sheet's form state (VIRT-14.c.2).
#[derive(Debug, Clone, Default)]
pub(crate) struct ExposeForm {
    pub guest_port: String,
    /// false = TCP, true = UDP.
    pub proto_udp: bool,
    pub mesh: bool,
    pub lan: bool,
    pub wan: bool,
}

/// Expose-request payload published to `compute/expose/<peer>` (serialize
/// mirror of `compute_expose::ExposeRequest`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct ExposeRequest {
    pub vm_nebula_ip: String,
    pub guest_port: u16,
    pub proto: String,
    pub networks: Vec<Network>,
}

/// Unexpose-request payload (serialize mirror of
/// `compute_expose::UnexposeRequest`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct UnexposeRequest {
    pub vm_nebula_ip: String,
    pub host_port: u16,
    pub proto: String,
}

/// Build a validated `ExposeRequest` from the sheet form. Returns `None`
/// when the port isn't a 1–65535 integer or no network is selected. Pure.
pub(crate) fn build_expose_request(form: &ExposeForm, vm_nebula_ip: &str) -> Option<ExposeRequest> {
    let port: u16 = form.guest_port.parse().ok()?;
    if port == 0 {
        return None;
    }
    let mut networks = Vec::new();
    if form.mesh {
        networks.push(Network::Mesh);
    }
    if form.lan {
        networks.push(Network::Lan);
    }
    if form.wan {
        networks.push(Network::Wan);
    }
    if networks.is_empty() {
        return None;
    }
    Some(ExposeRequest {
        vm_nebula_ip: vm_nebula_ip.to_string(),
        guest_port: port,
        proto: if form.proto_udp { "udp" } else { "tcp" }.to_string(),
        networks,
    })
}

/// Migration-request payload published to `action/compute/migrate`
/// (serialize mirror of `compute_migrate::MigrateRequest`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct MigrateRequest {
    pub source_peer: String,
    pub target_peer: String,
    pub vm_id: String,
    pub disk_path: String,
}

/// The migrate-target candidates: every fleet peer except the source,
/// as `(nebula_ip, display_label)`. Pure.
pub(crate) fn migrate_targets(invs: &[Inventory], source_peer: &str) -> Vec<(String, String)> {
    invs.iter()
        .filter(|i| !i.peer.is_empty() && i.peer != source_peer)
        .map(|i| {
            let label = if i.hostname.is_empty() {
                i.peer.clone()
            } else {
                i.hostname.clone()
            };
            (i.peer.clone(), label)
        })
        .collect()
}

// ── Create progress (VIRT-15.b) ─────────────────────────────────────────

/// Create-ack payload read from `compute/create-ack/<request_ulid>`
/// (deserialize mirror of `compute_provision::CreateAck`).
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateAck {
    #[serde(default)]
    pub vm_id: Option<String>,
    #[serde(default)]
    pub nebula_ip: Option<String>,
    #[serde(default)]
    pub meshfs_skipped: bool,
    #[serde(default)]
    pub error: Option<String>,
}

/// The status of an in-flight VM creation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CreateStatus {
    Pending,
    Done {
        vm_id: String,
        nebula_ip: String,
        meshfs_skipped: bool,
    },
    Failed {
        error: String,
    },
}

/// Map a parsed ack to a terminal status. Pure.
pub(crate) fn status_from_ack(ack: CreateAck) -> CreateStatus {
    match ack.error.filter(|e| !e.is_empty()) {
        Some(error) => CreateStatus::Failed { error },
        None => CreateStatus::Done {
            vm_id: ack.vm_id.unwrap_or_default(),
            nebula_ip: ack.nebula_ip.unwrap_or_default(),
            meshfs_skipped: ack.meshfs_skipped,
        },
    }
}

/// A VM create awaiting (or resolved to) its ack.
#[derive(Debug, Clone)]
pub(crate) struct PendingCreate {
    pub request_ulid: String,
    pub status: CreateStatus,
}

/// Read the create-ack for a request, if the worker has replied yet.
fn read_create_ack(request_ulid: &str) -> Option<CreateStatus> {
    let root = bus_root()?;
    let dir = root.join("compute").join("create-ack").join(request_ulid);
    let body = latest_json_in(&dir)?;
    let ack: CreateAck = serde_json::from_str(&body).ok()?;
    Some(status_from_ack(ack))
}

// ── Template save (VIRT-16.a) ───────────────────────────────────────────

/// Parse `virsh dominfo` for `(vcpus, configured-RAM-MB)`. Pure.
pub(crate) fn parse_dominfo_config(stdout: &str) -> (u32, u64) {
    let mut vcpus = 0u32;
    let mut ram_mb = 0u64;
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("CPU(s):") {
            vcpus = rest.trim().parse().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("Max memory:") {
            let kib: u64 = rest
                .trim()
                .split_whitespace()
                .next()
                .and_then(|x| x.parse().ok())
                .unwrap_or(0);
            ram_mb = kib / 1024;
        }
    }
    (vcpus, ram_mb)
}

/// Parse `virsh domblkinfo` for the disk capacity in GB. Pure.
pub(crate) fn parse_blkinfo_capacity_gb(stdout: &str) -> u64 {
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("Capacity:") {
            let bytes: u64 = rest
                .trim()
                .split_whitespace()
                .next()
                .and_then(|x| x.parse().ok())
                .unwrap_or(0);
            return bytes / (1024 * 1024 * 1024);
        }
    }
    0
}

/// Query a local VM's config via virsh: `(vcpus, ram_mb, disk_gb)`.
fn query_vm_config(name: &str, disk_path: &str) -> (u32, u64, u64) {
    let (vcpus, ram_mb) =
        parse_dominfo_config(&run_cmd("virsh", &["-c", "qemu:///system", "dominfo", name]));
    let disk_gb = parse_blkinfo_capacity_gb(&run_cmd(
        "virsh",
        &["-c", "qemu:///system", "domblkinfo", name, disk_path],
    ));
    (vcpus, ram_mb, disk_gb)
}

/// Write a template to `~/.local/share/mde/vm-templates/<ulid>.json`.
fn save_template(t: &Template) -> Option<std::path::PathBuf> {
    let dir = crate::wizard::templates_dir()?;
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join(format!("{}.json", ulid::Ulid::new()));
    let body = serde_json::to_string_pretty(t).ok()?;
    std::fs::write(&path, body).ok()?;
    Some(path)
}

// ── Live sampling (VIRT-17.b) ───────────────────────────────────────────

/// Sum a `virsh domstats <vm> --vcpu` payload's `vcpu.N.time=` ns. Pure.
pub(crate) fn parse_domstats_vcpu_ns(stdout: &str) -> u64 {
    let mut total = 0u64;
    for line in stdout.lines() {
        let t = line.trim();
        if let Some(eq) = t.find('=') {
            let key = &t[..eq];
            if key.starts_with("vcpu.") && key.ends_with(".time") {
                if let Ok(v) = t[eq + 1..].trim().parse::<u64>() {
                    total += v;
                }
            }
        }
    }
    total
}

/// Parse a `virsh dommemstat <vm>` payload's `rss` KiB → MB. Pure.
pub(crate) fn parse_dommemstat_rss_mb(stdout: &str) -> u64 {
    for line in stdout.lines() {
        if let Some(rest) = line.trim().strip_prefix("rss ") {
            if let Ok(kib) = rest.trim().parse::<u64>() {
                return kib / 1024;
            }
        }
    }
    0
}

/// Sample a local VM directly via virsh: CPU% from the vcpu-time delta
/// over `interval_s`, RSS MB from dommemstat. Returns
/// `(cpu_pct, ram_mb, curr_vcpu_ns)` — the caller stores `curr_vcpu_ns`
/// as the next sample's `prev_ns`.
fn sample_vm(name: &str, prev_ns: Option<u64>, interval_s: f64) -> (f64, u64, u64) {
    let curr_ns = parse_domstats_vcpu_ns(&run_cmd(
        "virsh",
        &["-c", "qemu:///system", "domstats", name, "--vcpu"],
    ));
    let cpu_pct = match prev_ns {
        Some(p) if curr_ns >= p && interval_s > 0.0 => {
            ((curr_ns - p) as f64 / (interval_s * 1e9)) * 100.0
        }
        _ => 0.0,
    };
    let ram_mb = parse_dommemstat_rss_mb(&run_cmd(
        "virsh",
        &["-c", "qemu:///system", "dommemstat", name],
    ));
    (cpu_pct, ram_mb, curr_ns)
}

// ── VM detail (VIRT-14.a) ────────────────────────────────────────────────

/// A snapshot of one VM's data for the detail panel. Carries `is_local`
/// so the panel can enable actions only for VMs on this peer (§13 M4).
#[derive(Debug, Clone)]
pub(crate) struct VmDetail {
    pub id: String,
    pub name: String,
    pub state: String,
    pub cpu_pct: f64,
    pub ram_mb: u64,
    pub disk_path: String,
    pub nebula_ip: String,
    pub meshfs_available: bool,
    pub is_local: bool,
    /// The VM's host peer Nebula addr (the `compute/expose/<peer>` target).
    pub peer: String,
}

impl VmDetail {
    fn from_row(r: &ResourceRow, peer: &str, is_local: bool) -> Self {
        Self {
            id: r.id.clone(),
            name: r.name.clone(),
            state: r.state.clone(),
            cpu_pct: r.cpu_pct,
            ram_mb: r.ram_mb,
            disk_path: r.disk_path.clone(),
            nebula_ip: r.nebula_ip.clone(),
            meshfs_available: r.meshfs_available,
            is_local,
            peer: peer.to_string(),
        }
    }
}

// ── Iced application ──────────────────────────────────────────────────────

/// The two top-level tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    Fleet,
    Local,
}

/// Application messages.
#[derive(Debug, Clone)]
pub(crate) enum Message {
    SwitchTab(Tab),
    TogglePeer(String),
    Refresh,
    PollLoaded(PollResult),
    /// A lifecycle action on this peer's compute (Local rows + detail panel).
    Action {
        kind: ResourceKind,
        name: String,
        verb: ActionVerb,
    },
    /// Open the detail panel for a VM (row name clicked).
    SelectVm(VmDetail),
    /// Close the detail panel.
    CloseDetail,
    /// Launch the graphical console for a local VM.
    Console(String),
    /// A selected VM's snapshot list finished loading.
    SnapshotsLoaded(Vec<String>),
    /// Take a snapshot of the named local VM.
    TakeSnapshot(String),
    /// Delete a named snapshot from a local VM.
    DeleteSnapshot { vm: String, snap: String },
    /// A selected VM's exposed-port list finished loading.
    ExposedLoaded(Vec<ActiveRule>),
    /// Open the `[Expose port…]` sheet for the selected VM.
    OpenExposeSheet,
    /// Close the expose sheet without submitting.
    CloseExposeSheet,
    /// Guest-port text input changed.
    ExposePortInput(String),
    /// Toggle the protocol (TCP ↔ UDP).
    ExposeToggleProto,
    /// Toggle a network checkbox.
    ExposeToggleNet(Network),
    /// Submit the expose sheet → publish an `ExposeRequest`.
    SubmitExpose,
    /// Remove an active forward → publish an `UnexposeRequest`.
    Unexpose {
        vm_nebula_ip: String,
        host_port: u16,
        proto: String,
    },
    /// Open the `[Migrate to…]` peer-picker for the selected local VM.
    OpenMigrateSheet,
    /// Close the migrate picker without submitting.
    CloseMigrateSheet,
    /// Submit a migration of the selected VM to `target_peer`.
    MigrateTo(String),
    /// Open the `[+ New VM]` creation wizard.
    OpenWizard,
    /// A message from the open creation wizard.
    Wizard(WizardMsg),
    /// A pending create's ack poll resolved (`None` = still no ack).
    CreateAckLoaded(Option<CreateStatus>),
    /// Dismiss the create-status banner.
    DismissCreate,
    /// Open the `[Save as template…]` name prompt for the selected VM.
    OpenSaveTemplate,
    /// Template-name prompt input changed.
    SaveTemplateNameInput(String),
    /// Close the save-template prompt without saving.
    CancelSaveTemplate,
    /// Save the selected VM's config as a template.
    SubmitSaveTemplate,
    /// 2 s sampler tick (active only while a local VM is selected).
    SampleTick,
    /// A live virsh sample finished: `(cpu_pct, ram_mb, curr_vcpu_ns)`.
    SampleLoaded((f64, u64, u64)),
    /// VIRT-21.b: Fleet-tab timer to check `compute/event/*` for new VM
    /// lifecycle events (start/stop/crash).
    CheckComputeEvents,
    /// Newest `compute/event` ULID observed (`None` = no events / no Bus).
    ComputeEventMarker(Option<String>),
    /// VIRT-19.a: local Podman images + volumes finished loading.
    ImagesVolumesLoaded(Vec<ImageEntry>, Vec<VolumeEntry>),
    /// VIRT-18.b.2: open the delete-confirmation for a local container.
    RequestContainerDelete(String),
    /// Confirm the pending container delete (`podman rm -f`).
    ConfirmContainerDelete,
    /// Dismiss the delete confirmation without removing.
    CancelContainerDelete,
    /// VIRT-18.b.3: start/stop every container in a pod (`podman pod`).
    PodLifecycle { pod: String, start: bool },
    /// Open the delete-confirmation for a pod (removes it + its containers).
    RequestPodDelete(String),
    /// Confirm the pending pod delete (`podman pod rm -f`).
    ConfirmPodDelete,
    /// Dismiss the pod delete confirmation.
    CancelPodDelete,
    /// VIRT-19.b: open the delete-confirmation for a local image.
    RequestImageDelete(String),
    /// Confirm the pending image delete (`podman rmi`).
    ConfirmImageDelete,
    /// Dismiss the image delete confirmation.
    CancelImageDelete,
    /// VIRT-20: toggle a Local row's bulk selection.
    ToggleBulk(String),
    /// VIRT-20: select (`true`) or clear (`false`) all visible Local rows.
    SelectAllBulk(bool),
    /// VIRT-20: open the confirmation for a bulk verb over the selection.
    RequestBulk(ActionVerb),
    /// VIRT-20: confirm the pending bulk action (one command per resource).
    ConfirmBulk,
    /// VIRT-20: dismiss the bulk confirmation.
    CancelBulk,
}

/// `mde-virtual` application state.
pub(crate) struct VirtualApp {
    tokens: Tokens,
    tab: Tab,
    fleet: FleetState,
    /// Direct local read, populated only while the Bus is unavailable.
    local_direct: Option<Inventory>,
    /// Per-peer expansion state; absent = expanded (default open).
    expanded: HashMap<String, bool>,
    /// The VM whose detail panel is open, if any.
    selected: Option<VmDetail>,
    /// Snapshot names for the selected VM (VIRT-14.b); refreshed on select.
    selected_snapshots: Vec<String>,
    /// Active exposed-port rules for the selected VM (VIRT-14.c); on select.
    exposed_rules: Vec<ActiveRule>,
    /// The open `[Expose port…]` sheet form, if any (VIRT-14.c.2).
    expose_form: Option<ExposeForm>,
    /// Whether the `[Migrate to…]` peer-picker is open (VIRT-14.d).
    migrate_open: bool,
    /// The open VM-creation wizard, if any (VIRT-15).
    wizard: Option<WizardState>,
    /// An in-flight / just-resolved VM creation, for the status banner (15.b).
    pending_create: Option<PendingCreate>,
    /// The open `[Save as template…]` name prompt, if any (VIRT-16.a).
    save_template_name: Option<String>,
    /// Per-metric sparkline ring buffers for the selected local VM (17.a).
    cpu_history: VecDeque<f32>,
    ram_history: VecDeque<f32>,
    /// Prev cumulative vcpu-time ns for the 2 s CPU%-delta sampler (17.b).
    prev_cpu_ns: Option<u64>,
    local_host: String,
    /// VIRT-21.b: newest `compute/event` ULID seen, so the Fleet tab can
    /// detect a fresh lifecycle event and refresh promptly. Seeded on the
    /// first check so startup backlog doesn't trigger a refresh.
    event_marker: Option<String>,
    /// VIRT-19.a: local Podman images + volumes (read-only Local-tab lists),
    /// refreshed on each poll.
    images: Vec<ImageEntry>,
    volumes: Vec<VolumeEntry>,
    /// VIRT-18.b.2: the local container awaiting delete confirmation, if any.
    pending_container_delete: Option<String>,
    /// VIRT-18.b.3: the pod awaiting delete confirmation, if any.
    pending_pod_delete: Option<String>,
    /// VIRT-19.b: the image ref awaiting delete confirmation, if any.
    pending_image_delete: Option<String>,
    /// VIRT-20: selected Local-row keys for bulk actions.
    selected_bulk: HashSet<String>,
    /// VIRT-20: the bulk verb awaiting confirmation, if any.
    pending_bulk: Option<ActionVerb>,
}

impl VirtualApp {
    /// Boot the app: resolve tokens, do the initial synchronous poll, and
    /// capture this machine's hostname for the Local tab.
    pub(crate) fn new() -> Self {
        let PollResult { fleet, local_direct } = poll();
        Self {
            tokens: Tokens::resolve(Theme::Dark, Density::Comfortable),
            tab: Tab::Fleet,
            fleet,
            local_direct,
            expanded: HashMap::new(),
            selected: None,
            selected_snapshots: Vec::new(),
            exposed_rules: Vec::new(),
            expose_form: None,
            migrate_open: false,
            wizard: None,
            pending_create: None,
            save_template_name: None,
            cpu_history: VecDeque::new(),
            ram_history: VecDeque::new(),
            prev_cpu_ns: None,
            local_host: local_hostname(),
            event_marker: None,
            images: Vec::new(),
            volumes: Vec::new(),
            pending_container_delete: None,
            pending_pod_delete: None,
            pending_image_delete: None,
            selected_bulk: HashSet::new(),
            pending_bulk: None,
        }
    }

    pub(crate) fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::CheckComputeEvents => {
                Task::perform(async { newest_compute_event_ulid() }, Message::ComputeEventMarker)
            }
            Message::ComputeEventMarker(m) => {
                // Seed silently on first observation so startup backlog
                // doesn't refresh; afterwards a newer ULID means a fresh
                // lifecycle event → re-poll inventory now.
                if self.event_marker.is_none() {
                    self.event_marker = m;
                    Task::none()
                } else if m.is_some() && m != self.event_marker {
                    self.event_marker = m;
                    Task::perform(async {}, |()| Message::Refresh)
                } else {
                    Task::none()
                }
            }
            Message::SwitchTab(t) => {
                self.tab = t;
                Task::none()
            }
            Message::TogglePeer(peer) => {
                let e = self.expanded.entry(peer).or_insert(true);
                *e = !*e;
                Task::none()
            }
            Message::Refresh => {
                let mut tasks = vec![Task::perform(async { poll() }, Message::PollLoaded)];
                // Keep the open detail panel's exposed-port list fresh.
                if let Some(ip) = self
                    .selected
                    .as_ref()
                    .map(|d| d.nebula_ip.clone())
                    .filter(|s| !s.is_empty())
                {
                    tasks.push(Task::perform(
                        async move { read_exposed_for_vm(&ip) },
                        Message::ExposedLoaded,
                    ));
                }
                // Poll the in-flight create's ack until it resolves.
                if let Some(pc) = &self.pending_create {
                    if pc.status == CreateStatus::Pending {
                        let ulid = pc.request_ulid.clone();
                        tasks.push(Task::perform(
                            async move { read_create_ack(&ulid) },
                            Message::CreateAckLoaded,
                        ));
                    }
                }
                // VIRT-19.a: refresh the Local-tab images/volumes lists so
                // they're ready the moment the operator opens the Local tab.
                tasks.push(Task::perform(
                    async { read_local_images_volumes() },
                    |(images, volumes)| Message::ImagesVolumesLoaded(images, volumes),
                ));
                Task::batch(tasks)
            }
            Message::ImagesVolumesLoaded(images, volumes) => {
                self.images = images;
                self.volumes = volumes;
                Task::none()
            }
            Message::RequestContainerDelete(name) => {
                self.pending_container_delete = Some(name);
                Task::none()
            }
            Message::CancelContainerDelete => {
                self.pending_container_delete = None;
                Task::none()
            }
            Message::ConfirmContainerDelete => {
                let Some(name) = self.pending_container_delete.take() else {
                    return Task::none();
                };
                let (bin, args) = container_delete_command(&name);
                Task::perform(
                    async move {
                        let _ = std::process::Command::new(bin).args(&args).status();
                    },
                    |()| Message::Refresh,
                )
            }
            Message::PodLifecycle { pod, start } => {
                let (bin, args) = pod_lifecycle_command(&pod, start);
                Task::perform(
                    async move {
                        let _ = std::process::Command::new(bin).args(&args).status();
                    },
                    |()| Message::Refresh,
                )
            }
            Message::RequestPodDelete(pod) => {
                self.pending_pod_delete = Some(pod);
                Task::none()
            }
            Message::CancelPodDelete => {
                self.pending_pod_delete = None;
                Task::none()
            }
            Message::ConfirmPodDelete => {
                let Some(pod) = self.pending_pod_delete.take() else {
                    return Task::none();
                };
                let (bin, args) = pod_delete_command(&pod);
                Task::perform(
                    async move {
                        let _ = std::process::Command::new(bin).args(&args).status();
                    },
                    |()| Message::Refresh,
                )
            }
            Message::RequestImageDelete(image_ref) => {
                self.pending_image_delete = Some(image_ref);
                Task::none()
            }
            Message::CancelImageDelete => {
                self.pending_image_delete = None;
                Task::none()
            }
            Message::ConfirmImageDelete => {
                let Some(image_ref) = self.pending_image_delete.take() else {
                    return Task::none();
                };
                let (bin, args) = image_delete_command(&image_ref);
                Task::perform(
                    async move {
                        let _ = std::process::Command::new(bin).args(&args).status();
                    },
                    |()| Message::Refresh,
                )
            }
            Message::ToggleBulk(key) => {
                if !self.selected_bulk.remove(&key) {
                    self.selected_bulk.insert(key);
                }
                Task::none()
            }
            Message::SelectAllBulk(all) => {
                if all {
                    self.selected_bulk = self
                        .local_rows()
                        .iter()
                        .map(|r| row_key(r.kind, &r.name))
                        .collect();
                } else {
                    self.selected_bulk.clear();
                }
                Task::none()
            }
            Message::RequestBulk(verb) => {
                self.pending_bulk = Some(verb);
                Task::none()
            }
            Message::CancelBulk => {
                self.pending_bulk = None;
                Task::none()
            }
            Message::ConfirmBulk => {
                let Some(verb) = self.pending_bulk.take() else {
                    return Task::none();
                };
                let targets = bulk_targets(&self.selected_bulk, &self.local_rows());
                self.selected_bulk.clear();
                Task::perform(
                    async move {
                        for (kind, name) in targets {
                            let (bin, args) = command_for(kind, verb, &name);
                            let _ = std::process::Command::new(bin).args(&args).status();
                        }
                    },
                    |()| Message::Refresh,
                )
            }
            Message::PollLoaded(r) => {
                self.fleet = r.fleet;
                self.local_direct = r.local_direct;
                // Refresh a selected REMOTE VM's last-known stats from the
                // fresh inventory (local VMs are driven live by the 2 s
                // sampler below — no sparkline for remote per §13 M-table).
                if let Some((peer, name)) = self
                    .selected
                    .as_ref()
                    .filter(|d| !d.is_local)
                    .map(|d| (d.peer.clone(), d.name.clone()))
                {
                    let sample = self
                        .fleet
                        .inventories()
                        .iter()
                        .find(|i| i.peer == peer)
                        .and_then(|inv| inv.vms.iter().find(|v| v.name == name))
                        .map(|v| (v.cpu_pct, v.ram_mb));
                    if let Some((cpu, ram)) = sample {
                        if let Some(d) = self.selected.as_mut() {
                            d.cpu_pct = cpu;
                            d.ram_mb = ram;
                        }
                    }
                }
                Task::none()
            }
            Message::SampleTick => {
                let Some(name) = self
                    .selected
                    .as_ref()
                    .filter(|d| d.is_local)
                    .map(|d| d.name.clone())
                else {
                    return Task::none();
                };
                let prev = self.prev_cpu_ns;
                Task::perform(
                    async move { sample_vm(&name, prev, 2.0) },
                    Message::SampleLoaded,
                )
            }
            Message::SampleLoaded((cpu, ram, ns)) => {
                self.prev_cpu_ns = Some(ns);
                if let Some(d) = self.selected.as_mut() {
                    if d.is_local {
                        d.cpu_pct = cpu;
                        d.ram_mb = ram;
                    }
                }
                push_sample(&mut self.cpu_history, cpu as f32);
                push_sample(&mut self.ram_history, ram as f32);
                Task::none()
            }
            Message::Action { kind, name, verb } => {
                let (prog, args) = command_for(kind, verb, &name);
                let prog = prog.to_string();
                Task::perform(
                    async move {
                        let _ = std::process::Command::new(prog).args(&args).output();
                    },
                    |()| Message::Refresh,
                )
            }
            Message::SelectVm(detail) => {
                self.selected_snapshots = Vec::new();
                self.exposed_rules = Vec::new();
                self.migrate_open = false;
                self.save_template_name = None;
                self.cpu_history = VecDeque::new();
                self.ram_history = VecDeque::new();
                self.prev_cpu_ns = None;
                let ip = detail.nebula_ip.clone();
                let local_name = detail.is_local.then(|| detail.name.clone());
                self.selected = Some(detail);
                // Exposed ports are mesh-readable for any VM; snapshots only
                // for local VMs (virsh is local).
                let exposed =
                    Task::perform(async move { read_exposed_for_vm(&ip) }, Message::ExposedLoaded);
                match local_name {
                    Some(vm) => Task::batch([
                        exposed,
                        Task::perform(async move { snapshot_list(&vm) }, Message::SnapshotsLoaded),
                    ]),
                    None => exposed,
                }
            }
            Message::CloseDetail => {
                self.selected = None;
                self.migrate_open = false;
                self.save_template_name = None;
                Task::none()
            }
            Message::OpenWizard => {
                self.selected = None;
                self.migrate_open = false;
                self.wizard = Some(WizardState::new());
                Task::none()
            }
            Message::Wizard(wm) => {
                let Some(w) = self.wizard.as_mut() else {
                    return Task::none();
                };
                match w.update(wm) {
                    WizardAction::None => Task::none(),
                    WizardAction::Cancel => {
                        self.wizard = None;
                        Task::none()
                    }
                    WizardAction::Create(req) => {
                        self.wizard = None;
                        let Some(addr) = self.local_peer_addr() else {
                            return Task::none();
                        };
                        self.pending_create = Some(PendingCreate {
                            request_ulid: req.request_ulid.clone(),
                            status: CreateStatus::Pending,
                        });
                        let topic = format!("compute/create/{addr}");
                        let body = serde_json::to_string(&req).unwrap_or_default();
                        Task::perform(
                            async move {
                                let _ = std::process::Command::new("mde-bus")
                                    .args(["publish", &topic, "--body-flag", &body])
                                    .output();
                            },
                            |()| Message::Refresh,
                        )
                    }
                }
            }
            Message::CreateAckLoaded(opt) => {
                if let (Some(status), Some(pc)) = (opt, self.pending_create.as_mut()) {
                    pc.status = status;
                }
                Task::none()
            }
            Message::DismissCreate => {
                self.pending_create = None;
                Task::none()
            }
            Message::OpenSaveTemplate => {
                self.save_template_name = self.selected.as_ref().map(|d| d.name.clone());
                Task::none()
            }
            Message::SaveTemplateNameInput(s) => {
                if self.save_template_name.is_some() {
                    self.save_template_name = Some(s);
                }
                Task::none()
            }
            Message::CancelSaveTemplate => {
                self.save_template_name = None;
                Task::none()
            }
            Message::SubmitSaveTemplate => {
                let name = self
                    .save_template_name
                    .clone()
                    .filter(|n| !n.trim().is_empty());
                let (Some(name), Some(d)) = (name, self.selected.as_ref()) else {
                    return Task::none();
                };
                let vm_name = d.name.clone();
                let disk_path = d.disk_path.clone();
                let meshfs = d.meshfs_available;
                let tmpl_name = name.trim().to_string();
                self.save_template_name = None;
                Task::perform(
                    async move {
                        let (vcpus, ram_mb, disk_gb) = query_vm_config(&vm_name, &disk_path);
                        let t = Template {
                            name: tmpl_name,
                            vcpus,
                            ram_mb,
                            disk_gb,
                            share_meshfs: meshfs,
                        };
                        let _ = save_template(&t);
                    },
                    |()| Message::Refresh,
                )
            }
            Message::Console(name) => {
                let (prog, args) = console_command(&name);
                // Detached spawn — the console is a long-lived child window.
                let _ = std::process::Command::new(prog).args(&args).spawn();
                Task::none()
            }
            Message::SnapshotsLoaded(snaps) => {
                self.selected_snapshots = snaps;
                Task::none()
            }
            Message::ExposedLoaded(rules) => {
                self.exposed_rules = rules;
                Task::none()
            }
            Message::OpenExposeSheet => {
                self.expose_form = Some(ExposeForm {
                    mesh: true,
                    ..ExposeForm::default()
                });
                Task::none()
            }
            Message::CloseExposeSheet => {
                self.expose_form = None;
                Task::none()
            }
            Message::ExposePortInput(s) => {
                if let Some(form) = self.expose_form.as_mut() {
                    // Digits only, capped at 5 (a u16 fits in 5 digits).
                    form.guest_port = s.chars().filter(char::is_ascii_digit).take(5).collect();
                }
                Task::none()
            }
            Message::ExposeToggleProto => {
                if let Some(form) = self.expose_form.as_mut() {
                    form.proto_udp = !form.proto_udp;
                }
                Task::none()
            }
            Message::ExposeToggleNet(net) => {
                if let Some(form) = self.expose_form.as_mut() {
                    match net {
                        Network::Mesh => form.mesh = !form.mesh,
                        Network::Lan => form.lan = !form.lan,
                        Network::Wan => form.wan = !form.wan,
                    }
                }
                Task::none()
            }
            Message::SubmitExpose => {
                let built = self
                    .expose_form
                    .as_ref()
                    .zip(self.selected.as_ref())
                    .and_then(|(form, d)| {
                        build_expose_request(form, &d.nebula_ip).map(|r| (r, d.peer.clone()))
                    });
                let Some((req, peer)) = built else {
                    return Task::none(); // invalid form — keep the sheet open
                };
                self.expose_form = None;
                let topic = format!("compute/expose/{peer}");
                let body = serde_json::to_string(&req).unwrap_or_default();
                Task::perform(
                    async move {
                        let _ = std::process::Command::new("mde-bus")
                            .args(["publish", &topic, "--body-flag", &body])
                            .output();
                    },
                    |()| Message::Refresh,
                )
            }
            Message::Unexpose {
                vm_nebula_ip,
                host_port,
                proto,
            } => {
                let peer = self
                    .selected
                    .as_ref()
                    .map(|d| d.peer.clone())
                    .unwrap_or_default();
                let req = UnexposeRequest {
                    vm_nebula_ip,
                    host_port,
                    proto,
                };
                let topic = format!("compute/expose/{peer}");
                let body = serde_json::to_string(&req).unwrap_or_default();
                Task::perform(
                    async move {
                        let _ = std::process::Command::new("mde-bus")
                            .args(["publish", &topic, "--body-flag", &body])
                            .output();
                    },
                    |()| Message::Refresh,
                )
            }
            Message::OpenMigrateSheet => {
                self.migrate_open = true;
                Task::none()
            }
            Message::CloseMigrateSheet => {
                self.migrate_open = false;
                Task::none()
            }
            Message::MigrateTo(target_peer) => {
                self.migrate_open = false;
                let req = self.selected.as_ref().map(|d| MigrateRequest {
                    source_peer: d.peer.clone(),
                    target_peer,
                    vm_id: d.id.clone(),
                    disk_path: d.disk_path.clone(),
                });
                match req {
                    Some(req) => {
                        let body = serde_json::to_string(&req).unwrap_or_default();
                        Task::perform(
                            async move {
                                let _ = std::process::Command::new("mde-bus")
                                    .args([
                                        "publish",
                                        "action/compute/migrate",
                                        "--body-flag",
                                        &body,
                                    ])
                                    .output();
                            },
                            |()| Message::Refresh,
                        )
                    }
                    None => Task::none(),
                }
            }
            Message::TakeSnapshot(vm) => {
                let (prog, args) = snapshot_create_command(&vm);
                let prog = prog.to_string();
                Task::perform(
                    async move {
                        let _ = std::process::Command::new(prog).args(&args).output();
                        snapshot_list(&vm)
                    },
                    Message::SnapshotsLoaded,
                )
            }
            Message::DeleteSnapshot { vm, snap } => {
                let (prog, args) = snapshot_delete_command(&vm, &snap);
                let prog = prog.to_string();
                Task::perform(
                    async move {
                        let _ = std::process::Command::new(prog).args(&args).output();
                        snapshot_list(&vm)
                    },
                    Message::SnapshotsLoaded,
                )
            }
        }
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        let mut subs =
            vec![iced::time::every(Duration::from_secs(5)).map(|_| Message::Refresh)];
        // VIRT-21.b: on the Fleet tab, poll `compute/event/*` every 2 s so a
        // VM start/stop/crash refreshes the affected row promptly (≈2 s)
        // instead of waiting for the next 10 s inventory broadcast + 5 s poll.
        if matches!(self.tab, Tab::Fleet) {
            subs.push(
                iced::time::every(Duration::from_secs(2)).map(|_| Message::CheckComputeEvents),
            );
        }
        // While a local VM's detail panel is open, sample it every 2 s (17.b);
        // the sampler stops the moment the panel closes ("pause when hidden").
        if self.selected.as_ref().is_some_and(|d| d.is_local) {
            subs.push(iced::time::every(Duration::from_secs(2)).map(|_| Message::SampleTick));
        }
        Subscription::batch(subs)
    }

    pub(crate) fn theme(&self) -> iced::Theme {
        match self.tokens.theme {
            Theme::Dark => iced::Theme::Dark,
            Theme::Light => iced::Theme::Light,
        }
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        let tab_body = match self.tab {
            Tab::Fleet => self.fleet_view(),
            Tab::Local => self.local_view(),
        };
        let content: Element<'_, Message> = if let Some(w) = &self.wizard {
            row![
                container(tab_body).width(Length::FillPortion(3)).height(Length::Fill),
                w.view(&self.tokens).map(Message::Wizard),
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else if let Some(d) = &self.selected {
            row![
                container(tab_body).width(Length::FillPortion(3)).height(Length::Fill),
                self.detail_panel(d),
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else {
            tab_body
        };
        let inner = column![self.header_bar(), content]
            .width(Length::Fill)
            .height(Length::Fill);
        container(inner)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_t| container::Style {
                snap: false,
                background: Some(Background::Color(rgba(palette.background))),
                ..container::Style::default()
            })
            .into()
    }

    fn header_bar(&self) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let mut bar = row![
            text("Virtual")
                .size(TypeRole::Subheading.size_in(self.tokens.font_size))
                .color(rgba(palette.text)),
            Space::new().width(Length::Fill),
            self.tab_button("Fleet", Tab::Fleet),
            self.tab_button("Local", Tab::Local),
        ]
        .spacing(f32::from(space.sm))
        .padding([space.sm2, space.lg2])
        .align_y(iced::alignment::Vertical::Center);
        // `[+ New VM]` — Local tab only, and only when this peer's mded is
        // publishing inventory (so we have a create target).
        if self.tab == Tab::Local && self.local_peer_addr().is_some() {
            bar = bar.push(self.simple_button("+ New VM", Message::OpenWizard));
        }
        bar.into()
    }

    fn tab_button(&self, label: &str, tab: Tab) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let radius = f32::from(self.tokens.radii.input);
        let active = self.tab == tab;
        button(text(label.to_string()).size(TypeRole::Body.size_in(self.tokens.font_size)))
            .on_press(Message::SwitchTab(tab))
            .padding([space.xs, space.sm2])
            .style(move |_t, _status| {
                let (bg, fg, border) = if active {
                    (palette.accent, palette.background, palette.accent)
                } else {
                    (palette.surface, palette.text, palette.border)
                };
                button::Style {
                    background: Some(Background::Color(rgba(bg))),
                    text_color: rgba(fg),
                    border: Border {
                        color: rgba(border),
                        width: 1.0,
                        radius: radius.into(),
                    },
                    ..button::Style::default()
                }
            })
            .into()
    }

    fn fleet_view(&self) -> Element<'_, Message> {
        if self.fleet.is_unavailable() {
            return self.banner("Mesh unavailable");
        }
        let invs = self.fleet.inventories();
        if invs.is_empty() {
            return self.empty_state("No compute discovered on the mesh.");
        }
        self.peer_list(invs.iter(), false)
    }

    /// This peer's own Nebula overlay addr (from its inventory) — the
    /// `compute/create/<addr>` target. `None` when the local mded isn't
    /// publishing inventory (then `[+ New VM]` is hidden).
    fn local_peer_addr(&self) -> Option<String> {
        self.fleet
            .inventories()
            .iter()
            .find(|i| is_local(i, &self.local_host))
            .map(|i| i.peer.clone())
    }

    fn local_view(&self) -> Element<'_, Message> {
        let space = self.tokens.space;
        let local_inv: Option<&Inventory> = match &self.fleet {
            FleetState::Available(invs) => invs.iter().find(|i| is_local(i, &self.local_host)),
            FleetState::Unavailable => self.local_direct.as_ref(),
        };
        let mut sections: Vec<Element<'_, Message>> = Vec::new();
        match local_inv {
            Some(inv) => sections.push(self.peer_section(inv, true)),
            None => sections.push(self.section_note("No local VMs or containers.")),
        }
        // VIRT-19.a: read-only Images + Volumes browse sections.
        sections.push(self.images_section());
        sections.push(self.volumes_section());
        let list: Element<'_, Message> = scrollable(
            column(sections)
                .spacing(f32::from(space.sm))
                .padding([space.sm, space.lg2])
                .width(Length::Fill),
        )
        .height(Length::Fill)
        .into();
        let mut banners: Vec<Element<'_, Message>> = Vec::new();
        if let Some(b) = self.create_status_banner() {
            banners.push(b);
        }
        if let Some(b) = self.container_delete_banner() {
            banners.push(b);
        }
        if let Some(b) = self.pod_delete_banner() {
            banners.push(b);
        }
        if let Some(b) = self.image_delete_banner() {
            banners.push(b);
        }
        if let Some(b) = self.bulk_confirm_banner() {
            banners.push(b);
        }
        let action_bar = self.bulk_action_bar();
        if banners.is_empty() && action_bar.is_none() {
            list
        } else {
            let mut col = column![].spacing(f32::from(space.sm)).height(Length::Fill);
            for b in banners {
                col = col.push(b);
            }
            col = col.push(list);
            if let Some(bar) = action_bar {
                col = col.push(bar);
            }
            col.into()
        }
    }

    /// VIRT-20: the contextual bulk-action bar at the bottom of the Local
    /// tab, shown when ≥1 row is selected. Offers the verbs valid across the
    /// selection (Start all / Stop all) + a Clear.
    fn bulk_action_bar(&self) -> Option<Element<'_, Message>> {
        if self.selected_bulk.is_empty() {
            return None;
        }
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let kinds: Vec<ResourceKind> = bulk_targets(&self.selected_bulk, &self.local_rows())
            .iter()
            .map(|(k, _)| *k)
            .collect();
        let mut bar = row![text(format!("{} selected", self.selected_bulk.len()))
            .size(TypeRole::Caption.size_in(self.tokens.font_size))
            .color(rgba(palette.text_muted))
            .width(Length::Fill)]
        .spacing(f32::from(space.sm))
        .align_y(iced::alignment::Vertical::Center);
        for verb in bulk_actions_for(&kinds) {
            let label = if matches!(verb, ActionVerb::Start) {
                "Start all"
            } else {
                "Stop all"
            };
            bar = bar.push(self.simple_button(label, Message::RequestBulk(verb)));
        }
        bar = bar.push(self.simple_button("Clear", Message::SelectAllBulk(false)));
        Some(
            container(bar)
                .width(Length::Fill)
                .padding([space.sm, space.lg2])
                .style(move |_t| container::Style {
                    snap: false,
                    background: Some(Background::Color(rgba(palette.raised))),
                    border: Border {
                        color: rgba(palette.border),
                        width: 1.0,
                        radius: 0.0.into(),
                    },
                    ..container::Style::default()
                })
                .into(),
        )
    }

    /// VIRT-20: the bulk-action confirmation banner — lists the selected
    /// resource names before executing.
    fn bulk_confirm_banner(&self) -> Option<Element<'_, Message>> {
        let verb = self.pending_bulk?;
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let radius = f32::from(self.tokens.radii.md);
        let names: Vec<String> = bulk_targets(&self.selected_bulk, &self.local_rows())
            .into_iter()
            .map(|(_, n)| n)
            .collect();
        let verb_label = if matches!(verb, ActionVerb::Start) {
            "Start"
        } else {
            "Stop"
        };
        let msg = format!("{verb_label} {} resource(s): {}?", names.len(), names.join(", "));
        Some(
            container(
                row![
                    text(msg)
                        .size(TypeRole::Body.size_in(self.tokens.font_size))
                        .color(rgba(palette.text))
                        .width(Length::Fill),
                    self.simple_button(verb_label, Message::ConfirmBulk),
                    self.simple_button("Cancel", Message::CancelBulk),
                ]
                .align_y(iced::alignment::Vertical::Center)
                .spacing(f32::from(space.sm)),
            )
            .width(Length::Fill)
            .padding([space.sm, space.lg2])
            .style(move |_t| container::Style {
                snap: false,
                background: Some(Background::Color(rgba(palette.surface))),
                border: Border {
                    color: rgba(palette.border),
                    width: 1.0,
                    radius: radius.into(),
                },
                ..container::Style::default()
            })
            .into(),
        )
    }

    /// This peer's containers (Bus inventory, or the direct local read when
    /// the mesh is down) — for the VIRT-19.b image in-use guard.
    fn local_containers(&self) -> Vec<ContainerEntry> {
        match &self.fleet {
            FleetState::Available(invs) => invs
                .iter()
                .find(|i| is_local(i, &self.local_host))
                .map(|i| i.containers.clone())
                .unwrap_or_default(),
            FleetState::Unavailable => self
                .local_direct
                .as_ref()
                .map(|i| i.containers.clone())
                .unwrap_or_default(),
        }
    }

    /// This peer's display rows (VMs + containers), or empty when there's no
    /// local inventory. For VIRT-20 bulk select-all + dispatch.
    fn local_rows(&self) -> Vec<ResourceRow> {
        let inv = match &self.fleet {
            FleetState::Available(invs) => invs.iter().find(|i| is_local(i, &self.local_host)),
            FleetState::Unavailable => self.local_direct.as_ref(),
        };
        inv.map(rows_for).unwrap_or_default()
    }

    /// VIRT-19.b: the image delete-confirmation banner. When the image is in
    /// use by a running container it refuses inline rather than confirming.
    fn image_delete_banner(&self) -> Option<Element<'_, Message>> {
        let image_ref = self.pending_image_delete.as_ref()?;
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let radius = f32::from(self.tokens.radii.md);
        let in_use = image_in_use(image_ref, &self.local_containers());
        let msg = if in_use {
            format!("Can't delete {image_ref}: in use by a running container.")
        } else {
            format!("Delete image {image_ref}? (`podman rmi`)")
        };
        let mut bar = row![text(msg)
            .size(TypeRole::Body.size_in(self.tokens.font_size))
            .color(rgba(palette.text))
            .width(Length::Fill)]
        .align_y(iced::alignment::Vertical::Center)
        .spacing(f32::from(space.sm));
        if !in_use {
            bar = bar.push(self.simple_button("Delete", Message::ConfirmImageDelete));
        }
        bar = bar.push(self.simple_button(
            if in_use { "OK" } else { "Cancel" },
            Message::CancelImageDelete,
        ));
        Some(
            container(bar)
                .width(Length::Fill)
                .padding([space.sm, space.lg2])
                .style(move |_t| container::Style {
                    snap: false,
                    background: Some(Background::Color(rgba(palette.surface))),
                    border: Border {
                        color: rgba(palette.border),
                        width: 1.0,
                        radius: radius.into(),
                    },
                    ..container::Style::default()
                })
                .into(),
        )
    }

    /// VIRT-18.b.3: the pod delete-confirmation banner (removes the pod and
    /// all its containers). `None` when nothing is awaiting confirmation.
    fn pod_delete_banner(&self) -> Option<Element<'_, Message>> {
        let pod = self.pending_pod_delete.as_ref()?;
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let radius = f32::from(self.tokens.radii.md);
        Some(
            container(
                row![
                    text(format!(
                        "Delete pod {pod} and all its containers? (`podman pod rm -f`)"
                    ))
                    .size(TypeRole::Body.size_in(self.tokens.font_size))
                    .color(rgba(palette.text))
                    .width(Length::Fill),
                    self.simple_button("Delete", Message::ConfirmPodDelete),
                    self.simple_button("Cancel", Message::CancelPodDelete),
                ]
                .align_y(iced::alignment::Vertical::Center)
                .spacing(f32::from(space.sm)),
            )
            .width(Length::Fill)
            .padding([space.sm, space.lg2])
            .style(move |_t| container::Style {
                snap: false,
                background: Some(Background::Color(rgba(palette.surface))),
                border: Border {
                    color: rgba(palette.border),
                    width: 1.0,
                    radius: radius.into(),
                },
                ..container::Style::default()
            })
            .into(),
        )
    }

    /// VIRT-18.b.2: the container delete-confirmation banner, shown while a
    /// delete is pending. `None` when nothing is awaiting confirmation.
    fn container_delete_banner(&self) -> Option<Element<'_, Message>> {
        let name = self.pending_container_delete.as_ref()?;
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let radius = f32::from(self.tokens.radii.md);
        Some(
            container(
                row![
                    text(format!(
                        "Delete container {name}? It is force-removed with `podman rm -f`."
                    ))
                    .size(TypeRole::Body.size_in(self.tokens.font_size))
                    .color(rgba(palette.text))
                    .width(Length::Fill),
                    self.simple_button("Delete", Message::ConfirmContainerDelete),
                    self.simple_button("Cancel", Message::CancelContainerDelete),
                ]
                .align_y(iced::alignment::Vertical::Center)
                .spacing(f32::from(space.sm)),
            )
            .width(Length::Fill)
            .padding([space.sm, space.lg2])
            .style(move |_t| container::Style {
                snap: false,
                background: Some(Background::Color(rgba(palette.surface))),
                border: Border {
                    color: rgba(palette.border),
                    width: 1.0,
                    radius: radius.into(),
                },
                ..container::Style::default()
            })
            .into(),
        )
    }

    /// A muted one-line note used in place of an empty section body.
    fn section_note(&self, msg: &str) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        text(msg.to_string())
            .size(TypeRole::Body.size_in(self.tokens.font_size))
            .color(rgba(palette.text_muted))
            .into()
    }

    /// A titled, bordered section card wrapping `body` (VIRT-19.a).
    fn titled_card<'a>(&self, title: &str, body: Element<'a, Message>) -> Element<'a, Message> {
        let space = self.tokens.space;
        let palette = self.tokens.palette;
        let radius = f32::from(self.tokens.radii.md);
        let heading = text(title.to_string())
            .size(TypeRole::Subheading.size_in(self.tokens.font_size))
            .color(rgba(palette.text));
        container(column![heading, body].spacing(f32::from(space.xs)))
            .width(Length::Fill)
            .padding([space.sm, space.sm2])
            .style(move |_t| container::Style {
                snap: false,
                background: Some(Background::Color(rgba(palette.surface))),
                border: Border {
                    color: rgba(palette.border),
                    width: 1.0,
                    radius: radius.into(),
                },
                ..container::Style::default()
            })
            .into()
    }

    /// VIRT-19.a: read-only Images list (repository:tag, size, created).
    fn images_section(&self) -> Element<'_, Message> {
        let space = self.tokens.space;
        let palette = self.tokens.palette;
        let fs = self.tokens.font_size;
        let body: Element<'_, Message> = if self.images.is_empty() {
            self.section_note("No local images.")
        } else {
            let mut rows = column![].spacing(f32::from(space.xs));
            for img in &self.images {
                let image_ref = format!("{}:{}", img.repository, img.tag);
                rows = rows.push(
                    row![
                        text(image_ref.clone())
                            .size(TypeRole::Body.size_in(fs))
                            .color(rgba(palette.text))
                            .width(Length::FillPortion(3)),
                        text(img.size.clone())
                            .size(TypeRole::Caption.size_in(fs))
                            .color(rgba(palette.text_muted))
                            .width(Length::FillPortion(1)),
                        text(img.created.clone())
                            .size(TypeRole::Caption.size_in(fs))
                            .color(rgba(palette.text_muted))
                            .width(Length::FillPortion(2)),
                    ]
                    .push(self.pod_button("Delete", Message::RequestImageDelete(image_ref)))
                    .spacing(f32::from(space.sm)),
                );
            }
            rows.into()
        };
        self.titled_card("Images", body)
    }

    /// VIRT-19.a: read-only Volumes list (name, mount point, driver).
    fn volumes_section(&self) -> Element<'_, Message> {
        let space = self.tokens.space;
        let palette = self.tokens.palette;
        let fs = self.tokens.font_size;
        let body: Element<'_, Message> = if self.volumes.is_empty() {
            self.section_note("No local volumes.")
        } else {
            let mut rows = column![].spacing(f32::from(space.xs));
            for vol in &self.volumes {
                rows = rows.push(
                    row![
                        text(vol.name.clone())
                            .size(TypeRole::Body.size_in(fs))
                            .color(rgba(palette.text))
                            .width(Length::FillPortion(2)),
                        text(vol.mountpoint.clone())
                            .size(TypeRole::Caption.size_in(fs))
                            .color(rgba(palette.text_muted))
                            .width(Length::FillPortion(3)),
                        text(vol.driver.clone())
                            .size(TypeRole::Caption.size_in(fs))
                            .color(rgba(palette.text_muted))
                            .width(Length::FillPortion(1)),
                    ]
                    .spacing(f32::from(space.sm)),
                );
            }
            rows.into()
        };
        self.titled_card("Volumes", body)
    }

    /// The VM-creation status banner (VIRT-15.b), shown while a create is
    /// in flight or just resolved. `None` when no create is tracked.
    fn create_status_banner(&self) -> Option<Element<'_, Message>> {
        let pc = self.pending_create.as_ref()?;
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let radius = f32::from(self.tokens.radii.md);
        let (msg, fg) = match &pc.status {
            CreateStatus::Pending => ("Creating VM…".to_string(), palette.text_muted),
            CreateStatus::Done {
                vm_id,
                nebula_ip,
                meshfs_skipped,
            } => {
                let ip = if nebula_ip.is_empty() {
                    String::new()
                } else {
                    format!(" ({nebula_ip})")
                };
                let note = if *meshfs_skipped { " — MeshFS skipped" } else { "" };
                (format!("Created {vm_id}{ip}{note}"), palette.accent)
            }
            CreateStatus::Failed { error } => (format!("Create failed: {error}"), palette.text),
        };
        Some(
            container(
                row![
                    text(msg)
                        .size(TypeRole::Body.size_in(self.tokens.font_size))
                        .color(rgba(fg))
                        .width(Length::Fill),
                    self.simple_button("\u{00d7}", Message::DismissCreate),
                ]
                .align_y(iced::alignment::Vertical::Center)
                .spacing(f32::from(space.sm)),
            )
            .width(Length::Fill)
            .padding([space.sm, space.lg2])
            .style(move |_t| container::Style {
                snap: false,
                background: Some(Background::Color(rgba(palette.surface))),
                border: Border {
                    color: rgba(palette.border),
                    width: 1.0,
                    radius: radius.into(),
                },
                ..container::Style::default()
            })
            .into(),
        )
    }

    fn peer_list<'a, I>(&'a self, invs: I, show_actions: bool) -> Element<'a, Message>
    where
        I: Iterator<Item = &'a Inventory>,
    {
        let space = self.tokens.space;
        let sections: Vec<Element<'a, Message>> =
            invs.map(|inv| self.peer_section(inv, show_actions)).collect();
        scrollable(
            column(sections)
                .spacing(f32::from(space.sm))
                .padding([space.sm, space.lg2])
                .width(Length::Fill),
        )
        .height(Length::Fill)
        .into()
    }

    fn peer_section<'a>(&'a self, inv: &'a Inventory, show_actions: bool) -> Element<'a, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let local = is_local(inv, &self.local_host);
        let expanded = self.expanded.get(&inv.peer).copied().unwrap_or(true);
        let chevron = if expanded { "\u{25be}" } else { "\u{25b8}" }; // ▾ / ▸
        let host_label = if inv.hostname.is_empty() {
            inv.peer.clone()
        } else {
            inv.hostname.clone()
        };

        let header = button(
            row![
                text(chevron)
                    .size(TypeRole::Body.size_in(self.tokens.font_size))
                    .color(rgba(palette.text_muted)),
                text(host_label)
                    .size(TypeRole::Subheading.size_in(self.tokens.font_size))
                    .color(rgba(palette.text)),
                Space::new().width(Length::Fill),
                text(inv.peer.clone())
                    .size(TypeRole::Caption.size_in(self.tokens.font_size))
                    .color(rgba(palette.text_muted)),
            ]
            .spacing(f32::from(space.sm))
            .align_y(iced::alignment::Vertical::Center),
        )
        .on_press(Message::TogglePeer(inv.peer.clone()))
        .width(Length::Fill)
        .padding([space.sm, space.md])
        .style(move |_t, _status| button::Style {
            background: Some(Background::Color(rgba(palette.surface))),
            text_color: rgba(palette.text),
            border: Border {
                color: rgba(palette.border),
                width: 1.0,
                radius: 0.0.into(),
            },
            ..button::Style::default()
        });

        let mut col = column![header].width(Length::Fill);
        if expanded {
            let rows = rows_for(inv);
            if rows.is_empty() {
                col = col.push(self.muted_line("No VMs or containers."));
            } else {
                // VIRT-20: a select-all checkbox over this section's rows (Local).
                if show_actions {
                    let all_keys: Vec<String> =
                        rows.iter().map(|r| row_key(r.kind, &r.name)).collect();
                    let all_selected =
                        !all_keys.is_empty() && all_keys.iter().all(|k| self.selected_bulk.contains(k));
                    col = col.push(
                        container(
                            checkbox(all_selected)
                                .label("Select all")
                                .on_toggle(Message::SelectAllBulk),
                        )
                        .padding([space.xs, space.md]),
                    );
                }
                // VIRT-18.b: VMs + standalone containers render flat; pods
                // get a header with their child containers indented beneath.
                let (flat, pods) = group_by_pod(&rows);
                for r in &flat {
                    col = col.push(self.resource_row(r, show_actions, local, &inv.peer));
                }
                for (pod, children) in &pods {
                    col = col.push(self.pod_header_row(pod, show_actions));
                    for r in children {
                        col = col.push(row![
                            Space::new().width(Length::Fixed(f32::from(space.lg2))),
                            self.resource_row(r, show_actions, local, &inv.peer),
                        ]);
                    }
                }
            }
        }
        container(col).width(Length::Fill).into()
    }

    /// VIRT-18.b: a pod grouping header above its indented child containers.
    /// On the Local tab (`show_actions`) it carries pod-level Start/Stop/
    /// Delete buttons applying to every child container (VIRT-18.b.3).
    fn pod_header_row(&self, pod: &str, show_actions: bool) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let mut header = row![text(format!("\u{25be} Pod: {pod}"))
            .size(TypeRole::Caption.size_in(self.tokens.font_size))
            .color(rgba(palette.text_muted))
            .width(Length::Fill)]
        .spacing(f32::from(space.sm))
        .align_y(iced::alignment::Vertical::Center);
        if show_actions {
            header = header
                .push(self.pod_button(
                    "Start",
                    Message::PodLifecycle { pod: pod.to_string(), start: true },
                ))
                .push(self.pod_button(
                    "Stop",
                    Message::PodLifecycle { pod: pod.to_string(), start: false },
                ))
                .push(self.pod_button("Delete", Message::RequestPodDelete(pod.to_string())));
        }
        container(header)
            .width(Length::Fill)
            .padding([space.xs, space.md])
            .into()
    }

    /// A small pod-level action button (VIRT-18.b.3).
    fn pod_button(&self, label: &str, msg: Message) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let radius = f32::from(self.tokens.radii.sm);
        button(text(label.to_string()).size(TypeRole::Caption.size_in(self.tokens.font_size)))
            .on_press(msg)
            .padding([space.xs2, space.xs])
            .style(move |_t, _status| button::Style {
                background: Some(Background::Color(rgba(palette.surface))),
                text_color: rgba(palette.accent),
                border: Border {
                    color: rgba(palette.border),
                    width: 1.0,
                    radius: radius.into(),
                },
                ..button::Style::default()
            })
            .into()
    }

    fn resource_row(
        &self,
        r: &ResourceRow,
        show_actions: bool,
        is_local: bool,
        peer: &str,
    ) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        // Context column: VMs show their Nebula IP, containers their image.
        let context = match r.kind {
            ResourceKind::Vm => {
                if r.nebula_ip.is_empty() {
                    "\u{2014}".to_string() // —
                } else {
                    r.nebula_ip.clone()
                }
            }
            ResourceKind::Container => {
                if r.image.is_empty() {
                    "\u{2014}".to_string()
                } else {
                    r.image.rsplit('/').next().unwrap_or(&r.image).to_string()
                }
            }
        };

        // VM names are clickable → open the detail panel; containers are
        // plain text (their management lands with VIRT-18).
        let name_el: Element<'_, Message> = if matches!(r.kind, ResourceKind::Vm) {
            let detail = VmDetail::from_row(r, peer, is_local);
            button(
                text(r.name.clone())
                    .size(TypeRole::Body.size_in(self.tokens.font_size))
                    .color(rgba(palette.accent)),
            )
            .on_press(Message::SelectVm(detail))
            .padding(0)
            .width(Length::FillPortion(4))
            .style(|_t, _s| button::Style {
                background: None,
                ..button::Style::default()
            })
            .into()
        } else {
            text(r.name.clone())
                .size(TypeRole::Body.size_in(self.tokens.font_size))
                .color(rgba(palette.text))
                .width(Length::FillPortion(4))
                .into()
        };

        let mut widget = row![
            name_el,
            self.type_badge(r.kind),
            self.state_badge(&r.state),
            text(format!("{:.0}%", r.cpu_pct))
                .size(TypeRole::Caption.size_in(self.tokens.font_size))
                .color(rgba(palette.text_muted))
                .width(Length::FillPortion(2)),
            text(format!("{} MB", r.ram_mb))
                .size(TypeRole::Caption.size_in(self.tokens.font_size))
                .color(rgba(palette.text_muted))
                .width(Length::FillPortion(2)),
            text(context)
                .size(TypeRole::Caption.size_in(self.tokens.font_size))
                .color(rgba(palette.text_muted))
                .width(Length::FillPortion(3)),
        ]
        .spacing(f32::from(space.sm))
        .padding([space.xs, space.md])
        .align_y(iced::alignment::Vertical::Center);

        if show_actions {
            for verb in actions_for_state(&r.state) {
                widget = widget.push(self.action_button(r.kind, &r.name, verb, true));
            }
            // VIRT-18.b.2: containers get a confirmation-gated Delete.
            if matches!(r.kind, ResourceKind::Container) {
                widget = widget.push(self.delete_button(&r.name));
            }
            // VIRT-20: a bulk-selection checkbox leads the row on the Local tab.
            let key = row_key(r.kind, &r.name);
            let checked = self.selected_bulk.contains(&key);
            return row![
                checkbox(checked).on_toggle(move |_| Message::ToggleBulk(key.clone())),
                widget,
            ]
            .spacing(f32::from(space.sm))
            .align_y(iced::alignment::Vertical::Center)
            .into();
        }
        widget.into()
    }

    /// A container `[Delete]` button → opens the delete confirmation
    /// (VIRT-18.b.2). Styled like an action button.
    fn delete_button(&self, name: &str) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let radius = f32::from(self.tokens.radii.sm);
        let name = name.to_string();
        button(text("Delete").size(TypeRole::Caption.size_in(self.tokens.font_size)))
            .on_press(Message::RequestContainerDelete(name))
            .padding([space.xs2, space.xs])
            .style(move |_t, _status| button::Style {
                background: Some(Background::Color(rgba(palette.surface))),
                text_color: rgba(palette.text),
                border: Border {
                    color: rgba(palette.border),
                    width: 1.0,
                    radius: radius.into(),
                },
                ..button::Style::default()
            })
            .into()
    }

    /// A lifecycle action button. When `enabled` is false the button
    /// renders greyed with no `on_press` (iced's disabled state).
    fn action_button(
        &self,
        kind: ResourceKind,
        name: &str,
        verb: ActionVerb,
        enabled: bool,
    ) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let radius = f32::from(self.tokens.radii.sm);
        let name = name.to_string();
        let mut b = button(text(verb.label()).size(TypeRole::Caption.size_in(self.tokens.font_size)))
            .padding([space.xs2, space.xs])
            .style(move |_t, _status| button::Style {
                background: Some(Background::Color(rgba(palette.surface))),
                text_color: if enabled {
                    rgba(palette.accent)
                } else {
                    rgba(palette.text_muted)
                },
                border: Border {
                    color: rgba(palette.border),
                    width: 1.0,
                    radius: radius.into(),
                },
                ..button::Style::default()
            });
        if enabled {
            b = b.on_press(Message::Action { kind, name, verb });
        }
        b.into()
    }

    fn detail_panel<'a>(&'a self, d: &'a VmDetail) -> Element<'a, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let radius = f32::from(self.tokens.radii.md);

        let title = row![
            text(d.name.clone())
                .size(TypeRole::Subheading.size_in(self.tokens.font_size))
                .color(rgba(palette.text)),
            Space::new().width(Length::Fill),
            button(text("\u{00d7}").size(TypeRole::Subheading.size_in(self.tokens.font_size))) // ×
                .on_press(Message::CloseDetail)
                .padding([space.xs2, space.xs])
                .style(move |_t, _s| button::Style {
                    background: None,
                    text_color: rgba(palette.text_muted),
                    ..button::Style::default()
                }),
        ]
        .align_y(iced::alignment::Vertical::Center);

        let meshfs = if d.meshfs_available {
            self.badge_chip("\u{2713} MeshFS", palette.accent, palette.surface) // ✓
        } else {
            self.badge_chip("\u{26a0} MeshFS offline", palette.text_muted, palette.surface) // ⚠
        };

        let mut col = column![
            title,
            row![self.state_badge(&d.state), meshfs].spacing(f32::from(space.sm)),
            self.detail_kv("CPU", &format!("{:.0}%", d.cpu_pct)),
            self.detail_kv("RAM", &format!("{} MB", d.ram_mb)),
            self.detail_kv(
                "Disk",
                if d.disk_path.is_empty() { "\u{2014}" } else { &d.disk_path }
            ),
            self.detail_kv(
                "Nebula IP",
                if d.nebula_ip.is_empty() { "\u{2014}" } else { &d.nebula_ip }
            ),
        ]
        .spacing(f32::from(space.sm))
        .width(Length::Fill);

        // Live CPU/RAM sparklines (local VMs only) — VIRT-17.a.
        if d.is_local {
            let palette = self.tokens.palette;
            col = col.push(self.spark_row("CPU %", &self.cpu_history, palette.accent));
            col = col.push(self.spark_row("RAM (MB)", &self.ram_history, palette.text_muted));
        }

        // Action buttons — enabled only on local VMs whose state allows
        // the verb (§13 M4: remote VMs are read-only).
        let mut actions = row![].spacing(f32::from(space.xs));
        for verb in DETAIL_ACTIONS {
            let enabled = d.is_local && verb_applies(verb, &d.state);
            actions = actions.push(self.action_button(ResourceKind::Vm, &d.name, verb, enabled));
        }
        col = col.push(actions);

        // Console — local VMs only.
        let mut console = button(
            text("Console").size(TypeRole::Caption.size_in(self.tokens.font_size)),
        )
        .padding([space.xs2, space.xs])
        .style(move |_t, _s| button::Style {
            background: Some(Background::Color(rgba(palette.surface))),
            text_color: if d.is_local {
                rgba(palette.accent)
            } else {
                rgba(palette.text_muted)
            },
            border: Border {
                color: rgba(palette.border),
                width: 1.0,
                radius: f32::from(self.tokens.radii.sm).into(),
            },
            ..button::Style::default()
        });
        if d.is_local {
            console = console.on_press(Message::Console(d.name.clone()));
        }
        col = col.push(console);

        // Exposed ports (any VM — exposures are mesh-readable) — VIRT-14.c.
        col = col.push(self.exposed_section(d));

        // Snapshots + migrate + save-template (local VMs only).
        if d.is_local {
            col = col.push(self.snapshot_section(d));
            col = col.push(self.migrate_section(d));
            col = col.push(self.save_template_section());
        }

        container(col)
            .width(Length::FillPortion(2))
            .height(Length::Fill)
            .padding([space.md, space.lg2])
            .style(move |_t| container::Style {
                snap: false,
                background: Some(Background::Color(rgba(palette.surface))),
                border: Border {
                    color: rgba(palette.border),
                    width: 1.0,
                    radius: radius.into(),
                },
                ..container::Style::default()
            })
            .into()
    }

    fn detail_kv(&self, label: &str, value: &str) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        row![
            text(label.to_string())
                .size(TypeRole::Caption.size_in(self.tokens.font_size))
                .color(rgba(palette.text_muted))
                .width(Length::FillPortion(2)),
            text(value.to_string())
                .size(TypeRole::Body.size_in(self.tokens.font_size))
                .color(rgba(palette.text))
                .width(Length::FillPortion(3)),
        ]
        .spacing(f32::from(space.sm))
        .into()
    }

    fn spark_row(&self, label: &str, history: &VecDeque<f32>, color: Rgba) -> Element<'_, Message> {
        let space = self.tokens.space;
        let palette = self.tokens.palette;
        column![
            text(label.to_string())
                .size(TypeRole::Caption.size_in(self.tokens.font_size))
                .color(rgba(palette.text_muted)),
            sparkline::sparkline(history.iter().copied().collect(), rgba(color), 36.0),
        ]
        .spacing(f32::from(space.xs2))
        .width(Length::Fill)
        .into()
    }

    fn exposed_section<'a>(&'a self, d: &'a VmDetail) -> Element<'a, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let mut col = column![text("Exposed ports")
            .size(TypeRole::Subheading.size_in(self.tokens.font_size))
            .color(rgba(palette.text))]
        .spacing(f32::from(space.xs))
        .width(Length::Fill);
        if self.exposed_rules.is_empty() {
            col = col.push(self.muted_line("No exposed ports."));
        } else {
            for r in &self.exposed_rules {
                let mut rule_row = row![text(format!(
                    "{}: {}/{}",
                    r.network.label(),
                    r.host_port,
                    r.proto
                ))
                .size(TypeRole::Body.size_in(self.tokens.font_size))
                .color(rgba(palette.text))
                .width(Length::Fill)]
                .spacing(f32::from(space.sm))
                .align_y(iced::alignment::Vertical::Center);
                if d.is_local {
                    rule_row = rule_row.push(self.simple_button(
                        "\u{00d7}", // ×
                        Message::Unexpose {
                            vm_nebula_ip: d.nebula_ip.clone(),
                            host_port: r.host_port,
                            proto: r.proto.clone(),
                        },
                    ));
                }
                col = col.push(rule_row);
            }
        }
        // Expose controls — local VMs only (remote read-only per M4).
        if d.is_local {
            col = match self.expose_form.as_ref() {
                Some(form) => col.push(self.expose_sheet(form)),
                None => col.push(self.simple_button("Expose port…", Message::OpenExposeSheet)),
            };
        }
        col.into()
    }

    fn expose_sheet<'a>(&'a self, form: &'a ExposeForm) -> Element<'a, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let radius = f32::from(self.tokens.radii.md);
        let sm_radius = f32::from(self.tokens.radii.sm);
        let valid = self
            .selected
            .as_ref()
            .is_some_and(|d| build_expose_request(form, &d.nebula_ip).is_some());

        let port_input = text_input("guest port", &form.guest_port)
            .on_input(Message::ExposePortInput)
            .padding(f32::from(space.xs))
            .size(TypeRole::Body.size_in(self.tokens.font_size));

        let proto_label = if form.proto_udp {
            "Proto: UDP"
        } else {
            "Proto: TCP"
        };

        let nets = row![
            checkbox(form.mesh)
                .label("Mesh")
                .on_toggle(|_| Message::ExposeToggleNet(Network::Mesh)),
            checkbox(form.lan)
                .label("LAN")
                .on_toggle(|_| Message::ExposeToggleNet(Network::Lan)),
            checkbox(form.wan)
                .label("WAN")
                .on_toggle(|_| Message::ExposeToggleNet(Network::Wan)),
        ]
        .spacing(f32::from(space.sm));

        let mut submit =
            button(text("Expose").size(TypeRole::Caption.size_in(self.tokens.font_size)))
                .padding([space.xs2, space.xs])
                .style(move |_t, _s| button::Style {
                    background: Some(Background::Color(rgba(palette.surface))),
                    text_color: if valid {
                        rgba(palette.accent)
                    } else {
                        rgba(palette.text_muted)
                    },
                    border: Border {
                        color: rgba(palette.border),
                        width: 1.0,
                        radius: sm_radius.into(),
                    },
                    ..button::Style::default()
                });
        if valid {
            submit = submit.on_press(Message::SubmitExpose);
        }

        let mut col = column![
            port_input,
            self.simple_button(proto_label, Message::ExposeToggleProto),
            nets,
            row![submit, self.simple_button("Cancel", Message::CloseExposeSheet)]
                .spacing(f32::from(space.sm)),
        ]
        .spacing(f32::from(space.xs))
        .width(Length::Fill);
        if !valid {
            col = col.push(self.muted_line("Enter a port (1–65535) and pick at least one network."));
        }
        container(col)
            .padding(f32::from(space.sm))
            .style(move |_t| container::Style {
                snap: false,
                background: Some(Background::Color(rgba(palette.background))),
                border: Border {
                    color: rgba(palette.border),
                    width: 1.0,
                    radius: radius.into(),
                },
                ..container::Style::default()
            })
            .into()
    }

    fn snapshot_section<'a>(&'a self, d: &'a VmDetail) -> Element<'a, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let mut col = column![
            text("Snapshots")
                .size(TypeRole::Subheading.size_in(self.tokens.font_size))
                .color(rgba(palette.text)),
            self.simple_button("Take snapshot", Message::TakeSnapshot(d.name.clone())),
        ]
        .spacing(f32::from(space.xs))
        .width(Length::Fill);
        if self.selected_snapshots.is_empty() {
            col = col.push(self.muted_line("No snapshots."));
        } else {
            for snap in &self.selected_snapshots {
                col = col.push(
                    row![
                        text(snap.clone())
                            .size(TypeRole::Body.size_in(self.tokens.font_size))
                            .color(rgba(palette.text))
                            .width(Length::FillPortion(3)),
                        Space::new().width(Length::Fill),
                        self.simple_button(
                            "Delete",
                            Message::DeleteSnapshot {
                                vm: d.name.clone(),
                                snap: snap.clone(),
                            },
                        ),
                    ]
                    .spacing(f32::from(space.sm))
                    .align_y(iced::alignment::Vertical::Center),
                );
            }
        }
        col.into()
    }

    fn migrate_section<'a>(&'a self, d: &'a VmDetail) -> Element<'a, Message> {
        if !self.migrate_open {
            return self.simple_button("Migrate to…", Message::OpenMigrateSheet);
        }
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let targets = migrate_targets(self.fleet.inventories(), &d.peer);
        let mut col = column![text("Migrate to")
            .size(TypeRole::Subheading.size_in(self.tokens.font_size))
            .color(rgba(palette.text))]
        .spacing(f32::from(space.xs))
        .width(Length::Fill);
        if targets.is_empty() {
            col = col.push(self.muted_line("No other peers online."));
        } else {
            for (peer, label) in targets {
                col = col.push(self.simple_button(&label, Message::MigrateTo(peer)));
            }
        }
        col = col.push(self.simple_button("Cancel", Message::CloseMigrateSheet));
        col.into()
    }

    fn save_template_section(&self) -> Element<'_, Message> {
        let space = self.tokens.space;
        match &self.save_template_name {
            None => self.simple_button("Save as template…", Message::OpenSaveTemplate),
            Some(name) => column![
                text_input("template name", name)
                    .on_input(Message::SaveTemplateNameInput)
                    .padding(f32::from(space.xs))
                    .size(TypeRole::Caption.size_in(self.tokens.font_size)),
                row![
                    self.simple_button("Save", Message::SubmitSaveTemplate),
                    self.simple_button("Cancel", Message::CancelSaveTemplate),
                ]
                .spacing(f32::from(space.sm)),
            ]
            .spacing(f32::from(space.xs))
            .width(Length::Fill)
            .into(),
        }
    }

    fn simple_button(&self, label: &str, msg: Message) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let radius = f32::from(self.tokens.radii.sm);
        button(text(label.to_string()).size(TypeRole::Caption.size_in(self.tokens.font_size)))
            .on_press(msg)
            .padding([space.xs2, space.xs])
            .style(move |_t, _status| button::Style {
                background: Some(Background::Color(rgba(palette.surface))),
                text_color: rgba(palette.accent),
                border: Border {
                    color: rgba(palette.border),
                    width: 1.0,
                    radius: radius.into(),
                },
                ..button::Style::default()
            })
            .into()
    }

    fn type_badge(&self, kind: ResourceKind) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        self.badge_chip(kind.badge(), palette.text_muted, palette.overlay)
    }

    fn state_badge(&self, state: &str) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        let (fg, label) = if state_is_running(state) {
            (palette.accent, "running")
        } else if state_is_paused(state) {
            (palette.text_muted, "paused")
        } else {
            (palette.text_muted, "stopped")
        };
        self.badge_chip(label, fg, palette.surface)
    }

    fn badge_chip(&self, label: &str, fg: Rgba, bg: Rgba) -> Element<'_, Message> {
        let space = self.tokens.space;
        let radius = f32::from(self.tokens.radii.sm);
        container(
            text(label.to_string())
                .size(TypeRole::Caption.size_in(self.tokens.font_size))
                .color(rgba(fg)),
        )
        .padding([space.xs2, space.xs])
        .style(move |_t| container::Style {
            snap: false,
            background: Some(Background::Color(rgba(bg))),
            border: Border {
                color: rgba(bg),
                width: 1.0,
                radius: radius.into(),
            },
            ..container::Style::default()
        })
        .into()
    }

    fn banner(&self, msg: &str) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let radius = f32::from(self.tokens.radii.md);
        container(
            text(msg.to_string())
                .size(TypeRole::Body.size_in(self.tokens.font_size))
                .color(rgba(palette.text)),
        )
        .width(Length::Fill)
        .padding([space.sm, space.lg2])
        .style(move |_t| container::Style {
            snap: false,
            background: Some(Background::Color(rgba(palette.surface))),
            border: Border {
                color: rgba(palette.border),
                width: 1.0,
                radius: radius.into(),
            },
            ..container::Style::default()
        })
        .into()
    }

    fn empty_state(&self, msg: &str) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        container(
            text(msg.to_string())
                .size(TypeRole::Body.size_in(self.tokens.font_size))
                .color(rgba(palette.text_muted)),
        )
        .width(Length::Fill)
        .padding([space.lg2, space.lg2])
        .into()
    }

    fn muted_line(&self, msg: &str) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        container(
            text(msg.to_string())
                .size(TypeRole::Caption.size_in(self.tokens.font_size))
                .color(rgba(palette.text_muted)),
        )
        .padding([space.xs, space.md])
        .into()
    }
}

fn rgba(c: Rgba) -> Color {
    c.into_iced_color()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_json(peer: &str, host: &str) -> String {
        format!(
            r#"{{"peer":"{peer}","hostname":"{host}",
                 "vms":[{{"id":"u1","name":"web","state":"running",
                          "cpu_pct":12.5,"ram_mb":2048,"disk_path":"/var/lib/mde-vms/web.qcow2",
                          "nebula_ip":"10.42.0.5","meshfs_available":true}}],
                 "containers":[{{"id":"c1","name":"redis","state":"running",
                                 "image":"redis","cpu_pct":1.0,"ram_mb":64}}]}}"#
        )
    }

    #[test]
    fn newest_ulid_picks_lexical_max() {
        let ulids = vec![
            "01H000000000000000000AAAAA".to_string(),
            "01H000000000000000000CCCCC".to_string(),
            "01H000000000000000000BBBBB".to_string(),
        ];
        assert_eq!(
            newest_ulid(ulids.into_iter()),
            Some("01H000000000000000000CCCCC".to_string())
        );
        assert_eq!(newest_ulid(std::iter::empty()), None);
    }

    #[test]
    fn parse_podman_images_tolerates_shapes() {
        // Names array + int Size + CreatedAt string.
        let json = r#"[{"Names":["docker.io/library/redis:7"],"Size":1234567,"CreatedAt":"2024-01-02 03:04:05"}]"#;
        let imgs = parse_podman_images(json);
        assert_eq!(imgs.len(), 1);
        assert_eq!(imgs[0].repository, "docker.io/library/redis");
        assert_eq!(imgs[0].tag, "7");
        assert_eq!(imgs[0].size, "1.2 MB");
        assert_eq!(imgs[0].created, "2024-01-02 03:04:05");
        // Repository/Tag fallback + pre-formatted string Size.
        let json2 = r#"[{"Repository":"localhost/app","Tag":"dev","Size":"10 MB"}]"#;
        let imgs2 = parse_podman_images(json2);
        assert_eq!(imgs2[0].repository, "localhost/app");
        assert_eq!(imgs2[0].tag, "dev");
        assert_eq!(imgs2[0].size, "10 MB");
        // Bad JSON → empty.
        assert!(parse_podman_images("not json").is_empty());
    }

    #[test]
    fn parse_podman_volumes_reads_fields_and_drops_nameless() {
        let json = r#"[{"Name":"pgdata","Mountpoint":"/var/lib/containers/storage/volumes/pgdata/_data","Driver":"local"},
                       {"Name":""}]"#;
        let vols = parse_podman_volumes(json);
        assert_eq!(vols.len(), 1); // the empty-name row is dropped
        assert_eq!(vols[0].name, "pgdata");
        assert_eq!(vols[0].driver, "local");
        assert!(vols[0].mountpoint.ends_with("/pgdata/_data"));
    }

    #[test]
    fn human_size_scales() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2.0 KB");
        assert_eq!(human_size(1_500_000), "1.4 MB");
    }

    fn pod_test_row(name: &str, kind: ResourceKind, pod: &str) -> ResourceRow {
        ResourceRow {
            id: String::new(),
            name: name.to_string(),
            kind,
            state: "running".to_string(),
            cpu_pct: 0.0,
            ram_mb: 0,
            nebula_ip: String::new(),
            disk_path: String::new(),
            meshfs_available: false,
            image: String::new(),
            pod: pod.to_string(),
        }
    }

    #[test]
    fn group_by_pod_partitions_vms_and_pods() {
        let rows = vec![
            pod_test_row("web", ResourceKind::Vm, ""),
            pod_test_row("db", ResourceKind::Container, "app-pod"),
            pod_test_row("cache", ResourceKind::Container, "app-pod"),
            pod_test_row("standalone", ResourceKind::Container, ""),
        ];
        let (flat, pods) = group_by_pod(&rows);
        // VM + standalone container stay flat, in order.
        assert_eq!(flat.len(), 2);
        assert_eq!(flat[0].name, "web");
        assert_eq!(flat[1].name, "standalone");
        // One pod group carrying both members.
        assert_eq!(pods.len(), 1);
        assert_eq!(pods[0].0, "app-pod");
        assert_eq!(pods[0].1.len(), 2);
    }

    #[test]
    fn container_delete_command_is_rm_force() {
        let (bin, args) = container_delete_command("redis");
        assert_eq!(bin, "podman");
        assert_eq!(
            args,
            vec!["rm".to_string(), "-f".to_string(), "redis".to_string()]
        );
    }

    #[test]
    fn container_delete_confirmation_state() {
        let mut app = VirtualApp::new();
        assert!(app.pending_container_delete.is_none());
        let _ = app.update(Message::RequestContainerDelete("redis".into()));
        assert_eq!(app.pending_container_delete.as_deref(), Some("redis"));
        let _ = app.update(Message::CancelContainerDelete);
        assert!(app.pending_container_delete.is_none());
    }

    #[test]
    fn pod_commands_build_argv() {
        assert_eq!(
            pod_lifecycle_command("app", true),
            ("podman", vec!["pod".to_string(), "start".to_string(), "app".to_string()])
        );
        assert_eq!(
            pod_lifecycle_command("app", false),
            ("podman", vec!["pod".to_string(), "stop".to_string(), "app".to_string()])
        );
        assert_eq!(
            pod_delete_command("app"),
            (
                "podman",
                vec!["pod".to_string(), "rm".to_string(), "-f".to_string(), "app".to_string()]
            )
        );
    }

    #[test]
    fn pod_delete_confirmation_state() {
        let mut app = VirtualApp::new();
        let _ = app.update(Message::RequestPodDelete("app-pod".into()));
        assert_eq!(app.pending_pod_delete.as_deref(), Some("app-pod"));
        let _ = app.update(Message::CancelPodDelete);
        assert!(app.pending_pod_delete.is_none());
    }

    #[test]
    fn image_delete_command_is_rmi() {
        let (bin, args) = image_delete_command("redis:7");
        assert_eq!(bin, "podman");
        assert_eq!(args, vec!["rmi".to_string(), "redis:7".to_string()]);
    }

    #[test]
    fn image_in_use_detects_running_container() {
        let containers = vec![
            ContainerEntry {
                name: "c1".into(),
                image: "redis:7".into(),
                state: "running".into(),
                cpu_pct: 0.0,
                ram_mb: 0,
                pod: String::new(),
            },
            ContainerEntry {
                name: "c2".into(),
                image: "nginx:1".into(),
                state: "exited".into(),
                cpu_pct: 0.0,
                ram_mb: 0,
                pod: String::new(),
            },
        ];
        assert!(image_in_use("redis:7", &containers)); // running ⇒ in use
        assert!(!image_in_use("nginx:1", &containers)); // exited ⇒ not in use
        assert!(!image_in_use("absent:1", &containers));
    }

    #[test]
    fn image_delete_confirmation_state() {
        let mut app = VirtualApp::new();
        let _ = app.update(Message::RequestImageDelete("redis:7".into()));
        assert_eq!(app.pending_image_delete.as_deref(), Some("redis:7"));
        let _ = app.update(Message::CancelImageDelete);
        assert!(app.pending_image_delete.is_none());
    }

    #[test]
    fn bulk_actions_intersection_for_mixed_selection() {
        // VMs and containers both support Start/Stop, so a mixed selection
        // still exposes both.
        assert_eq!(
            bulk_actions_for(&[ResourceKind::Vm, ResourceKind::Container]),
            vec![ActionVerb::Start, ActionVerb::Stop]
        );
        assert_eq!(bulk_actions_for(&[]), Vec::<ActionVerb>::new());
    }

    #[test]
    fn bulk_targets_one_per_selected_row() {
        let rows = vec![
            pod_test_row("web", ResourceKind::Vm, ""),
            pod_test_row("db", ResourceKind::Container, ""),
            pod_test_row("cache", ResourceKind::Container, ""),
        ];
        let mut sel = std::collections::HashSet::new();
        sel.insert(row_key(ResourceKind::Vm, "web"));
        sel.insert(row_key(ResourceKind::Container, "cache"));
        let targets = bulk_targets(&sel, &rows);
        assert_eq!(targets.len(), 2); // one per selected; db excluded
        assert!(targets.iter().any(|(k, n)| *k == ResourceKind::Vm && n == "web"));
        assert!(targets
            .iter()
            .any(|(k, n)| *k == ResourceKind::Container && n == "cache"));
    }

    #[test]
    fn bulk_toggle_and_clear_state() {
        let mut app = VirtualApp::new();
        let _ = app.update(Message::ToggleBulk("vm:web".into()));
        assert!(app.selected_bulk.contains("vm:web"));
        let _ = app.update(Message::ToggleBulk("vm:web".into())); // toggle off
        assert!(!app.selected_bulk.contains("vm:web"));
        let _ = app.update(Message::ToggleBulk("ct:db".into()));
        let _ = app.update(Message::SelectAllBulk(false)); // clear
        assert!(app.selected_bulk.is_empty());
    }

    #[test]
    fn inventory_deserializes_from_published_json() {
        let inv: Inventory = serde_json::from_str(&sample_json("10.42.0.5", "alpha")).unwrap();
        assert_eq!(inv.peer, "10.42.0.5");
        assert_eq!(inv.hostname, "alpha");
        assert_eq!(inv.vms.len(), 1);
        assert_eq!(inv.vms[0].name, "web");
        assert_eq!(inv.vms[0].nebula_ip, "10.42.0.5");
        assert_eq!(inv.vms[0].disk_path, "/var/lib/mde-vms/web.qcow2");
        assert!(inv.vms[0].meshfs_available);
        assert_eq!(inv.containers.len(), 1);
        assert_eq!(inv.containers[0].name, "redis");
    }

    #[test]
    fn inventory_tolerates_missing_fields() {
        let inv: Inventory = serde_json::from_str(r#"{"peer":"10.42.0.9"}"#).unwrap();
        assert_eq!(inv.peer, "10.42.0.9");
        assert!(inv.hostname.is_empty());
        assert!(inv.vms.is_empty());
        assert!(inv.containers.is_empty());
    }

    #[test]
    fn rows_for_flattens_vms_then_containers_with_vm_fields() {
        let inv: Inventory = serde_json::from_str(&sample_json("p", "h")).unwrap();
        let rows = rows_for(&inv);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].kind, ResourceKind::Vm);
        assert_eq!(rows[0].id, "u1");
        assert_eq!(rows[0].nebula_ip, "10.42.0.5");
        assert_eq!(rows[0].disk_path, "/var/lib/mde-vms/web.qcow2");
        assert!(rows[0].meshfs_available);
        assert_eq!(rows[1].kind, ResourceKind::Container);
        assert!(rows[1].id.is_empty());
        assert_eq!(rows[1].image, "redis");
        assert!(rows[0].image.is_empty()); // VM row carries no image
        assert!(rows[1].nebula_ip.is_empty());
        assert!(rows[1].disk_path.is_empty());
        assert!(!rows[1].meshfs_available);
    }

    #[test]
    fn pick_latest_per_peer_keeps_newest_ulid() {
        let older: Inventory = serde_json::from_str(&sample_json("10.42.0.5", "alpha-old")).unwrap();
        let newer: Inventory = serde_json::from_str(&sample_json("10.42.0.5", "alpha-new")).unwrap();
        let entries = vec![
            ("01JAAAAAAAAAAAAAAAAAAAAAAA".to_string(), older),
            ("01JZZZZZZZZZZZZZZZZZZZZZZZ".to_string(), newer),
        ];
        let out = pick_latest_per_peer(entries);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].hostname, "alpha-new");
    }

    #[test]
    fn pick_latest_per_peer_groups_distinct_peers_sorted() {
        let a: Inventory = serde_json::from_str(&sample_json("10.42.0.6", "bravo")).unwrap();
        let b: Inventory = serde_json::from_str(&sample_json("10.42.0.5", "alpha")).unwrap();
        let out = pick_latest_per_peer(vec![("01A".into(), a), ("01B".into(), b)]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].hostname, "alpha");
        assert_eq!(out[1].hostname, "bravo");
    }

    #[test]
    fn is_local_matches_hostname_only() {
        let inv: Inventory = serde_json::from_str(&sample_json("p", "alpha")).unwrap();
        assert!(is_local(&inv, "alpha"));
        assert!(!is_local(&inv, "bravo"));
        assert!(!is_local(&inv, ""));
    }

    #[test]
    fn state_helpers_classify() {
        assert!(state_is_running("running"));
        assert!(state_is_running("Running"));
        assert!(!state_is_running("shut off"));
        assert!(state_is_paused("paused"));
        assert!(!state_is_paused("running"));
    }

    #[test]
    fn type_badge_labels() {
        assert_eq!(ResourceKind::Vm.badge(), "KVM");
        assert_eq!(ResourceKind::Container.badge(), "Podman");
    }

    #[test]
    fn collect_inventory_files_empty_on_missing_dir() {
        let entries =
            collect_inventory_files(Path::new("/nonexistent/mde/bus/compute/inventory"));
        assert!(entries.is_empty());
    }

    #[test]
    fn fleet_state_helpers() {
        let unavail = FleetState::Unavailable;
        assert!(unavail.is_unavailable());
        assert!(unavail.inventories().is_empty());
        let avail = FleetState::Available(vec![]);
        assert!(!avail.is_unavailable());
    }

    #[test]
    fn parse_virsh_list_state_parses_name_and_multiword_state() {
        let out = parse_virsh_list_state(
            " Id   Name      State\n\
             --------------------------\n\
             1     web       running\n\
             -     db        shut off\n",
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], ("web".to_string(), "running".to_string()));
        assert_eq!(out[1], ("db".to_string(), "shut off".to_string()));
    }

    #[test]
    fn parse_podman_ps_local_extracts_name_state() {
        let json = r#"[{"Id":"abc","Names":["redis"],"State":"running","Image":"docker.io/library/redis:7"},
                       {"Id":"def","Names":["pg"],"State":"exited","Image":"postgres:16"}]"#;
        let out = parse_podman_ps_local(json);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, "redis");
        assert_eq!(out[0].state, "running");
        assert_eq!(out[0].image, "docker.io/library/redis:7");
        assert_eq!(out[1].name, "pg");
        assert_eq!(out[1].state, "exited");
    }

    #[test]
    fn parse_podman_ps_local_empty_on_garbage() {
        assert!(parse_podman_ps_local("not json").is_empty());
        assert!(parse_podman_ps_local("[]").is_empty());
    }

    #[test]
    fn actions_for_state_is_contextual() {
        assert_eq!(actions_for_state("running"), vec![ActionVerb::Stop, ActionVerb::Suspend]);
        assert_eq!(actions_for_state("paused"), vec![ActionVerb::Resume]);
        assert_eq!(actions_for_state("shut off"), vec![ActionVerb::Start]);
    }

    #[test]
    fn command_for_builds_correct_argv() {
        let (p, a) = command_for(ResourceKind::Vm, ActionVerb::Stop, "web");
        assert_eq!(p, "virsh");
        assert_eq!(a, vec!["-c", "qemu:///system", "shutdown", "web"]);
        let (_, a) = command_for(ResourceKind::Vm, ActionVerb::ForceOff, "web");
        assert_eq!(a, vec!["-c", "qemu:///system", "destroy", "web"]);
        let (p, a) = command_for(ResourceKind::Container, ActionVerb::Start, "redis");
        assert_eq!(p, "podman");
        assert_eq!(a, vec!["start", "redis"]);
        let (_, a) = command_for(ResourceKind::Container, ActionVerb::ForceOff, "redis");
        assert_eq!(a, vec!["kill", "redis"]);
    }

    #[test]
    fn console_command_targets_system_libvirtd() {
        let (p, a) = console_command("web");
        assert_eq!(p, "virt-viewer");
        assert_eq!(a, vec!["--connect", "qemu:///system", "web"]);
    }

    #[test]
    fn verb_applies_by_state() {
        // Stopped VM: only Start applies.
        assert!(verb_applies(ActionVerb::Start, "shut off"));
        assert!(!verb_applies(ActionVerb::Stop, "shut off"));
        // Running VM: stop/force/suspend apply, start/resume don't.
        assert!(verb_applies(ActionVerb::Stop, "running"));
        assert!(verb_applies(ActionVerb::ForceOff, "running"));
        assert!(verb_applies(ActionVerb::Suspend, "running"));
        assert!(!verb_applies(ActionVerb::Start, "running"));
        assert!(!verb_applies(ActionVerb::Resume, "running"));
        // Paused VM: only Resume applies.
        assert!(verb_applies(ActionVerb::Resume, "paused"));
        assert!(!verb_applies(ActionVerb::Stop, "paused"));
    }

    #[test]
    fn vm_detail_from_row_carries_fields_and_locality() {
        let inv: Inventory = serde_json::from_str(&sample_json("10.42.0.5", "alpha")).unwrap();
        let rows = rows_for(&inv);
        let d = VmDetail::from_row(&rows[0], "10.42.0.1", true);
        assert_eq!(d.id, "u1");
        assert_eq!(d.name, "web");
        assert_eq!(d.disk_path, "/var/lib/mde-vms/web.qcow2");
        assert_eq!(d.nebula_ip, "10.42.0.5");
        assert_eq!(d.peer, "10.42.0.1");
        assert!(d.meshfs_available);
        assert!(d.is_local);
    }

    #[test]
    fn parse_virsh_snapshot_list_names_only() {
        let out = parse_virsh_snapshot_list("snap-1\nsnap-2\n\n");
        assert_eq!(out, vec!["snap-1".to_string(), "snap-2".to_string()]);
    }

    #[test]
    fn parse_virsh_snapshot_list_tolerates_table_form() {
        let out = parse_virsh_snapshot_list(
            " Name      Creation Time               State\n\
             ------------------------------------------------\n\
             snap-1    2026-05-31 10:00:00 +0000   shutoff\n",
        );
        assert_eq!(out, vec!["snap-1".to_string()]);
    }

    #[test]
    fn snapshot_commands_build_correct_argv() {
        let (p, a) = snapshot_create_command("web");
        assert_eq!(p, "virsh");
        assert_eq!(a, vec!["-c", "qemu:///system", "snapshot-create-as", "web"]);
        let (_, a) = snapshot_delete_command("web", "snap-1");
        assert_eq!(a, vec!["-c", "qemu:///system", "snapshot-delete", "web", "snap-1"]);
    }

    #[test]
    fn network_deserializes_lowercase_wire_names() {
        assert_eq!(serde_json::from_str::<Network>("\"mesh\"").unwrap(), Network::Mesh);
        assert_eq!(serde_json::from_str::<Network>("\"lan\"").unwrap(), Network::Lan);
        assert_eq!(serde_json::from_str::<Network>("\"wan\"").unwrap(), Network::Wan);
    }

    #[test]
    fn rules_for_vm_filters_by_overlay_ip() {
        let json = r#"{"peer":"10.42.0.1","rules":[
            {"network":"mesh","vm_nebula_ip":"10.42.128.5","host_port":8080,"proto":"tcp"},
            {"network":"lan","vm_nebula_ip":"10.42.128.9","host_port":443,"proto":"tcp"}
        ]}"#;
        let state: ExposedState = serde_json::from_str(json).unwrap();
        assert_eq!(state.peer, "10.42.0.1");
        let mine = rules_for_vm(vec![state], "10.42.128.5");
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].host_port, 8080);
        assert_eq!(mine[0].network, Network::Mesh);
        assert_eq!(mine[0].proto, "tcp");
    }

    #[test]
    fn rules_for_vm_empty_ip_matches_nothing() {
        let json =
            r#"{"peer":"p","rules":[{"network":"mesh","vm_nebula_ip":"","host_port":1,"proto":"tcp"}]}"#;
        let state: ExposedState = serde_json::from_str(json).unwrap();
        assert!(rules_for_vm(vec![state], "").is_empty());
    }

    #[test]
    fn build_expose_request_valid() {
        let form = ExposeForm {
            guest_port: "8080".to_string(),
            proto_udp: false,
            mesh: true,
            lan: true,
            wan: false,
        };
        let req = build_expose_request(&form, "10.42.128.5").unwrap();
        assert_eq!(req.vm_nebula_ip, "10.42.128.5");
        assert_eq!(req.guest_port, 8080);
        assert_eq!(req.proto, "tcp");
        assert_eq!(req.networks, vec![Network::Mesh, Network::Lan]);
    }

    #[test]
    fn build_expose_request_rejects_bad_input() {
        let no_net = ExposeForm {
            guest_port: "80".into(),
            ..Default::default()
        };
        assert!(build_expose_request(&no_net, "ip").is_none());
        let empty_port = ExposeForm {
            guest_port: String::new(),
            mesh: true,
            ..Default::default()
        };
        assert!(build_expose_request(&empty_port, "ip").is_none());
        let zero_port = ExposeForm {
            guest_port: "0".into(),
            mesh: true,
            ..Default::default()
        };
        assert!(build_expose_request(&zero_port, "ip").is_none());
    }

    #[test]
    fn expose_request_serializes_with_lowercase_networks() {
        let form = ExposeForm {
            guest_port: "443".into(),
            proto_udp: true,
            wan: true,
            ..Default::default()
        };
        let req = build_expose_request(&form, "10.42.128.9").unwrap();
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"guest_port\":443"));
        assert!(json.contains("\"proto\":\"udp\""));
        assert!(json.contains("\"wan\""));
    }

    #[test]
    fn unexpose_request_serializes() {
        let req = UnexposeRequest {
            vm_nebula_ip: "10.42.128.5".into(),
            host_port: 8080,
            proto: "tcp".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"host_port\":8080"));
        assert!(json.contains("10.42.128.5"));
    }

    #[test]
    fn migrate_targets_excludes_source() {
        let a: Inventory = serde_json::from_str(&sample_json("10.42.0.5", "alpha")).unwrap();
        let b: Inventory = serde_json::from_str(&sample_json("10.42.0.6", "bravo")).unwrap();
        let targets = migrate_targets(&[a, b], "10.42.0.5");
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].0, "10.42.0.6");
        assert_eq!(targets[0].1, "bravo");
    }

    #[test]
    fn migrate_request_serializes() {
        let req = MigrateRequest {
            source_peer: "10.42.0.5".into(),
            target_peer: "10.42.0.6".into(),
            vm_id: "u1".into(),
            disk_path: "/var/lib/mde-vms/web.qcow2".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"source_peer\":\"10.42.0.5\""));
        assert!(json.contains("\"target_peer\":\"10.42.0.6\""));
        assert!(json.contains("\"vm_id\":\"u1\""));
        assert!(json.contains("web.qcow2"));
    }

    #[test]
    fn create_ack_success_maps_to_done() {
        let ack: CreateAck =
            serde_json::from_str(r#"{"vm_id":"web-abc12345","nebula_ip":"10.42.128.5"}"#).unwrap();
        match status_from_ack(ack) {
            CreateStatus::Done {
                vm_id,
                nebula_ip,
                meshfs_skipped,
            } => {
                assert_eq!(vm_id, "web-abc12345");
                assert_eq!(nebula_ip, "10.42.128.5");
                assert!(!meshfs_skipped);
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn create_ack_error_maps_to_failed() {
        let ack: CreateAck = serde_json::from_str(r#"{"error":"no disk space"}"#).unwrap();
        assert_eq!(
            status_from_ack(ack),
            CreateStatus::Failed {
                error: "no disk space".to_string()
            }
        );
    }

    #[test]
    fn create_ack_meshfs_skipped_flag() {
        let ack: CreateAck =
            serde_json::from_str(r#"{"vm_id":"x","meshfs_skipped":true}"#).unwrap();
        match status_from_ack(ack) {
            CreateStatus::Done { meshfs_skipped, .. } => assert!(meshfs_skipped),
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn parse_dominfo_config_extracts_vcpus_and_ram() {
        let out = "Id:             3\n\
                   Name:           web\n\
                   CPU(s):         4\n\
                   Max memory:     4194304 KiB\n\
                   Used memory:    2097152 KiB\n";
        let (vcpus, ram_mb) = parse_dominfo_config(out);
        assert_eq!(vcpus, 4);
        assert_eq!(ram_mb, 4096); // 4194304 KiB / 1024
    }

    #[test]
    fn parse_blkinfo_capacity_gb_converts_bytes() {
        let out = "Capacity:       21474836480\n\
                   Allocation:     1234\n\
                   Physical:       21474836480\n";
        assert_eq!(parse_blkinfo_capacity_gb(out), 20); // 20 GiB
    }

    #[test]
    fn parse_domstats_vcpu_ns_sums_times() {
        let out = "Domain: 'web'\n\
                   vcpu.current=2\n\
                   vcpu.0.state=1\n\
                   vcpu.0.time=1000000000\n\
                   vcpu.1.state=1\n\
                   vcpu.1.time=500000000\n";
        assert_eq!(parse_domstats_vcpu_ns(out), 1_500_000_000);
    }

    #[test]
    fn parse_dommemstat_rss_to_mb() {
        let out = "actual 2097152\nrss 524288\navailable 2097152\n";
        assert_eq!(parse_dommemstat_rss_mb(out), 512); // 524288 KiB / 1024
    }
}
