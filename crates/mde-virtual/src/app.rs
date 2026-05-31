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

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Background, Border, Color, Element, Length, Subscription, Task};
use mde_theme::{Density, Rgba, Theme, Tokens, TypeRole};
use serde::Deserialize;

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
    pub state: String,
    #[serde(default)]
    pub cpu_pct: f64,
    #[serde(default)]
    pub ram_mb: u64,
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
}

/// Flatten an inventory's VMs (first) then containers into display rows.
pub(crate) fn rows_for(inv: &Inventory) -> Vec<ResourceRow> {
    let mut out = Vec::with_capacity(inv.vms.len() + inv.containers.len());
    for vm in &inv.vms {
        out.push(ResourceRow {
            name: vm.name.clone(),
            kind: ResourceKind::Vm,
            state: vm.state.clone(),
            cpu_pct: vm.cpu_pct,
            ram_mb: vm.ram_mb,
            nebula_ip: vm.nebula_ip.clone(),
            disk_path: vm.disk_path.clone(),
            meshfs_available: vm.meshfs_available,
        });
    }
    for c in &inv.containers {
        out.push(ResourceRow {
            name: c.name.clone(),
            kind: ResourceKind::Container,
            state: c.state.clone(),
            cpu_pct: c.cpu_pct,
            ram_mb: c.ram_mb,
            nebula_ip: String::new(),
            disk_path: String::new(),
            meshfs_available: false,
        });
    }
    out
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
            if name.is_empty() {
                return None;
            }
            Some(ContainerEntry {
                name,
                state,
                cpu_pct: 0.0,
                ram_mb: 0,
            })
        })
        .collect()
}

/// Read this peer's compute directly off libvirt + podman — the Bus-
/// independent fallback the Local tab uses when the mesh is unavailable
/// (§13 M11). CPU/RAM/disk are left empty (degraded offline view).
fn read_local_direct() -> Inventory {
    let vms = parse_virsh_list_state(&run_cmd("virsh", &["-c", "qemu:///system", "list", "--all"]))
        .into_iter()
        .map(|(name, state)| VmEntry {
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

// ── VM detail (VIRT-14.a) ────────────────────────────────────────────────

/// A snapshot of one VM's data for the detail panel. Carries `is_local`
/// so the panel can enable actions only for VMs on this peer (§13 M4).
#[derive(Debug, Clone)]
pub(crate) struct VmDetail {
    pub name: String,
    pub state: String,
    pub cpu_pct: f64,
    pub ram_mb: u64,
    pub disk_path: String,
    pub nebula_ip: String,
    pub meshfs_available: bool,
    pub is_local: bool,
}

impl VmDetail {
    fn from_row(r: &ResourceRow, is_local: bool) -> Self {
        Self {
            name: r.name.clone(),
            state: r.state.clone(),
            cpu_pct: r.cpu_pct,
            ram_mb: r.ram_mb,
            disk_path: r.disk_path.clone(),
            nebula_ip: r.nebula_ip.clone(),
            meshfs_available: r.meshfs_available,
            is_local,
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
    local_host: String,
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
            local_host: local_hostname(),
        }
    }

    pub(crate) fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::SwitchTab(t) => {
                self.tab = t;
                Task::none()
            }
            Message::TogglePeer(peer) => {
                let e = self.expanded.entry(peer).or_insert(true);
                *e = !*e;
                Task::none()
            }
            Message::Refresh => Task::perform(async { poll() }, Message::PollLoaded),
            Message::PollLoaded(r) => {
                self.fleet = r.fleet;
                self.local_direct = r.local_direct;
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
                let fetch = detail.is_local.then(|| detail.name.clone());
                self.selected = Some(detail);
                match fetch {
                    Some(vm) => {
                        Task::perform(async move { snapshot_list(&vm) }, Message::SnapshotsLoaded)
                    }
                    None => Task::none(),
                }
            }
            Message::CloseDetail => {
                self.selected = None;
                Task::none()
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
        iced::time::every(Duration::from_secs(5)).map(|_| Message::Refresh)
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
        let content: Element<'_, Message> = match &self.selected {
            Some(d) => row![
                container(tab_body).width(Length::FillPortion(3)).height(Length::Fill),
                self.detail_panel(d),
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
            None => tab_body,
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
        row![
            text("Virtual")
                .size(TypeRole::Subheading.size_in(self.tokens.font_size))
                .color(rgba(palette.text)),
            Space::new().width(Length::Fill),
            self.tab_button("Fleet", Tab::Fleet),
            self.tab_button("Local", Tab::Local),
        ]
        .spacing(f32::from(space.sm))
        .padding([space.sm2, space.lg2])
        .align_y(iced::alignment::Vertical::Center)
        .into()
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

    fn local_view(&self) -> Element<'_, Message> {
        let local_inv: Option<&Inventory> = match &self.fleet {
            FleetState::Available(invs) => invs.iter().find(|i| is_local(i, &self.local_host)),
            FleetState::Unavailable => self.local_direct.as_ref(),
        };
        match local_inv {
            Some(inv) => self.peer_list(std::iter::once(inv), true),
            None => self.empty_state("No local compute discovered."),
        }
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
                for r in &rows {
                    col = col.push(self.resource_row(r, show_actions, local));
                }
            }
        }
        container(col).width(Length::Fill).into()
    }

    fn resource_row(&self, r: &ResourceRow, show_actions: bool, is_local: bool) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let nebula = if r.nebula_ip.is_empty() {
            "\u{2014}".to_string() // —
        } else {
            r.nebula_ip.clone()
        };

        // VM names are clickable → open the detail panel; containers are
        // plain text (their management lands with VIRT-18).
        let name_el: Element<'_, Message> = if matches!(r.kind, ResourceKind::Vm) {
            let detail = VmDetail::from_row(r, is_local);
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
            text(nebula)
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
        }
        widget.into()
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

        // Snapshots (local VMs only) — VIRT-14.b.
        if d.is_local {
            col = col.push(self.snapshot_section(d));
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
        assert_eq!(rows[0].nebula_ip, "10.42.0.5");
        assert_eq!(rows[0].disk_path, "/var/lib/mde-vms/web.qcow2");
        assert!(rows[0].meshfs_available);
        assert_eq!(rows[1].kind, ResourceKind::Container);
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
        let json = r#"[{"Id":"abc","Names":["redis"],"State":"running"},
                       {"Id":"def","Names":["pg"],"State":"exited"}]"#;
        let out = parse_podman_ps_local(json);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, "redis");
        assert_eq!(out[0].state, "running");
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
        let d = VmDetail::from_row(&rows[0], true);
        assert_eq!(d.name, "web");
        assert_eq!(d.disk_path, "/var/lib/mde-vms/web.qcow2");
        assert_eq!(d.nebula_ip, "10.42.0.5");
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
}
