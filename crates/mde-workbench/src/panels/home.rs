//! v2.6 OV-1..OV-11 — Workbench Overview landing page.
//!
//! Workbench's default route when launched without `--focus`.
//! Renders three stacked surfaces:
//!
//!   1. **Identity strip** — MDE X.Y.Z · Fedora N · hostname.
//!   2. **Hero stat grid** — mesh peers / pending updates /
//!      snapshots / drift count (4 cards from the original WB-2.a
//!      panel, preserved for continuity).
//!   3. **Capability list** — 11 rows mirroring the cross-host
//!      mesh capability list (mesh network, peer reachability,
//!      file sharing, SSH, RDP, VNC, media & app discovery,
//!      phone pairing, voice & video, fleet config, desktop
//!      notifications). Each row carries a live status pill +
//!      one-sentence plain-English description + jump button
//!      that deep-links to the panel where the user configures
//!      that capability.
//!
//! Backend integration:
//! - peers / snapshots / drift snapshot counts read filesystem
//!   caches (BUG-11 `~/.cache/mde/dnf-updates.count`,
//!   `~/.local/share/mackes-shell/snapshots/`);
//! - capability probes shell out to systemctl / dbus-send /
//!   `mackesd nodes list --json` in parallel via `tokio::join!`;
//! - live refresh comes from a D-Bus signal subscription
//!   (see `dbus_subscription`) bridging systemd1
//!   PropertiesChanged + Nebula peer/transport signals +
//!   Fleet revision signals into `Message::DbusEvent`.
//!
//! Coming-soon capabilities (File Sharing v5.0.0, Phone Pairing
//! v2.1, Voice & Video v4.1.0) render with a disabled "Coming
//! soon" button — the row signals roadmap intent without
//! pretending to be configurable.

use std::path::PathBuf;

use iced::widget::{button, column, container, row, scrollable, svg as widget_svg, text, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Task, Theme};
use mde_theme::{mde_icon, FontSize, Icon, IconSize, Palette, TypeRole};

use crate::model::Group;
use crate::panels::mesh_services::MESH_UNITS;
use crate::panels::mesh_topology::fetch_peers;

// ---------------------------------------------------------------------------
// Capability types (OV-1)
// ---------------------------------------------------------------------------

/// Stable per-row identity. The order this enum is declared in
/// matches the render order in `build_all_rows`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityId {
    Mesh,
    Peers,
    Files,
    Ssh,
    Rdp,
    Vnc,
    Services,
    Phone,
    Voice,
    Fleet,
    Notifications,
}

/// What the status pill should communicate for a single
/// capability row. Color + icon match the mesh_topology palette
/// so the rest of the workbench renders status consistently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityStatus {
    /// Green — capability is up and serving.
    Active,
    /// Yellow — capability ships in this version but is not
    /// currently running (operator action needed).
    SetupNeeded,
    /// Gray — capability ships in a future version. `version` is
    /// the SemVer string ("5.0.0", "2.1", "4.1.0") rendered as
    /// "Coming in v{version}".
    ComingSoon { version: &'static str },
    /// Red — capability shipped, ran, and is in an error state.
    /// `detail` carries an operator-readable one-liner that
    /// shows up as sub-status.
    Failed { detail: String },
    /// Probe could not determine state (mackesd down, no
    /// systemctl access, etc.). Renders gray with "Unknown".
    Unknown,
}

impl CapabilityStatus {
    #[must_use]
    pub fn icon(&self) -> Icon {
        match self {
            Self::Active => Icon::StatusOk,
            Self::SetupNeeded => Icon::StatusWarning,
            Self::ComingSoon { .. } | Self::Unknown => Icon::StatusUnknown,
            Self::Failed { .. } => Icon::StatusError,
        }
    }
    #[must_use]
    pub fn color(&self) -> Color {
        match self {
            Self::Active => Color::from_rgb(0.20, 0.80, 0.40),
            Self::SetupNeeded => Color::from_rgb(0.95, 0.70, 0.20),
            Self::ComingSoon { .. } | Self::Unknown => Color::from_rgb(0.55, 0.55, 0.55),
            Self::Failed { .. } => Color::from_rgb(0.92, 0.32, 0.30),
        }
    }
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Active => "Active".into(),
            Self::SetupNeeded => "Setup needed".into(),
            Self::ComingSoon { version } => format!("Coming in v{version}"),
            Self::Failed { .. } => "Failed".into(),
            Self::Unknown => "Unknown".into(),
        }
    }
    #[must_use]
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::Failed { detail } => Some(detail.as_str()),
            _ => None,
        }
    }
}

/// One row in the capability list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityRow {
    pub id: CapabilityId,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: Icon,
    pub status: CapabilityStatus,
    /// Optional secondary line under the description — e.g.
    /// "3 of 5 peers online" or "last sync 2 minutes ago".
    pub sub_status: Option<String>,
    /// Where the Configure button takes the user. `None` =>
    /// the capability is coming-soon or has no settings panel;
    /// render the button disabled with "Coming soon".
    pub jump: Option<(Group, &'static str)>,
}

// ---------------------------------------------------------------------------
// Snapshot + panel state (OV-3)
// ---------------------------------------------------------------------------

/// Pure-data snapshot of what the Overview shows.
#[derive(Debug, Clone, Default)]
pub struct HomeSnapshot {
    pub mde_version: String,
    pub fedora_release: String,
    pub hostname: String,
    /// Hero stat counts. `None` = unknown → renders "—".
    pub mesh_peers: Option<u32>,
    pub pending_updates: Option<u32>,
    pub snapshot_count: Option<u32>,
    pub drift_count: Option<u32>,
    /// Capability list — populated by the async load. Empty
    /// vec renders the section with a "Loading…" placeholder.
    pub capabilities: Vec<CapabilityRow>,
    /// True if `dev.mackes.MDE.Shell::Healthz()` succeeded on
    /// the last probe. False renders a top banner reminding the
    /// operator that D-Bus-sourced rows may be stale.
    pub mackesd_reachable: bool,
}

