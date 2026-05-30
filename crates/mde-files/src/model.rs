//! Data model for the Artifact Manager — Rust port of the FM_*
//! types in the prototype.
//!
//! v4.0.1 AF-* mega (2026-05-23) — Phase G migration: every
//! `&'static str` field becomes `String` so the same struct can
//! carry dummy (`DemoBackend`) data and live (`DBusBackend`,
//! `LocalFsBackend`) data without a separate wire-type per
//! source. `Copy` is dropped from the carrier structs; the
//! enum-like value types (`PinIcon`, `Mime`, `PeerKind`,
//! `PeerStatus`, `TxDir`, `LatencyBucket`, `Layout`) stay
//! `Copy` because they're tiny.
//!
//! `View::Peer` carries `String` now (was `&'static str`). The
//! `View` enum drops `Copy` as a consequence — Iced state
//! `Clone`s its `View` field on every render, so the runtime
//! cost is one Arc-free `String::clone` per render. The
//! ergonomic cost is that callsites doing `let v = self.view;`
//! become `let v = self.view.clone();`.

/// A mesh peer (the rows in the sidebar's MESH section + the
/// cards in the overview).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peer {
    pub id: String,
    pub host: String,
    pub label: String,
    pub kind: PeerKind,
    pub addr: String,
    pub status: PeerStatus,
    /// Milliseconds. `None` when the peer is offline.
    pub latency: Option<u32>,
    pub files: u32,
    pub shared: u32,
    /// Human-readable "last seen" stamp (e.g. `"now"`, `"3 min"`,
    /// `"2 h ago"`).
    pub last: String,
    pub derp: String,
}

/// The local node ("this node"). Distinguished from peers because
/// the UI tints it with the rust accent instead of the
/// success-green online dot.
#[derive(Debug, Clone, Default)]
pub struct SelfNode {
    pub id: String,
    pub host: String,
    pub label: String,
    pub addr: String,
    pub files: u32,
    pub shared: u32,
}

/// One row in a file list. `mesh` and `from` both attribute the
/// file to a peer; the prototype uses `mesh` for Downloads-style
/// listings and `from` for Inbox-style listings. The visual pill
/// is the same in both cases.
#[derive(Debug, Clone)]
pub struct FileRow {
    pub name: String,
    pub mime: Mime,
    pub size: String,
    pub age: String,
    pub mesh: Option<String>,
    pub from: Option<String>,
    /// MESHFS-11.1 — true when a `.conflict-<ts>-<host>` sibling exists
    /// in the same directory. Triggers the yellow conflict chip.
    pub has_conflict: bool,
    /// MESHFS-11.1 — filename of the conflict sibling (not a full path;
    /// relative to the same directory). `None` when `has_conflict` is false.
    pub conflict_sibling: Option<String>,
    /// MESHFS-11.1 — true while the LizardFS fleet is healing (under-
    /// replicated). Triggers the sync indicator badge on mesh-homed rows.
    pub syncing: bool,
}

impl FileRow {
    #[must_use]
    pub fn local(
        name: impl Into<String>,
        mime: Mime,
        size: impl Into<String>,
        age: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            mime,
            size: size.into(),
            age: age.into(),
            mesh: None,
            from: None,
            has_conflict: false,
            conflict_sibling: None,
            syncing: false,
        }
    }

    #[must_use]
    pub fn with_mesh(mut self, peer_host: impl Into<String>) -> Self {
        self.mesh = Some(peer_host.into());
        self
    }

    #[must_use]
    pub fn with_from(mut self, peer_host: impl Into<String>) -> Self {
        self.from = Some(peer_host.into());
        self
    }

    /// Mark this row as having a conflict sibling file.
    #[must_use]
    pub fn with_conflict(mut self, sibling_name: impl Into<String>) -> Self {
        self.has_conflict = true;
        self.conflict_sibling = Some(sibling_name.into());
        self
    }

    /// Mark this row as currently syncing (fleet healing).
    #[must_use]
    pub fn with_syncing(mut self, syncing: bool) -> Self {
        self.syncing = syncing;
        self
    }

    #[must_use]
    pub fn origin(&self) -> Option<&str> {
        self.mesh.as_deref().or(self.from.as_deref())
    }

    #[must_use]
    pub fn is_mesh(&self) -> bool {
        self.origin().is_some()
    }
}

/// A pin in the local-veil grid (Home / Documents / Pictures /…).
#[derive(Debug, Clone)]
pub struct LocalPin {
    pub id: String,
    pub name: String,
    pub path: String,
    pub icon: PinIcon,
}

/// Which icon a local pin should use. Maps onto `icons::svg_for_pin`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinIcon {
    Home,
    Doc2,
    Image,
    Doc,
    Player,
    Rust,
    Hdd,
    Trash,
}

