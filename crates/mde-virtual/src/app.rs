//! VIRT-13.a — `mde-virtual` application core.
//!
//! Holds the read-only Fleet/Local viewer: the Bus-fed inventory data
//! model, the 5 s poll, and the Iced `update` / `view` / `subscription`
//! surface. Every visual value flows from `mde-theme` tokens — no
//! hardcoded colors or sizes (lint-design-tokens contract).

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
// publishes to `compute/inventory/<peer>` (see
// `crates/mackesd/src/workers/compute_registry.rs`). We own a local copy
// rather than depend on that crate: the worker lives behind the
// `async-services` feature, and this GUI only ever *reads* the JSON.
// Every field is `#[serde(default)]` so a future schema addition can't
// break deserialization (forward-compatible, matching the daemon's own
// tolerant-read convention).

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
    pub nebula_ip: String,
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

// ── Bus read ────────────────────────────────────────────────────────────

/// Resolve the Mackes Bus on-disk root (`$XDG_DATA_HOME/mde/bus`).
///
/// Mirrors `mde_bus::default_data_dir()`; replicated here as a one-liner
/// over the stable BUS-1.6/1.7 path convention so the GUI doesn't pull in
/// the whole broker crate just to learn one directory.
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

/// One poll's result. `Unavailable` means the Bus root itself is missing
/// or unreachable (broker not running / no home) — the Fleet tab renders
/// the "Mesh unavailable" banner. An *empty* `Available` means the Bus is
/// up but no peer has published compute yet.
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

/// Read the current fleet inventory off the Bus tree.
pub(crate) fn read_fleet() -> FleetState {
    let Some(root) = bus_root() else {
        return FleetState::Unavailable;
    };
    if !root.is_dir() {
        // Broker has never run / bus tree absent — treat as unreachable.
        return FleetState::Unavailable;
    }
    let inv_dir = root.join("compute").join("inventory");
    let entries = collect_inventory_files(&inv_dir);
    FleetState::Available(pick_latest_per_peer(entries))
}

/// Collect `(ulid_filename, Inventory)` pairs from the inventory topic
/// tree: `<inv_dir>/<peer>/<ulid>.json` (and tolerating flat
/// `<inv_dir>/<ulid>.json`). Unreadable / unparseable files are skipped.
fn collect_inventory_files(inv_dir: &Path) -> Vec<(String, Inventory)> {
    let mut out = Vec::new();
    let Ok(top) = std::fs::read_dir(inv_dir) else {
        return out; // topic tree not created yet → no compute published
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

/// Keep only the newest inventory per peer. ULID filenames sort
/// lexicographically by creation time, so the lexically-largest filename
/// is the freshest snapshot. Output is sorted by hostname (then peer) for
/// a stable display order. Pure — unit-tested.
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
    /// Switch the active tab.
    SwitchTab(Tab),
    /// Collapse/expand a peer's section (keyed by Nebula IP).
    TogglePeer(String),
    /// 5 s timer fired — kick off a fresh Bus read.
    Refresh,
    /// A Bus read finished.
    FleetLoaded(FleetState),
}

/// `mde-virtual` application state.
pub(crate) struct VirtualApp {
    tokens: Tokens,
    tab: Tab,
    fleet: FleetState,
    /// Per-peer expansion state; absent = expanded (default open).
    expanded: HashMap<String, bool>,
    local_host: String,
}

impl VirtualApp {
    /// Boot the app: resolve tokens, do the initial synchronous Bus read,
    /// and capture this machine's hostname for the Local tab.
    pub(crate) fn new() -> Self {
        Self {
            tokens: Tokens::resolve(Theme::Dark, Density::Comfortable),
            tab: Tab::Fleet,
            fleet: read_fleet(),
            expanded: HashMap::new(),
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
            Message::Refresh => Task::perform(async { read_fleet() }, Message::FleetLoaded),
            Message::FleetLoaded(fs) => {
                self.fleet = fs;
                Task::none()
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
        let body: Element<'_, Message> = match self.tab {
            Tab::Fleet => self.fleet_view(),
            Tab::Local => self.local_view(),
        };
        let inner = column![self.header_bar(), body]
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
        button(
            text(label.to_string()).size(TypeRole::Body.size_in(self.tokens.font_size)),
        )
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
        self.peer_list(invs.iter())
    }

    fn local_view(&self) -> Element<'_, Message> {
        let local_host = &self.local_host;
        let locals: Vec<&Inventory> = self
            .fleet
            .inventories()
            .iter()
            .filter(|i| is_local(i, local_host))
            .collect();
        if locals.is_empty() {
            // The Local tab keeps rendering even when the mesh is down;
            // the direct libvirt/podman fallback that fills it offline
            // (and the per-row action buttons) land with VIRT-13.b.
            return self.empty_state("No local compute discovered.");
        }
        self.peer_list(locals.into_iter())
    }

    fn peer_list<'a, I>(&'a self, invs: I) -> Element<'a, Message>
    where
        I: Iterator<Item = &'a Inventory>,
    {
        let space = self.tokens.space;
        let sections: Vec<Element<'a, Message>> =
            invs.map(|inv| self.peer_section(inv)).collect();
        scrollable(
            column(sections)
                .spacing(f32::from(space.sm))
                .padding([space.sm, space.lg2])
                .width(Length::Fill),
        )
        .height(Length::Fill)
        .into()
    }

    fn peer_section<'a>(&'a self, inv: &'a Inventory) -> Element<'a, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
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
                    col = col.push(self.resource_row(r));
                }
            }
        }
        container(col).width(Length::Fill).into()
    }

    fn resource_row(&self, r: &ResourceRow) -> Element<'_, Message> {
        let palette = self.tokens.palette;
        let space = self.tokens.space;
        let nebula = if r.nebula_ip.is_empty() {
            "\u{2014}".to_string() // —
        } else {
            r.nebula_ip.clone()
        };
        row![
            text(r.name.clone())
                .size(TypeRole::Body.size_in(self.tokens.font_size))
                .color(rgba(palette.text))
                .width(Length::FillPortion(4)),
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
        .align_y(iced::alignment::Vertical::Center)
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
                          "cpu_pct":12.5,"ram_mb":2048,"disk_path":"/x",
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
    fn rows_for_flattens_vms_then_containers() {
        let inv: Inventory = serde_json::from_str(&sample_json("p", "h")).unwrap();
        let rows = rows_for(&inv);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].kind, ResourceKind::Vm);
        assert_eq!(rows[0].nebula_ip, "10.42.0.5");
        assert_eq!(rows[1].kind, ResourceKind::Container);
        assert!(rows[1].nebula_ip.is_empty());
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
        assert_eq!(out[0].hostname, "alpha"); // sorted by hostname
        assert_eq!(out[1].hostname, "bravo");
    }

    #[test]
    fn is_local_matches_hostname_only() {
        let inv: Inventory = serde_json::from_str(&sample_json("p", "alpha")).unwrap();
        assert!(is_local(&inv, "alpha"));
        assert!(!is_local(&inv, "bravo"));
        assert!(!is_local(&inv, "")); // empty local host never matches
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
}