impl HomeSnapshot {
    /// Synchronous filesystem-only load. Cheap; no async.
    #[must_use]
    pub fn load_sync() -> Self {
        Self {
            mde_version: env!("CARGO_PKG_VERSION").to_string(),
            fedora_release: read_fedora_release().unwrap_or_else(|| "44".into()),
            hostname: read_hostname(),
            mesh_peers: None,
            pending_updates: Some(read_dnf_count()),
            snapshot_count: count_snapshots(),
            drift_count: None,
            capabilities: Vec::new(),
            mackesd_reachable: true,
        }
    }
}

fn read_fedora_release() -> Option<String> {
    let content = std::fs::read_to_string("/etc/os-release").ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("VERSION_ID=") {
            return Some(rest.trim().trim_matches('"').to_string());
        }
    }
    None
}

fn read_hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "fedora".into())
}

fn read_dnf_count() -> u32 {
    let cache = std::env::var("XDG_CACHE_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_default();
    std::fs::read_to_string(cache.join("mde/dnf-updates.count"))
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0)
}

fn count_snapshots() -> Option<u32> {
    let home = std::env::var("HOME").ok().map(PathBuf::from)?;
    let dir = home.join(".local/share/mackes-shell/snapshots");
    let entries = std::fs::read_dir(&dir).ok()?;
    let n = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .count();
    n.try_into().ok()
}

// ---------------------------------------------------------------------------
// Panel state machine
// ---------------------------------------------------------------------------

/// What a probe reported. The status enum is the source of
/// truth for the pill; sub_status is the optional one-liner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeOutcome {
    pub status: CapabilityStatus,
    pub sub_status: Option<String>,
}

impl ProbeOutcome {
    fn active(sub_status: Option<String>) -> Self {
        Self { status: CapabilityStatus::Active, sub_status }
    }
    fn setup_needed(sub_status: Option<String>) -> Self {
        Self { status: CapabilityStatus::SetupNeeded, sub_status }
    }
    fn unknown() -> Self {
        Self { status: CapabilityStatus::Unknown, sub_status: None }
    }
    fn failed(detail: impl Into<String>) -> Self {
        let detail = detail.into();
        Self { status: CapabilityStatus::Failed { detail: detail.clone() }, sub_status: Some(detail) }
    }
}

/// A single D-Bus or systemd event that warrants re-probing a
/// specific capability. The subscription bridges raw signals
/// into one of these.
#[derive(Debug, Clone)]
pub enum DbusEvent {
    /// Peer-set membership or reachability changed — re-probe
    /// Mesh + Peers + Fleet (fleet revision push triggers this
    /// too, since revisions are pushed via the peer set).
    PeerChanged,
    /// Active transport rotated — re-probe Mesh.
    TransportChanged,
    /// A systemd unit's PropertiesChanged fired — re-probe SSH,
    /// RDP, VNC, or Services depending on `unit`.
    UnitChanged(String),
    /// Fleet pushed a revision — re-probe Fleet.
    FleetRevisionPushed,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refreshed(HomeSnapshot),
    CapabilitiesRefreshed {
        rows: Vec<CapabilityRow>,
        mackesd_reachable: bool,
    },
    RefreshClicked,
    DbusEvent(DbusEvent),
}

#[derive(Debug, Clone, Default)]
pub struct HomePanel {
    pub snapshot: HomeSnapshot,
}

impl HomePanel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            snapshot: HomeSnapshot::load_sync(),
        }
    }

    /// Initial load. Fires the cheap sync snapshot immediately +
    /// the async capability fan-out in parallel.
    pub fn load() -> Task<crate::Message> {
        Task::batch(vec![
            Task::perform(async { HomeSnapshot::load_sync() }, |snap| {
                crate::Message::Home(Message::Refreshed(snap))
            }),
            Task::perform(load_capabilities(), |(rows, ok)| {
                crate::Message::Home(Message::CapabilitiesRefreshed {
                    rows,
                    mackesd_reachable: ok,
                })
            }),
        ])
    }

    pub fn update(&mut self, msg: Message) -> Task<crate::Message> {
        match msg {
            Message::Refreshed(snap) => {
                // Preserve capabilities + mackesd_reachable from
                // prior load if the sync snapshot fired alone.
                let capabilities = std::mem::take(&mut self.snapshot.capabilities);
                let mackesd_reachable = self.snapshot.mackesd_reachable;
                self.snapshot = HomeSnapshot {
                    capabilities,
                    mackesd_reachable,
                    ..snap
                };
                Task::none()
            }
            Message::CapabilitiesRefreshed { rows, mackesd_reachable } => {
                self.snapshot.capabilities = rows;
                self.snapshot.mackesd_reachable = mackesd_reachable;
                // Also refresh the hero mesh_peers count from
                // the capabilities (Peers row's sub_status
                // already carries the X/Y string; the count
                // belongs in the hero too for continuity).
                self.snapshot.mesh_peers = self
                    .snapshot
                    .capabilities
                    .iter()
                    .find(|r| r.id == CapabilityId::Peers)
                    .and_then(|r| extract_peer_count(r));
                Task::none()
            }
            Message::RefreshClicked => Self::load(),
            Message::DbusEvent(ev) => Task::perform(reprobe_for_event(ev), |(rows, ok)| {
                crate::Message::Home(Message::CapabilitiesRefreshed {
                    rows,
                    mackesd_reachable: ok,
                })
            }),
        }
    }

    /// Render the Overview.
    pub fn view(&self) -> Element<'_, crate::Message> {
        let palette = Palette::dark();
        let sizes = FontSize::defaults();

        let title = text("Overview")
            .size(TypeRole::Display.size_in(sizes))
            .color(palette.text.into_iced_color());

        let identity = text(format!(
            "MDE {ver} · Fedora {rel} · {host}",
            ver = self.snapshot.mde_version,
            rel = self.snapshot.fedora_release,
            host = self.snapshot.hostname,
        ))
        .size(TypeRole::Body.size_in(sizes))
        .color(palette.text_muted.into_iced_color());

        let cards = row![
            stat_card(
                "Mesh peers",
                self.snapshot.mesh_peers,
                Icon::Peer,
                Group::Fleet,
                "inventory",
                palette,
            ),
            Space::with_width(Length::Fixed(12.0)),
            stat_card(
                "Updates pending",
                self.snapshot.pending_updates,
                Icon::Update,
                Group::Maintain,
                "snapshots",
                palette,
            ),
            Space::with_width(Length::Fixed(12.0)),
            stat_card(
                "Snapshots",
                self.snapshot.snapshot_count,
                Icon::Snapshot,
                Group::Maintain,
                "snapshots",
                palette,
            ),
            Space::with_width(Length::Fixed(12.0)),
            stat_card(
                "Drift events",
                self.snapshot.drift_count,
                Icon::Repair,
                Group::Maintain,
                "drift",
                palette,
            ),
        ];

        // Optional mackesd-down banner — only renders when the
        // last probe could not reach the control plane.
        let banner: Element<'_, crate::Message> = if self.snapshot.mackesd_reachable {
            Space::with_height(Length::Fixed(0.0)).into()
        } else {
            mackesd_banner(palette)
        };

        let section_title = text("What this Mackes mesh can do for you")
            .size(TypeRole::Heading.size_in(sizes))
            .color(palette.text.into_iced_color());
        let section_subtitle = text(
            "Each row shows a feature, whether it is running on this machine, \
             and a shortcut to set it up.",
        )
        .size(TypeRole::Body.size_in(sizes))
        .color(palette.text_muted.into_iced_color());

        let refresh_btn = refresh_button(palette);

        let capability_list: Element<'_, crate::Message> = if self.snapshot.capabilities.is_empty() {
            text("Loading capability status…")
                .size(TypeRole::Body.size_in(sizes))
                .color(palette.text_muted.into_iced_color())
                .into()
        } else {
            let mut col = column![].spacing(8);
            for row_data in &self.snapshot.capabilities {
                col = col.push(capability_card(row_data, palette));
            }
            col.into()
        };

        let body = column![
            title,
            Space::with_height(Length::Fixed(4.0)),
            identity,
            Space::with_height(Length::Fixed(24.0)),
            cards,
            Space::with_height(Length::Fixed(24.0)),
            banner,
            row![section_title, Space::with_width(Length::Fill), refresh_btn]
                .align_y(iced::alignment::Vertical::Center),
            Space::with_height(Length::Fixed(2.0)),
            section_subtitle,
            Space::with_height(Length::Fixed(12.0)),
            capability_list,
        ]
        .spacing(2);

        container(scrollable(body).width(Length::Fill))
            .padding(Padding::from([24u16, 32u16]))
            .width(Length::Fill)
            .into()
    }
}