/// A short transfer-log row in the Mesh Overview.
#[derive(Debug, Clone)]
pub struct Transfer {
    pub dir: TxDir,
    pub name: String,
    pub peer: String,
    pub size: String,
    pub age: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxDir {
    In,
    Out,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerStatus {
    Online,
    Idle,
    Offline,
    Self_,
}

impl PeerStatus {
    /// NF-12.3 (v2.5) — true when Send-To can route a file
    /// directly. Online + Self_ qualify; Idle (probe within
    /// degradation threshold) also qualifies — the router
    /// downgrades transport but still delivers. Offline is
    /// the only state where Send-To greys out + tooltip
    /// reads "Peer is offline".
    #[must_use]
    pub const fn is_reachable(self) -> bool {
        matches!(self, Self::Online | Self::Idle | Self::Self_)
    }

    /// NF-12.3 — tooltip text for the destination chip.
    /// Empty string when the peer is reachable (no
    /// tooltip needed). Non-empty when greyed out.
    #[must_use]
    pub const fn tooltip_when_offline(self) -> &'static str {
        match self {
            Self::Offline => "Peer is offline",
            _ => "",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerKind {
    Desktop,
    Server,
    Phone,
    Ci,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mime {
    Folder,
    Doc,
    Image,
    Pdf,
    Archive,
    Disk,
}

/// Current routing target for the main content area.
///
/// Default is `MeshOverview` — the mesh is the home base, not
/// the local filesystem.
///
/// v4.x AF-mesh.2 (2026-05-24) — adds `MeshHome` + `MeshHomeChild`
/// for the shared XDG dirs (Documents, Pictures, Music, Videos,
/// Downloads). Per the v5.0.0 GlusterFS lock these dirs ARE the
/// mesh — they're full-mesh-replicated over Nebula — so they
/// belong in the mesh section of the UI, not the Local one.
/// `Downloads` stays as a top-level shortcut for the common case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    MeshOverview,
    Inbox,
    Peer(String),
    Downloads,
    Local,
    /// Mesh Home landing — shows the five shared XDG dirs as
    /// cards. Clicking a card routes to `MeshHomeChild(slug)`.
    MeshHome,
    /// Browsing one of the shared XDG dirs. `slug` is one of
    /// `docs` / `pics` / `music` / `videos` / `downloads`.
    MeshHomeChild(String),
    /// MESHFS-8.1 — "Recycle Bin" view: files recoverable from the
    /// LizardFS `.trash` virtual directory within the configured
    /// retention window (default 48 h).
    MeshUndelete,
}

impl Default for View {
    fn default() -> Self {
        Self::MeshOverview
    }
}

impl View {
    /// True for any view that operates on mesh content (mesh
    /// overview, inbox, a peer folder, mesh home).
    #[must_use]
    pub fn is_mesh(&self) -> bool {
        matches!(
            self,
            Self::MeshOverview
                | Self::Inbox
                | Self::Peer(_)
                | Self::MeshHome
                | Self::MeshHomeChild(_)
                | Self::MeshUndelete
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    #[default]
    List,
    Grid,
}

/// Format an `u32` the way the prototype's `fmt()` does:
/// ≥1000 → `4.9k`, ≥10000 → `18k`.
#[must_use]
pub fn fmt_count(n: u32) -> String {
    if n >= 10_000 {
        format!("{}k", n / 1000)
    } else if n >= 1000 {
        let kilos = n as f32 / 1000.0;
        format!("{kilos:.1}k")
    } else {
        n.to_string()
    }
}

/// Latency colour bucket for peer-card meta rows. Matches
/// `lat-good` (<50 ms) and `lat-ok` (<150 ms).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatencyBucket {
    Good,
    Ok,
    Slow,
}

#[must_use]
pub fn latency_bucket(latency_ms: u32) -> LatencyBucket {
    if latency_ms < 50 {
        LatencyBucket::Good
    } else if latency_ms < 150 {
        LatencyBucket::Ok
    } else {
        LatencyBucket::Slow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_count_thresholds_match_prototype() {
        assert_eq!(fmt_count(0), "0");
        assert_eq!(fmt_count(999), "999");
        assert_eq!(fmt_count(1000), "1.0k");
        assert_eq!(fmt_count(4912), "4.9k");
        assert_eq!(fmt_count(10_000), "10k");
        assert_eq!(fmt_count(18_403), "18k");
    }

    #[test]
    fn latency_buckets_split_at_50_and_150() {
        assert_eq!(latency_bucket(14), LatencyBucket::Good);
        assert_eq!(latency_bucket(49), LatencyBucket::Good);
        assert_eq!(latency_bucket(50), LatencyBucket::Ok);
        assert_eq!(latency_bucket(149), LatencyBucket::Ok);
        assert_eq!(latency_bucket(220), LatencyBucket::Slow);
    }

    #[test]
    fn view_is_mesh_recognises_peer_variants() {
        assert!(View::MeshOverview.is_mesh());
        assert!(View::Inbox.is_mesh());
        assert!(View::Peer("pine".into()).is_mesh());
        assert!(!View::Downloads.is_mesh());
        assert!(!View::Local.is_mesh());
    }

    #[test]
    fn view_is_mesh_recognises_mesh_home_variants() {
        assert!(View::MeshHome.is_mesh());
        assert!(View::MeshHomeChild("docs".into()).is_mesh());
        assert!(View::MeshHomeChild("pics".into()).is_mesh());
    }

    #[test]
    fn file_row_origin_prefers_mesh_then_from() {
        let r = FileRow::local("a", Mime::Doc, "1 KB", "now").with_mesh("pine.mesh");
        assert_eq!(r.origin(), Some("pine.mesh"));
        assert!(r.is_mesh());

        let r2 = FileRow::local("b", Mime::Doc, "1 KB", "now").with_from("oak.mesh");
        assert_eq!(r2.origin(), Some("oak.mesh"));

        let r3 = FileRow::local("c", Mime::Doc, "1 KB", "now");
        assert_eq!(r3.origin(), None);
        assert!(!r3.is_mesh());
    }
}