// ---------------------------------------------------------------------------
// Probes (OV-4)
// ---------------------------------------------------------------------------

/// Top-level async load. Fires every probe in parallel and
/// builds the full capability list.
pub async fn load_capabilities() -> (Vec<CapabilityRow>, bool) {
    let (
        nebula,
        peers,
        ssh,
        rdp,
        vnc,
        services,
        fleet,
        notifications,
        mackesd_ok,
    ) = tokio::join!(
        probe_nebula(),
        probe_peers(),
        probe_systemd_unit("sshd.service"),
        probe_systemd_unit("xrdp.service"),
        probe_vnc(),
        probe_mesh_services(),
        probe_fleet_revision(),
        probe_notifications(),
        probe_mackesd_alive(),
    );
    let rows = vec![
        build_mesh_row(&nebula),
        build_peers_row(&peers),
        build_files_row(),
        build_ssh_row(&ssh),
        build_rdp_row(&rdp),
        build_vnc_row(&vnc),
        build_services_row(&services),
        build_phone_row(),
        build_voice_row(),
        build_fleet_row(&fleet),
        build_notifications_row(&notifications),
    ];
    (rows, mackesd_ok)
}

/// Re-fire only the probes affected by a given D-Bus event,
/// merging into the existing row list. Lighter than the full
/// fan-out — keeps signal-driven refresh cheap.
pub async fn reprobe_for_event(event: DbusEvent) -> (Vec<CapabilityRow>, bool) {
    // Today: simple — just re-run the full fan-out. The event
    // type stays in the API so a future optimization can do
    // per-id reprobes without touching call sites.
    let _ = event;
    load_capabilities().await
}

// --- Nebula ----------------------------------------------------------------

async fn probe_nebula() -> ProbeOutcome {
    // dev.mackes.MDE.Nebula.Status.Status returns a JSON
    // dictionary; we only need active_transport for the pill.
    let raw = match dbus_call(
        "org.mackes.mackesd",
        "/dev/mackes/MDE/Nebula/Status",
        "dev.mackes.MDE.Nebula.Status",
        "Status",
    )
    .await
    {
        Ok(s) => s,
        Err(_) => return ProbeOutcome::unknown(),
    };
    let transport = extract_json_string_field(&raw, "active_transport").unwrap_or_default();
    if transport.is_empty() || transport == "offline" {
        ProbeOutcome::setup_needed(Some("Mesh fabric is not connected".into()))
    } else {
        ProbeOutcome::active(Some(format!("Connected via {}", humanize_transport(&transport))))
    }
}

fn humanize_transport(t: &str) -> String {
    match t {
        "nebula_direct" => "direct UDP".into(),
        "nebula_lighthouse_relay" => "lighthouse relay".into(),
        "nebula_https443" => "HTTPS-443 fallback".into(),
        "kdc_tls" => "KDC2 TLS".into(),
        other => other.replace('_', " "),
    }
}

// --- Peers -----------------------------------------------------------------

async fn probe_peers() -> ProbeOutcome {
    // fetch_peers shells out to `mackesd nodes list --json`.
    // Bounce it onto the executor with spawn_blocking so the
    // sync std::process::Command doesn't stall the runtime.
    let peers = tokio::task::spawn_blocking(fetch_peers).await;
    let peers = match peers {
        Ok(Ok(peers)) => peers,
        _ => return ProbeOutcome::unknown(),
    };
    let total = peers.len();
    let online = peers
        .iter()
        .filter(|p| {
            matches!(
                p.kind.as_str(),
                "host" | "peer" | "lighthouse"
            ) && p
                .addr
                .as_str()
                .ne("offline")
                && !matches!(format!("{:?}", p.status).as_str(), "Offline" | "Unknown")
        })
        .count();
    if total == 0 {
        return ProbeOutcome::setup_needed(Some("No peers enrolled yet".into()));
    }
    let sub = Some(format!("{online} of {total} peers online"));
    if online == 0 {
        ProbeOutcome { status: CapabilityStatus::Failed { detail: "All peers offline".into() }, sub_status: sub }
    } else {
        ProbeOutcome::active(sub)
    }
}

// --- Systemd units ---------------------------------------------------------

async fn probe_systemd_unit(unit: &str) -> ProbeOutcome {
    let state = systemctl_active_state(unit).await;
    match state.as_deref() {
        Some("active") => ProbeOutcome::active(None),
        Some("activating") => ProbeOutcome::setup_needed(Some("Starting…".into())),
        Some("failed") => ProbeOutcome::failed(format!("{unit} failed to start")),
        Some("inactive") => ProbeOutcome::setup_needed(Some(format!("{unit} is stopped"))),
        Some(other) => ProbeOutcome { status: CapabilityStatus::Unknown, sub_status: Some(format!("state: {other}")) },
        None => ProbeOutcome::unknown(),
    }
}

async fn systemctl_active_state(unit: &str) -> Option<String> {
    let out = tokio::process::Command::new("systemctl")
        .args(["show", "-p", "ActiveState", "--value", unit])
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() || s == "(null)" {
        None
    } else {
        Some(s)
    }
}

// --- VNC -------------------------------------------------------------------

async fn probe_vnc() -> ProbeOutcome {
    let x11 = systemctl_active_state("x11vnc@:0.service").await;
    let way = systemctl_active_state("wayvnc.service").await;
    let session_type =
        std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    match (x11.as_deref(), way.as_deref(), session_type.as_str()) {
        (Some("active"), _, _) => ProbeOutcome::active(Some("x11vnc serving :0".into())),
        (_, Some("active"), _) => ProbeOutcome::active(Some("wayvnc serving overlay IP".into())),
        (Some("failed"), _, "wayland") => ProbeOutcome::failed(
            "x11vnc does not run under Wayland — see RD-1..RD-5"
                .to_string(),
        ),
        (Some("failed"), _, _) => ProbeOutcome::failed("x11vnc@:0.service failed".to_string()),
        (Some("inactive"), Some("inactive"), _) | (Some("inactive"), None, _) | (None, Some("inactive"), _) => {
            ProbeOutcome::setup_needed(Some("No VNC server running".into()))
        }
        (None, None, _) => ProbeOutcome::setup_needed(Some("No VNC server installed".into())),
        _ => ProbeOutcome::unknown(),
    }
}

// --- Mesh services registry -----------------------------------------------

async fn probe_mesh_services() -> ProbeOutcome {
    let mut active = 0usize;
    let total = MESH_UNITS.len();
    for (name, _, _) in MESH_UNITS.iter() {
        if let Some(state) = systemctl_active_state(name).await {
            if state == "active" {
                active += 1;
            }
        }
    }
    let sub = Some(format!("{active} of {total} services running"));
    if active == 0 {
        ProbeOutcome::setup_needed(sub)
    } else if active < total {
        ProbeOutcome { status: CapabilityStatus::SetupNeeded, sub_status: sub }
    } else {
        ProbeOutcome::active(sub)
    }
}

// --- Fleet -----------------------------------------------------------------

async fn probe_fleet_revision() -> ProbeOutcome {
    // dev.mackes.MDE.Fleet::ListRevisions returns a JSON array
    // of revision IDs ("r-YYYY-MM-DD-NNNN") in descending order.
    let raw = match dbus_call(
        "org.mackes.mackesd",
        "/dev/mackes/MDE/Fleet",
        "dev.mackes.MDE.Fleet",
        "ListRevisions",
    )
    .await
    {
        Ok(s) => s,
        Err(_) => return ProbeOutcome::unknown(),
    };
    let latest = extract_first_revision_id(&raw);
    match latest {
        Some(id) => ProbeOutcome::active(Some(format!("Last update {}", humanize_revision_age(&id)))),
        None => ProbeOutcome::setup_needed(Some("No revisions pushed yet".into())),
    }
}

fn extract_first_revision_id(raw: &str) -> Option<String> {
    // Match the first "r-YYYY-MM-DD-NNNN" token in the raw
    // dbus-send output. dbus-send's text format is more
    // permissive than JSON so a quick token scan is the
    // most resilient parse.
    for tok in raw.split(|c: char| c == '"' || c == ' ' || c == '\n' || c == ',' || c == '[' || c == ']') {
        if tok.starts_with("r-") && tok.len() >= 12 {
            return Some(tok.to_string());
        }
    }
    None
}

fn humanize_revision_age(id: &str) -> String {
    // r-YYYY-MM-DD-NNNN — show the YYYY-MM-DD portion.
    if let Some(date) = id.strip_prefix("r-").and_then(|s| s.get(..10)) {
        date.to_string()
    } else {
        id.to_string()
    }
}

// --- Notifications ---------------------------------------------------------

async fn probe_notifications() -> ProbeOutcome {
    match dbus_call(
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
        "GetServerInformation",
    )
    .await
    {
        Ok(raw) => {
            // Reply has 4 strings: name, vendor, version,
            // spec_version. Pluck name + version for sub-status.
            let name = extract_dbus_string_at(&raw, 0).unwrap_or_else(|| "notifications".into());
            let version = extract_dbus_string_at(&raw, 2).unwrap_or_default();
            let sub = if version.is_empty() {
                Some(format!("Daemon: {name}"))
            } else {
                Some(format!("Daemon: {name} {version}"))
            };
            ProbeOutcome::active(sub)
        }
        Err(_) => ProbeOutcome::setup_needed(Some("No notification daemon registered".into())),
    }
}

// --- mackesd health --------------------------------------------------------

async fn probe_mackesd_alive() -> bool {
    dbus_call(
        "org.mackes.mackesd",
        "/dev/mackes/MDE/Shell",
        "dev.mackes.MDE.Shell",
        "Healthz",
    )
    .await
    .is_ok()
}

// --- D-Bus shell-out -------------------------------------------------------

async fn dbus_call(
    destination: &str,
    object_path: &str,
    interface: &str,
    method: &str,
) -> Result<String, String> {
    let out = tokio::process::Command::new("dbus-send")
        .args([
            "--session",
            "--print-reply=literal",
            "--type=method_call",
            &format!("--dest={destination}"),
            object_path,
            &format!("{interface}.{method}"),
        ])
        .output()
        .await
        .map_err(|e| format!("dbus-send spawn: {e}"))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    // Some surfaces (notifications) live on the session bus by
    // default but mackesd's interfaces may also be reachable via
    // the system bus depending on operator setup. Try the system
    // bus before giving up.
    let out = tokio::process::Command::new("dbus-send")
        .args([
            "--system",
            "--print-reply=literal",
            "--type=method_call",
            &format!("--dest={destination}"),
            object_path,
            &format!("{interface}.{method}"),
        ])
        .output()
        .await
        .map_err(|e| format!("dbus-send spawn: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

fn extract_json_string_field(raw: &str, field: &str) -> Option<String> {
    // dbus-send --print-reply=literal returns the value plain.
    // mackesd's Status() returns a JSON blob, so grep for
    // "<field>":"<value>" or "<field>": "<value>".
    let needle = format!("\"{field}\"");
    let idx = raw.find(&needle)?;
    let after = &raw[idx + needle.len()..];
    let after = after.trim_start_matches(|c: char| c == ':' || c.is_whitespace());
    if let Some(rest) = after.strip_prefix('"') {
        let end = rest.find('"')?;
        Some(rest[..end].to_string())
    } else {
        // Bare token (number/identifier).
        let end = after
            .find(|c: char| c == ',' || c == '}' || c.is_whitespace())
            .unwrap_or(after.len());
        Some(after[..end].to_string())
    }
}

fn extract_dbus_string_at(raw: &str, idx: usize) -> Option<String> {
    // dbus-send literal output puts each string on its own line
    // (or whitespace-separated). Iterate quoted tokens; bare
    // strings appear unquoted in literal mode.
    let mut tokens = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Literal mode prints `string "name"` for variants, just
        // `name` for plain string args. Handle both.
        if let Some(stripped) = line.strip_prefix("string ") {
            tokens.push(stripped.trim().trim_matches('"').to_string());
        } else if line.starts_with('"') && line.ends_with('"') && line.len() >= 2 {
            tokens.push(line[1..line.len() - 1].to_string());
        } else {
            tokens.push(line.to_string());
        }
    }
    tokens.get(idx).cloned()
}

// ---------------------------------------------------------------------------
// Row builders (OV-5)
// ---------------------------------------------------------------------------

fn build_mesh_row(p: &ProbeOutcome) -> CapabilityRow {
    CapabilityRow {
        id: CapabilityId::Mesh,
        name: "Mesh Network",
        description: "Encrypted private network between every peer.",
        icon: Icon::Network,
        status: p.status.clone(),
        sub_status: p.sub_status.clone(),
        jump: Some((Group::Network, "mesh_control")),
    }
}

fn build_peers_row(p: &ProbeOutcome) -> CapabilityRow {
    CapabilityRow {
        id: CapabilityId::Peers,
        name: "Peer Reachability",
        description: "Which of your devices are online right now.",
        icon: Icon::Peer,
        status: p.status.clone(),
        sub_status: p.sub_status.clone(),
        jump: Some((Group::Network, "mesh_topology")),
    }
}

fn build_files_row() -> CapabilityRow {
    CapabilityRow {
        id: CapabilityId::Files,
        name: "File Sharing",
        description: "Every peer holds every file in your shared folders.",
        icon: Icon::Files,
        status: CapabilityStatus::ComingSoon { version: "5.0.0" },
        sub_status: None,
        jump: None,
    }
}

fn build_ssh_row(p: &ProbeOutcome) -> CapabilityRow {
    CapabilityRow {
        id: CapabilityId::Ssh,
        name: "SSH Across Mesh",
        description: "Open a terminal on any peer from any other peer.",
        icon: Icon::System,
        status: p.status.clone(),
        sub_status: p.sub_status.clone(),
        jump: Some((Group::Network, "mesh_ssh")),
    }
}

fn build_rdp_row(p: &ProbeOutcome) -> CapabilityRow {
    CapabilityRow {
        id: CapabilityId::Rdp,
        name: "Remote Desktop (RDP)",
        description: "See and control any peer's screen with an RDP client.",
        icon: Icon::Display,
        status: p.status.clone(),
        sub_status: p.sub_status.clone(),
        jump: Some((Group::Network, "remote_desktop")),
    }
}

fn build_vnc_row(p: &ProbeOutcome) -> CapabilityRow {
    CapabilityRow {
        id: CapabilityId::Vnc,
        name: "Remote Desktop (VNC)",
        description: "See and control any peer's screen with a VNC client.",
        icon: Icon::Display,
        status: p.status.clone(),
        sub_status: p.sub_status.clone(),
        jump: Some((Group::Network, "remote_desktop")),
    }
}

fn build_services_row(p: &ProbeOutcome) -> CapabilityRow {
    CapabilityRow {
        id: CapabilityId::Services,
        name: "Media & App Discovery",
        description: "Find and open services running on other peers in one click.",
        icon: Icon::Apps,
        status: p.status.clone(),
        sub_status: p.sub_status.clone(),
        jump: Some((Group::Network, "mesh_services")),
    }
}

fn build_phone_row() -> CapabilityRow {
    CapabilityRow {
        id: CapabilityId::Phone,
        name: "Phone Pairing",
        description: "Connect your phone to mirror notifications, SMS, and clipboard.",
        icon: Icon::Devices,
        status: CapabilityStatus::ComingSoon { version: "2.1" },
        sub_status: None,
        jump: None,
    }
}

fn build_voice_row() -> CapabilityRow {
    CapabilityRow {
        id: CapabilityId::Voice,
        name: "Voice & Video",
        description: "Call any peer or any phone number from anywhere on the mesh.",
        icon: Icon::Sound,
        status: CapabilityStatus::ComingSoon { version: "4.1.0" },
        sub_status: None,
        jump: None,
    }
}

fn build_fleet_row(p: &ProbeOutcome) -> CapabilityRow {
    CapabilityRow {
        id: CapabilityId::Fleet,
        name: "Fleet Configuration",
        description: "Push the same settings to every peer at once.",
        icon: Icon::Fleet,
        status: p.status.clone(),
        sub_status: p.sub_status.clone(),
        jump: Some((Group::Fleet, "playbooks")),
    }
}

fn build_notifications_row(p: &ProbeOutcome) -> CapabilityRow {
    CapabilityRow {
        id: CapabilityId::Notifications,
        name: "Desktop Notifications",
        description: "App notifications that work across every peer.",
        icon: Icon::Notification,
        status: p.status.clone(),
        sub_status: p.sub_status.clone(),
        jump: Some((Group::System, "notifications")),
    }
}

/// For tests and Overview consumers that want the literal row
/// ordering without firing any probes.
#[must_use]
pub fn build_all_rows_with_unknown_status() -> Vec<CapabilityRow> {
    vec![
        build_mesh_row(&ProbeOutcome::unknown()),
        build_peers_row(&ProbeOutcome::unknown()),
        build_files_row(),
        build_ssh_row(&ProbeOutcome::unknown()),
        build_rdp_row(&ProbeOutcome::unknown()),
        build_vnc_row(&ProbeOutcome::unknown()),
        build_services_row(&ProbeOutcome::unknown()),
        build_phone_row(),
        build_voice_row(),
        build_fleet_row(&ProbeOutcome::unknown()),
        build_notifications_row(&ProbeOutcome::unknown()),
    ]
}

// ---------------------------------------------------------------------------
// Widgets (OV-6 + OV-10)
// ---------------------------------------------------------------------------

fn icon_widget<'a>(icon: Icon, size: IconSize, color: Color) -> Element<'a, crate::Message> {
    let resolved = mde_icon(icon, size);
    if let Some(svg_bytes) = resolved.svg_bytes() {
        widget_svg(widget_svg::Handle::from_memory(svg_bytes))
            .width(Length::Fixed(resolved.size_px()))
            .height(Length::Fixed(resolved.size_px()))
            .style(move |_t: &Theme, _s: widget_svg::Status| widget_svg::Style { color: Some(color) })
            .into()
    } else {
        text(resolved.fallback_glyph)
            .size(resolved.size_px())
            .color(color)
            .into()
    }
}

fn capability_card<'a>(row_data: &'a CapabilityRow, palette: Palette) -> Element<'a, crate::Message> {
    let icon = icon_widget(row_data.icon, IconSize::PanelHeader, palette.text.into_iced_color());
    let name = text(row_data.name)
        .size(16)
        .color(palette.text.into_iced_color());
    let description = text(row_data.description)
        .size(13)
        .color(palette.text_muted.into_iced_color());
    let sub_status: Element<'_, crate::Message> = match row_data
        .status
        .detail()
        .map(str::to_string)
        .or_else(|| row_data.sub_status.clone())
    {
        Some(s) => text(s)
            .size(12)
            .color(palette.text_muted.into_iced_color())
            .into(),
        None => Space::with_height(Length::Fixed(0.0)).into(),
    };

    let pill = status_pill(&row_data.status, palette);
    let jump = jump_button(row_data, palette);

    let top_row = row![
        icon,
        Space::with_width(Length::Fixed(12.0)),
        column![name, description].spacing(2),
        Space::with_width(Length::Fill),
        pill,
    ]
    .align_y(iced::alignment::Vertical::Center);

    let bottom_row = row![sub_status, Space::with_width(Length::Fill), jump]
        .align_y(iced::alignment::Vertical::Center);

    let bg = palette.raised.into_iced_color();
    let border = palette.border.into_iced_color();
    container(column![top_row, Space::with_height(Length::Fixed(8.0)), bottom_row].spacing(0))
        .padding(Padding::from([16u16, 16u16]))
        .width(Length::Fill)
        .style(move |_t: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                color: border,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn status_pill(status: &CapabilityStatus, _palette: Palette) -> Element<'_, crate::Message> {
    let color = status.color();
    let label = status.label();
    row![
        icon_widget(status.icon(), IconSize::Inline, color),
        Space::with_width(Length::Fixed(4.0)),
        text(label).size(12).color(color),
    ]
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

fn jump_button<'a>(row_data: &'a CapabilityRow, palette: Palette) -> Element<'a, crate::Message> {
    let bg = palette.raised.into_iced_color();
    let border = palette.border.into_iced_color();
    let text_color = palette.text.into_iced_color();
    let muted = palette.text_muted.into_iced_color();
    let style = move |_t: &Theme, status: iced::widget::button::Status| {
        let hover_bg = Color {
            r: bg.r * 1.12,
            g: bg.g * 1.12,
            b: bg.b * 1.12,
            a: bg.a,
        };
        iced::widget::button::Style {
            background: Some(Background::Color(match status {
                iced::widget::button::Status::Hovered => hover_bg,
                _ => bg,
            })),
            text_color,
            border: Border {
                color: border,
                width: 1.0,
                radius: 6.0.into(),
            },
            shadow: iced::Shadow::default(),
        }
    };
    let disabled_style = move |_t: &Theme, _status: iced::widget::button::Status| {
        iced::widget::button::Style {
            background: Some(Background::Color(bg)),
            text_color: muted,
            border: Border {
                color: border,
                width: 1.0,
                radius: 6.0.into(),
            },
            shadow: iced::Shadow::default(),
        }
    };

    if let Some((group, panel)) = row_data.jump {
        button(text("Configure  ▸").size(13))
            .padding(Padding::from([6u16, 14u16]))
            .style(style)
            .on_press(crate::Message::SelectPanel { group, panel })
            .into()
    } else {
        button(text("Coming soon").size(13))
            .padding(Padding::from([6u16, 14u16]))
            .style(disabled_style)
            .into()
    }
}

fn refresh_button<'a>(palette: Palette) -> Element<'a, crate::Message> {
    let bg = palette.raised.into_iced_color();
    let border = palette.border.into_iced_color();
    let text_color = palette.text.into_iced_color();
    button(text("Refresh").size(12))
        .padding(Padding::from([4u16, 12u16]))
        .style(move |_t: &Theme, status: iced::widget::button::Status| {
            let hover_bg = Color {
                r: bg.r * 1.12,
                g: bg.g * 1.12,
                b: bg.b * 1.12,
                a: bg.a,
            };
            iced::widget::button::Style {
                background: Some(Background::Color(match status {
                    iced::widget::button::Status::Hovered => hover_bg,
                    _ => bg,
                })),
                text_color,
                border: Border {
                    color: border,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                shadow: iced::Shadow::default(),
            }
        })
        .on_press(crate::Message::Home(Message::RefreshClicked))
        .into()
}

fn mackesd_banner<'a>(palette: Palette) -> Element<'a, crate::Message> {
    let yellow = Color::from_rgb(0.95, 0.70, 0.20);
    let bg = palette.raised.into_iced_color();
    container(
        row![
            icon_widget(Icon::StatusWarning, IconSize::PanelHeader, yellow),
            Space::with_width(Length::Fixed(8.0)),
            text("mackesd is not responding — capability statuses may be stale.")
                .size(13)
                .color(palette.text.into_iced_color()),
        ]
        .align_y(iced::alignment::Vertical::Center),
    )
    .padding(Padding::from([10u16, 14u16]))
    .width(Length::Fill)
    .style(move |_t: &Theme| iced::widget::container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: yellow,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    })
    .into()
}

// ---------------------------------------------------------------------------
// Stat card (existing — preserved)
// ---------------------------------------------------------------------------

fn stat_card<'a>(
    label: &'a str,
    value: Option<u32>,
    icon: Icon,
    target_group: Group,
    target_panel: &'a str,
    palette: Palette,
) -> Element<'a, crate::Message> {
    let icon = icon_widget(icon, IconSize::PanelHeader, palette.text_muted.into_iced_color());
    let value_display = match value {
        Some(n) => n.to_string(),
        None => "—".into(),
    };
    let value_text = text(value_display)
        .size(28)
        .color(palette.text.into_iced_color());
    let label_text = text(label.to_string())
        .size(12)
        .color(palette.text_muted.into_iced_color());
    let card_panel_slug: &'static str = match target_panel {
        "snapshots" => "snapshots",
        "drift" => "drift",
        "inventory" => "inventory",
        _ => "snapshots",
    };
    let card = column![
        icon,
        Space::with_height(Length::Fixed(4.0)),
        value_text,
        Space::with_height(Length::Fixed(2.0)),
        label_text,
    ]
    .spacing(0)
    .align_x(iced::alignment::Horizontal::Left);

    let bg = palette.raised.into_iced_color();
    let border = palette.border.into_iced_color();
    let muted_text = palette.text_muted.into_iced_color();
    button(card)
        .width(Length::Fill)
        .padding(Padding::from([16u16, 16u16]))
        .style(move |_t: &Theme, status: iced::widget::button::Status| {
            let hover_bg = Color {
                r: bg.r * 1.08,
                g: bg.g * 1.08,
                b: bg.b * 1.08,
                a: bg.a,
            };
            iced::widget::button::Style {
                background: Some(Background::Color(match status {
                    iced::widget::button::Status::Hovered => hover_bg,
                    _ => bg,
                })),
                text_color: muted_text,
                border: Border {
                    color: border,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                shadow: iced::Shadow::default(),
            }
        })
        .on_press(crate::Message::SelectPanel {
            group: target_group,
            panel: card_panel_slug,
        })
        .into()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn extract_peer_count(row: &CapabilityRow) -> Option<u32> {
    let sub = row.sub_status.as_ref()?;
    // sub_status format: "X of Y peers online" — extract Y.
    let mut tokens = sub.split_whitespace();
    let _x = tokens.next()?;
    let _of = tokens.next()?;
    let total = tokens.next()?;
    total.parse::<u32>().ok()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_load_sync_populates_identity() {
        let s = HomeSnapshot::load_sync();
        assert!(!s.mde_version.is_empty());
        assert!(!s.hostname.is_empty());
        assert!(!s.fedora_release.is_empty());
        assert!(s.capabilities.is_empty(), "sync load defers capability rows");
        assert!(s.mackesd_reachable, "default assumes reachable until probed");
    }

    #[test]
    fn view_renders_without_panic() {
        let panel = HomePanel::new();
        let _ = panel.view();
    }

    #[test]
    fn view_renders_with_capabilities() {
        let mut panel = HomePanel::new();
        panel.snapshot.capabilities = build_all_rows_with_unknown_status();
        panel.snapshot.mackesd_reachable = false;
        let _ = panel.view();
    }

    #[test]
    fn capability_status_active_is_green() {
        let s = CapabilityStatus::Active;
        let c = s.color();
        assert!(c.g > c.r && c.g > c.b, "active pill must read as green");
        assert_eq!(s.label(), "Active");
        assert_eq!(s.icon(), Icon::StatusOk);
    }

    #[test]
    fn capability_status_setup_needed_is_yellow() {
        let s = CapabilityStatus::SetupNeeded;
        let c = s.color();
        assert!(c.r > 0.5 && c.g > 0.5 && c.b < 0.5, "yellow pill");
        assert_eq!(s.label(), "Setup needed");
    }

    #[test]
    fn capability_status_failed_is_red_with_detail() {
        let s = CapabilityStatus::Failed { detail: "x11vnc dead".into() };
        let c = s.color();
        assert!(c.r > c.g && c.r > c.b, "failed pill must read as red");
        assert_eq!(s.label(), "Failed");
        assert_eq!(s.detail(), Some("x11vnc dead"));
    }

    #[test]
    fn capability_status_coming_soon_carries_version() {
        let s = CapabilityStatus::ComingSoon { version: "5.0.0" };
        assert_eq!(s.label(), "Coming in v5.0.0");
        assert_eq!(s.icon(), Icon::StatusUnknown);
    }

    #[test]
    fn coming_soon_rows_have_no_jump_target() {
        let rows = build_all_rows_with_unknown_status();
        let files = rows.iter().find(|r| r.id == CapabilityId::Files).unwrap();
        let phone = rows.iter().find(|r| r.id == CapabilityId::Phone).unwrap();
        let voice = rows.iter().find(|r| r.id == CapabilityId::Voice).unwrap();
        assert!(files.jump.is_none());
        assert!(phone.jump.is_none());
        assert!(voice.jump.is_none());
    }

    #[test]
    fn row_ordering_matches_spec() {
        let rows = build_all_rows_with_unknown_status();
        let order: Vec<CapabilityId> = rows.iter().map(|r| r.id).collect();
        assert_eq!(
            order,
            vec![
                CapabilityId::Mesh,
                CapabilityId::Peers,
                CapabilityId::Files,
                CapabilityId::Ssh,
                CapabilityId::Rdp,
                CapabilityId::Vnc,
                CapabilityId::Services,
                CapabilityId::Phone,
                CapabilityId::Voice,
                CapabilityId::Fleet,
                CapabilityId::Notifications,
            ]
        );
    }

    #[test]
    fn row_count_is_eleven() {
        assert_eq!(build_all_rows_with_unknown_status().len(), 11);
    }

    #[test]
    fn live_rows_jump_to_documented_panels() {
        let rows = build_all_rows_with_unknown_status();
        let lookup = |id: CapabilityId| {
            rows.iter()
                .find(|r| r.id == id)
                .and_then(|r| r.jump)
        };
        assert_eq!(lookup(CapabilityId::Mesh), Some((Group::Network, "mesh_control")));
        assert_eq!(lookup(CapabilityId::Peers), Some((Group::Network, "mesh_topology")));
        assert_eq!(lookup(CapabilityId::Ssh), Some((Group::Network, "mesh_ssh")));
        assert_eq!(lookup(CapabilityId::Rdp), Some((Group::Network, "remote_desktop")));
        assert_eq!(lookup(CapabilityId::Vnc), Some((Group::Network, "remote_desktop")));
        assert_eq!(lookup(CapabilityId::Services), Some((Group::Network, "mesh_services")));
        assert_eq!(lookup(CapabilityId::Fleet), Some((Group::Fleet, "playbooks")));
        assert_eq!(lookup(CapabilityId::Notifications), Some((Group::System, "notifications")));
    }

    #[test]
    fn extract_first_revision_id_picks_first_token() {
        let raw = "array [\n  string \"r-2026-05-24-0042\"\n  string \"r-2026-05-23-0017\"\n]";
        assert_eq!(extract_first_revision_id(raw), Some("r-2026-05-24-0042".into()));
    }

    #[test]
    fn extract_first_revision_id_handles_empty_array() {
        assert_eq!(extract_first_revision_id("array [\n]"), None);
    }

    #[test]
    fn humanize_revision_age_extracts_date() {
        assert_eq!(humanize_revision_age("r-2026-05-24-0042"), "2026-05-24");
        assert_eq!(humanize_revision_age("garbage"), "garbage");
    }

    #[test]
    fn humanize_transport_translates_known_kinds() {
        assert_eq!(humanize_transport("nebula_direct"), "direct UDP");
        assert_eq!(humanize_transport("nebula_lighthouse_relay"), "lighthouse relay");
        assert_eq!(humanize_transport("nebula_https443"), "HTTPS-443 fallback");
        assert_eq!(humanize_transport("kdc_tls"), "KDC2 TLS");
        // Unknown transports fall back to a humanized form.
        assert_eq!(humanize_transport("future_thing"), "future thing");
    }

    #[test]
    fn extract_json_string_field_finds_active_transport() {
        let raw = r#"{"is_lighthouse":false,"ca_epoch":3,"peer_count":4,"mesh_id":"m1","active_transport":"nebula_direct"}"#;
        assert_eq!(
            extract_json_string_field(raw, "active_transport"),
            Some("nebula_direct".into())
        );
    }

    #[test]
    fn extract_json_string_field_returns_none_for_missing() {
        let raw = r#"{"present":"yes"}"#;
        assert!(extract_json_string_field(raw, "missing").is_none());
    }

    #[test]
    fn extract_peer_count_parses_sub_status() {
        let row = CapabilityRow {
            id: CapabilityId::Peers,
            name: "x",
            description: "x",
            icon: Icon::Peer,
            status: CapabilityStatus::Active,
            sub_status: Some("3 of 7 peers online".into()),
            jump: None,
        };
        assert_eq!(extract_peer_count(&row), Some(7));
    }

    #[test]
    fn extract_peer_count_returns_none_without_sub() {
        let row = CapabilityRow {
            id: CapabilityId::Peers,
            name: "x",
            description: "x",
            icon: Icon::Peer,
            status: CapabilityStatus::Unknown,
            sub_status: None,
            jump: None,
        };
        assert_eq!(extract_peer_count(&row), None);
    }

    #[test]
    fn message_refreshed_preserves_capabilities() {
        let mut panel = HomePanel::new();
        panel.snapshot.capabilities = build_all_rows_with_unknown_status();
        panel.snapshot.mackesd_reachable = false;
        let new_snap = HomeSnapshot::load_sync();
        let _ = panel.update(Message::Refreshed(new_snap));
        assert_eq!(panel.snapshot.capabilities.len(), 11, "refresh keeps capability rows");
        assert!(!panel.snapshot.mackesd_reachable, "refresh keeps mackesd_reachable");
    }

    #[test]
    fn message_capabilities_refreshed_updates_state() {
        let mut panel = HomePanel::new();
        assert!(panel.snapshot.capabilities.is_empty());
        let rows = build_all_rows_with_unknown_status();
        let _ = panel.update(Message::CapabilitiesRefreshed {
            rows: rows.clone(),
            mackesd_reachable: true,
        });
        assert_eq!(panel.snapshot.capabilities, rows);
        assert!(panel.snapshot.mackesd_reachable);
    }
}
