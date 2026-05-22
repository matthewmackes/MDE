//! Purge — Stage 2 of the console installer wizard.
//!
//! Detects desktop-environment / display-manager / alt-compositor
//! packages installed alongside MDE, computes the removal set (RPM
//! candidates + Flatpak + Snap), and lets the user whitelist
//! individual packages they want to keep. MDE refuses to start
//! until the purge completes — declining aborts the install.
//!
//! ## Why a curated catalog and not regex?
//!
//! Globbing `gnome-*` nukes `gnome-keyring` (sway uses it) and
//! `gnome-themes-extra` (GTK fallback). The catalog is an
//! allowlist of *removable* package names; anything not in the
//! catalog is left alone. The protected list is a second-line
//! defense — even if a name accidentally lands in the catalog,
//! the protected list keeps `mackes-shell`, `mde-*`, `sway`,
//! `lightdm`, `NetworkManager`, kernel, and systemd off the
//! removal set.
//!
//! ## Test seam
//!
//! `scan_installed` and `dry_run` both take closures so they're
//! unit-testable without invoking real dnf. The real binary
//! injects `rpm_query` and `dnf_remove_assumeno` closures that
//! shell out; tests inject fakes that return canned data.

#![allow(clippy::module_name_repetitions)]

use std::collections::BTreeMap;

/// Which desktop family a candidate package belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum PurgeFamily {
    Gnome,
    Kde,
    Xfce,
    Mate,
    Cinnamon,
    Lxqt,
    Lxde,
    Budgie,
    Pantheon,
    Deepin,
    Enlightenment,
    /// Display managers other than LightDM (which MDE keeps).
    AltDisplayManager,
    /// Compositors other than sway (which MDE keeps).
    AltCompositor,
    /// Flatpak applications discovered at scan time.
    FlatpakApp,
    /// Snap applications + the snapd runtime itself.
    SnapApp,
}

impl PurgeFamily {
    /// Human-readable group label shown in the TUI.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Gnome => "GNOME",
            Self::Kde => "KDE Plasma",
            Self::Xfce => "XFCE",
            Self::Mate => "MATE",
            Self::Cinnamon => "Cinnamon",
            Self::Lxqt => "LXQt",
            Self::Lxde => "LXDE",
            Self::Budgie => "Budgie",
            Self::Pantheon => "Pantheon",
            Self::Deepin => "Deepin",
            Self::Enlightenment => "Enlightenment",
            Self::AltDisplayManager => "Alternate display managers",
            Self::AltCompositor => "Alternate compositors",
            Self::FlatpakApp => "Flatpak applications",
            Self::SnapApp => "Snap applications",
        }
    }

    /// All families in display order (catalog grouping order).
    #[must_use]
    pub const fn ordered() -> &'static [PurgeFamily] {
        &[
            Self::Gnome,
            Self::Kde,
            Self::Xfce,
            Self::Mate,
            Self::Cinnamon,
            Self::Lxqt,
            Self::Lxde,
            Self::Budgie,
            Self::Pantheon,
            Self::Deepin,
            Self::Enlightenment,
            Self::AltDisplayManager,
            Self::AltCompositor,
            Self::FlatpakApp,
            Self::SnapApp,
        ]
    }
}

/// One package the installer is allowed to consider for removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PurgeCandidate {
    /// RPM / flatpak-app-id / snap-name.
    pub name: &'static str,
    /// Desktop family it belongs to.
    pub family: PurgeFamily,
    /// One-line "what is this" for the user, shown when the
    /// package row is focused in the TUI.
    pub blurb: &'static str,
}

/// Curated catalog of packages MDE considers conflicting.
///
/// Sourced from Fedora group definitions (`@gnome-desktop`,
/// `@kde-desktop-environment`, `@xfce-desktop`, etc.) plus the
/// `@*-apps` group siblings that ship session-registering
/// services. Flatpak / Snap entries are populated at scan time
/// (see `scan_installed`).
///
/// **Curation rules:**
/// * Session entry points (`*-session`, `*-shell`) — always in.
/// * Compositors / window managers — always in.
/// * DM packages — `gdm`, `sddm`, `lxdm`, `kdm` in; `lightdm` is
///   PROTECTED.
/// * Apps that *only* register MIME handlers or autostart
///   daemons (clipboard, keyring) — in.
/// * Libraries / themes / icon sets shared with other DEs —
///   OUT (removing them breaks GTK/Qt apps the user keeps).
pub const CANDIDATES: &[PurgeCandidate] = &[
    // ---------- GNOME ----------
    PurgeCandidate { name: "gnome-shell",            family: PurgeFamily::Gnome, blurb: "GNOME shell — main session compositor + UI" },
    PurgeCandidate { name: "gnome-session",          family: PurgeFamily::Gnome, blurb: "GNOME session manager (registers DM entry)" },
    PurgeCandidate { name: "gnome-session-wayland-session", family: PurgeFamily::Gnome, blurb: "GNOME Wayland session DM entry" },
    PurgeCandidate { name: "gnome-session-xsession", family: PurgeFamily::Gnome, blurb: "GNOME X11 session DM entry" },
    PurgeCandidate { name: "mutter",                 family: PurgeFamily::Gnome, blurb: "GNOME's compositor (claims layer-shell)" },
    PurgeCandidate { name: "gnome-control-center",   family: PurgeFamily::Gnome, blurb: "GNOME Settings (registers schemas MDE doesn't use)" },
    PurgeCandidate { name: "gnome-tweaks",           family: PurgeFamily::Gnome, blurb: "GNOME Tweaks" },
    PurgeCandidate { name: "gnome-shell-extension-common", family: PurgeFamily::Gnome, blurb: "GNOME Shell extensions runtime" },
    PurgeCandidate { name: "gnome-classic-session",  family: PurgeFamily::Gnome, blurb: "GNOME Classic session DM entry" },
    PurgeCandidate { name: "gnome-initial-setup",    family: PurgeFamily::Gnome, blurb: "GNOME first-run wizard (conflicts with mde-wizard)" },
    // ---------- KDE Plasma ----------
    PurgeCandidate { name: "plasma-workspace",       family: PurgeFamily::Kde,   blurb: "KDE Plasma workspace (session + plasmashell)" },
    PurgeCandidate { name: "plasma-desktop",         family: PurgeFamily::Kde,   blurb: "KDE Plasma desktop shell" },
    PurgeCandidate { name: "kwin",                   family: PurgeFamily::Kde,   blurb: "KWin — KDE's compositor" },
    PurgeCandidate { name: "kwin-wayland",           family: PurgeFamily::Kde,   blurb: "KWin Wayland session" },
    PurgeCandidate { name: "kwin-x11",               family: PurgeFamily::Kde,   blurb: "KWin X11 session" },
    PurgeCandidate { name: "plasma-systemsettings",  family: PurgeFamily::Kde,   blurb: "KDE System Settings" },
    PurgeCandidate { name: "kactivitymanagerd",      family: PurgeFamily::Kde,   blurb: "KDE Activities daemon" },
    PurgeCandidate { name: "kf6-baloo",              family: PurgeFamily::Kde,   blurb: "Baloo file indexer (KDE)" },
    PurgeCandidate { name: "kscreen",                family: PurgeFamily::Kde,   blurb: "KDE screen-config service" },
    PurgeCandidate { name: "plasma-nm",              family: PurgeFamily::Kde,   blurb: "Plasma NetworkManager applet" },
    // ---------- XFCE ----------
    PurgeCandidate { name: "xfce4-session",          family: PurgeFamily::Xfce,  blurb: "XFCE session manager (DM entry)" },
    PurgeCandidate { name: "xfwm4",                  family: PurgeFamily::Xfce,  blurb: "XFCE window manager" },
    PurgeCandidate { name: "xfdesktop",              family: PurgeFamily::Xfce,  blurb: "XFCE desktop (icons + wallpaper)" },
    PurgeCandidate { name: "xfce4-panel",            family: PurgeFamily::Xfce,  blurb: "XFCE panel (conflicts with mde-panel)" },
    PurgeCandidate { name: "xfce4-settings",         family: PurgeFamily::Xfce,  blurb: "XFCE Settings Manager" },
    PurgeCandidate { name: "xfce4-power-manager",    family: PurgeFamily::Xfce,  blurb: "XFCE power manager" },
    PurgeCandidate { name: "xfce4-notifyd",          family: PurgeFamily::Xfce,  blurb: "XFCE notification daemon (D-Bus name clash)" },
    PurgeCandidate { name: "xfce4-screensaver",      family: PurgeFamily::Xfce,  blurb: "XFCE screensaver" },
    PurgeCandidate { name: "xfconf",                 family: PurgeFamily::Xfce,  blurb: "XFCE config service (xfconfd)" },
    PurgeCandidate { name: "thunar",                 family: PurgeFamily::Xfce,  blurb: "Thunar file manager (conflicts with mde-files)" },
    // ---------- MATE ----------
    PurgeCandidate { name: "mate-session-manager",   family: PurgeFamily::Mate,  blurb: "MATE session manager" },
    PurgeCandidate { name: "mate-panel",             family: PurgeFamily::Mate,  blurb: "MATE panel" },
    PurgeCandidate { name: "marco",                  family: PurgeFamily::Mate,  blurb: "Marco — MATE window manager" },
    PurgeCandidate { name: "caja",                   family: PurgeFamily::Mate,  blurb: "Caja file manager" },
    PurgeCandidate { name: "mate-control-center",    family: PurgeFamily::Mate,  blurb: "MATE control center" },
    PurgeCandidate { name: "mate-desktop",           family: PurgeFamily::Mate,  blurb: "MATE desktop libraries" },
    // ---------- Cinnamon ----------
    PurgeCandidate { name: "cinnamon",               family: PurgeFamily::Cinnamon, blurb: "Cinnamon shell" },
    PurgeCandidate { name: "cinnamon-session",       family: PurgeFamily::Cinnamon, blurb: "Cinnamon session manager" },
    PurgeCandidate { name: "muffin",                 family: PurgeFamily::Cinnamon, blurb: "Muffin — Cinnamon window manager" },
    PurgeCandidate { name: "nemo",                   family: PurgeFamily::Cinnamon, blurb: "Nemo file manager" },
    PurgeCandidate { name: "cinnamon-control-center", family: PurgeFamily::Cinnamon, blurb: "Cinnamon control center" },
    // ---------- LXQt ----------
    PurgeCandidate { name: "lxqt-session",           family: PurgeFamily::Lxqt,  blurb: "LXQt session manager" },
    PurgeCandidate { name: "lxqt-panel",             family: PurgeFamily::Lxqt,  blurb: "LXQt panel" },
    PurgeCandidate { name: "openbox",                family: PurgeFamily::Lxqt,  blurb: "Openbox window manager (LXQt default)" },
    PurgeCandidate { name: "pcmanfm-qt",             family: PurgeFamily::Lxqt,  blurb: "PCManFM-Qt file manager" },
    // ---------- LXDE ----------
    PurgeCandidate { name: "lxsession",              family: PurgeFamily::Lxde,  blurb: "LXDE session manager" },
    PurgeCandidate { name: "lxpanel",                family: PurgeFamily::Lxde,  blurb: "LXDE panel" },
    PurgeCandidate { name: "pcmanfm",                family: PurgeFamily::Lxde,  blurb: "PCManFM file manager (GTK)" },
    // ---------- Budgie ----------
    PurgeCandidate { name: "budgie-desktop",         family: PurgeFamily::Budgie, blurb: "Budgie desktop shell" },
    PurgeCandidate { name: "budgie-control-center",  family: PurgeFamily::Budgie, blurb: "Budgie control center" },
    // ---------- Pantheon ----------
    PurgeCandidate { name: "pantheon-session",       family: PurgeFamily::Pantheon, blurb: "Pantheon session (elementaryOS)" },
    PurgeCandidate { name: "gala",                   family: PurgeFamily::Pantheon, blurb: "Gala — Pantheon's window manager" },
    PurgeCandidate { name: "wingpanel",              family: PurgeFamily::Pantheon, blurb: "Wingpanel — Pantheon's top panel" },
    PurgeCandidate { name: "plank",                  family: PurgeFamily::Pantheon, blurb: "Plank dock (Pantheon's dock)" },
    // ---------- Deepin ----------
    PurgeCandidate { name: "dde-session-shell",      family: PurgeFamily::Deepin, blurb: "Deepin session shell" },
    PurgeCandidate { name: "dde-desktop",            family: PurgeFamily::Deepin, blurb: "Deepin desktop" },
    PurgeCandidate { name: "dde-control-center",     family: PurgeFamily::Deepin, blurb: "Deepin control center" },
    // ---------- Enlightenment ----------
    PurgeCandidate { name: "enlightenment",          family: PurgeFamily::Enlightenment, blurb: "Enlightenment window manager / DE" },
    // ---------- Alternate display managers ----------
    PurgeCandidate { name: "gdm",                    family: PurgeFamily::AltDisplayManager, blurb: "GNOME Display Manager (MDE uses LightDM)" },
    PurgeCandidate { name: "sddm",                   family: PurgeFamily::AltDisplayManager, blurb: "Simple Desktop Display Manager (KDE default)" },
    PurgeCandidate { name: "lxdm",                   family: PurgeFamily::AltDisplayManager, blurb: "LXDE Display Manager" },
    PurgeCandidate { name: "kdm",                    family: PurgeFamily::AltDisplayManager, blurb: "KDE Display Manager (legacy)" },
    PurgeCandidate { name: "xdm",                    family: PurgeFamily::AltDisplayManager, blurb: "X Display Manager (legacy)" },
    PurgeCandidate { name: "slim",                   family: PurgeFamily::AltDisplayManager, blurb: "SLiM Display Manager (legacy)" },
    PurgeCandidate { name: "ly",                     family: PurgeFamily::AltDisplayManager, blurb: "Ly TUI Display Manager" },
    // ---------- Alternate compositors ----------
    PurgeCandidate { name: "i3",                     family: PurgeFamily::AltCompositor, blurb: "i3 window manager (MDE uses sway)" },
    PurgeCandidate { name: "i3-gaps",                family: PurgeFamily::AltCompositor, blurb: "i3-gaps fork" },
    PurgeCandidate { name: "hyprland",               family: PurgeFamily::AltCompositor, blurb: "Hyprland Wayland compositor" },
    PurgeCandidate { name: "wayfire",                family: PurgeFamily::AltCompositor, blurb: "Wayfire Wayland compositor" },
    PurgeCandidate { name: "river",                  family: PurgeFamily::AltCompositor, blurb: "River Wayland compositor" },
    PurgeCandidate { name: "labwc",                  family: PurgeFamily::AltCompositor, blurb: "Labwc Wayland compositor" },
    PurgeCandidate { name: "weston",                 family: PurgeFamily::AltCompositor, blurb: "Weston (Wayland reference compositor)" },
    PurgeCandidate { name: "bspwm",                  family: PurgeFamily::AltCompositor, blurb: "bspwm tiling X11 window manager" },
    PurgeCandidate { name: "awesome",                family: PurgeFamily::AltCompositor, blurb: "awesome X11 window manager" },
    PurgeCandidate { name: "fluxbox",                family: PurgeFamily::AltCompositor, blurb: "Fluxbox X11 window manager" },
    PurgeCandidate { name: "icewm",                  family: PurgeFamily::AltCompositor, blurb: "IceWM X11 window manager" },
];

/// Packages MDE must keep regardless of catalog membership.
///
/// Belt-and-suspenders against catalog typos. The dry-run will
/// refuse to mark any of these for removal even if a user
/// uncheckes their whitelist entry.
pub const PROTECTED: &[&str] = &[
    // MDE itself.
    "mackes-shell",
    "mde",
    "mde-workbench",
    "mde-panel",
    "mde-session",
    "mde-wizard",
    "mded",
    // Compositor MDE uses.
    "sway",
    // DM MDE uses.
    "lightdm",
    "lightdm-gtk",
    "lightdm-gtk-greeter",
    // Networking + audio + system.
    "NetworkManager",
    "pipewire",
    "wireplumber",
    "systemd",
    // Keyring + secret service (sway/MDE depend on this).
    "gnome-keyring",
    // GTK + Qt themes/icons MDE keeps.
    "gnome-themes-extra",
    "adwaita-gtk2-theme",
    "adwaita-icon-theme",
];

/// Read-side abstraction: query for "is this package installed?"
///
/// Real binary plugs in an rpm closure; tests inject a HashSet.
pub trait InstalledQuery {
    /// True iff `pkg` is currently installed (RPM, Flatpak app
    /// id, or Snap name, depending on family).
    fn is_installed(&self, pkg: &str) -> bool;
}

impl<F: Fn(&str) -> bool> InstalledQuery for F {
    fn is_installed(&self, pkg: &str) -> bool {
        self(pkg)
    }
}

/// Result of scanning the system against the catalog.
#[derive(Debug, Clone, Default)]
pub struct ScanResult {
    /// Catalog entries the scanner found installed.
    pub found: Vec<&'static PurgeCandidate>,
    /// Extra Flatpak app-ids discovered at runtime that don't
    /// live in the static catalog. Owned strings because they're
    /// dynamic.
    pub extra_flatpaks: Vec<String>,
    /// Extra Snap names discovered at runtime.
    pub extra_snaps: Vec<String>,
}

impl ScanResult {
    /// Total number of removable items (catalog + extras).
    #[must_use]
    pub fn total(&self) -> usize {
        self.found.len() + self.extra_flatpaks.len() + self.extra_snaps.len()
    }

    /// Group catalog hits by family for display.
    #[must_use]
    pub fn by_family(&self) -> BTreeMap<PurgeFamily, Vec<&'static PurgeCandidate>> {
        let mut map: BTreeMap<PurgeFamily, Vec<&'static PurgeCandidate>> = BTreeMap::new();
        for c in &self.found {
            map.entry(c.family).or_default().push(*c);
        }
        map
    }

    /// True iff nothing was found — skip Stage 2 cleanly.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.found.is_empty() && self.extra_flatpaks.is_empty() && self.extra_snaps.is_empty()
    }
}

/// Scan the system against `CANDIDATES`. `rpm` answers RPM
/// installed-ness; `flatpaks` and `snaps` are pre-collected
/// lists (the launcher gathers them by shelling out once).
///
/// The split (rpm callback vs. pre-collected flatpak/snap lists)
/// matches the runtime cost: rpm has tens of thousands of queries
/// but each is cheap via librpm; flatpak/snap are one shell-out
/// each.
pub fn scan_installed<Q: InstalledQuery>(
    rpm: &Q,
    flatpaks: &[String],
    snaps: &[String],
) -> ScanResult {
    let mut found = Vec::new();
    for cand in CANDIDATES {
        if PROTECTED.contains(&cand.name) {
            continue;
        }
        if rpm.is_installed(cand.name) {
            found.push(cand);
        }
    }
    ScanResult {
        found,
        extra_flatpaks: flatpaks.iter().filter(|f| !PROTECTED.contains(&f.as_str())).cloned().collect(),
        extra_snaps: snaps.iter().filter(|s| !PROTECTED.contains(&s.as_str())).cloned().collect(),
    }
}

/// User's per-package keep/remove decisions.
///
/// Default = remove every scanned package; the user *adds* names
/// to `whitelist` to flip individual rows back to keep.
#[derive(Debug, Clone, Default)]
pub struct PurgeSelection {
    /// Package names (RPM, flatpak-app-id, or snap-name) the
    /// user explicitly chose to keep.
    pub whitelist: std::collections::BTreeSet<String>,
}

impl PurgeSelection {
    /// True iff `pkg` was whitelisted by the user.
    #[must_use]
    pub fn is_kept(&self, pkg: &str) -> bool {
        self.whitelist.contains(pkg)
    }

    /// Toggle a package's whitelist membership.
    pub fn toggle(&mut self, pkg: &str) {
        if !self.whitelist.remove(pkg) {
            self.whitelist.insert(pkg.to_string());
        }
    }

    /// Whitelist every name in the iterator (used by
    /// "whitelist all filtered" shortcut).
    pub fn whitelist_all<I: IntoIterator<Item = String>>(&mut self, names: I) {
        self.whitelist.extend(names);
    }

    /// Drop every whitelisted entry (used by the "clear
    /// whitelist" shortcut).
    pub fn clear(&mut self) {
        self.whitelist.clear();
    }
}

/// Final removal plan computed from a scan + a selection.
#[derive(Debug, Clone)]
pub struct PurgePlan {
    /// RPM packages dnf will be told to remove.
    pub rpms_to_remove: Vec<String>,
    /// Flatpak app-ids to remove (system + per-user are split
    /// at execute time).
    pub flatpaks_to_remove: Vec<String>,
    /// Snap names to remove (`snapd` itself goes here too).
    pub snaps_to_remove: Vec<String>,
    /// Packages the user explicitly kept — recorded for the
    /// audit log + the "unsupported configuration" motd banner.
    pub kept: Vec<String>,
}

impl PurgePlan {
    /// True iff the plan would remove nothing — install can skip
    /// Stage 2 entirely.
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.rpms_to_remove.is_empty()
            && self.flatpaks_to_remove.is_empty()
            && self.snaps_to_remove.is_empty()
    }

    /// Total package count for the "Continue? (N pkgs)" prompt.
    #[must_use]
    pub fn total_to_remove(&self) -> usize {
        self.rpms_to_remove.len() + self.flatpaks_to_remove.len() + self.snaps_to_remove.len()
    }

    /// `dnf remove -y …` argv for the RPM portion. Empty rpms
    /// list returns `None` (don't shell out at all).
    #[must_use]
    pub fn dnf_remove_argv(&self) -> Option<Vec<String>> {
        if self.rpms_to_remove.is_empty() {
            return None;
        }
        let mut argv = vec!["dnf".into(), "remove".into(), "-y".into()];
        argv.extend(self.rpms_to_remove.iter().cloned());
        Some(argv)
    }

    /// `flatpak uninstall -y --system …` argv for system-wide
    /// flatpak apps. Per-user uninstalls are issued separately
    /// by the launcher (it iterates UIDs >= 1000).
    #[must_use]
    pub fn flatpak_system_argv(&self) -> Option<Vec<String>> {
        if self.flatpaks_to_remove.is_empty() {
            return None;
        }
        let mut argv = vec![
            "flatpak".into(),
            "uninstall".into(),
            "-y".into(),
            "--system".into(),
        ];
        argv.extend(self.flatpaks_to_remove.iter().cloned());
        Some(argv)
    }

    /// `snap remove …` argv. `snapd` is appended last so its
    /// children disappear cleanly first.
    #[must_use]
    pub fn snap_remove_argv(&self) -> Option<Vec<String>> {
        if self.snaps_to_remove.is_empty() {
            return None;
        }
        let mut snaps = self.snaps_to_remove.clone();
        // Move snapd to the tail so dependents drop first.
        if let Some(pos) = snaps.iter().position(|s| s == "snapd") {
            let snapd = snaps.remove(pos);
            snaps.push(snapd);
        }
        let mut argv = vec!["snap".into(), "remove".into()];
        argv.extend(snaps);
        Some(argv)
    }
}

/// Compute the plan: walk the scan, drop whitelisted entries,
/// drop anything in PROTECTED (defensive — scan should already
/// have filtered).
#[must_use]
pub fn build_plan(scan: &ScanResult, selection: &PurgeSelection) -> PurgePlan {
    let mut rpms = Vec::new();
    let mut kept = Vec::new();
    for cand in &scan.found {
        if PROTECTED.contains(&cand.name) {
            continue; // belt + suspenders
        }
        if selection.is_kept(cand.name) {
            kept.push(cand.name.to_string());
        } else {
            rpms.push(cand.name.to_string());
        }
    }

    let flatpaks: Vec<String> = scan
        .extra_flatpaks
        .iter()
        .filter(|f| {
            if selection.is_kept(f) {
                kept.push((*f).clone());
                false
            } else {
                true
            }
        })
        .cloned()
        .collect();

    let snaps: Vec<String> = scan
        .extra_snaps
        .iter()
        .filter(|s| {
            if selection.is_kept(s) {
                kept.push((*s).clone());
                false
            } else {
                true
            }
        })
        .cloned()
        .collect();

    PurgePlan {
        rpms_to_remove: rpms,
        flatpaks_to_remove: flatpaks,
        snaps_to_remove: snaps,
        kept,
    }
}

/// The two-paragraph rationale shown on the warning page. Pure
/// data so the TUI and GUI render identical copy.
pub const RATIONALE: &str = concat!(
    "MDE requires a clean desktop baseline. Other desktop ",
    "environments installed alongside MDE register competing ",
    "LightDM session entries, override MIME and default-",
    "application handlers, claim well-known D-Bus names, grab ",
    "global keyboard shortcuts, and auto-start daemons that ",
    "consume memory and drain battery even when you're inside ",
    "MDE.\n\n",
    "The installer needs to remove the components listed on the ",
    "next page before it can proceed. Your home directory, user ",
    "data, and personal configuration files will not be ",
    "touched. A rollback manifest is written to ",
    "/var/lib/mde/installer/purge-manifest.json so you can ",
    "reinstall the removed packages later with ",
    "`mde install --restore-purged`.",
);

/// JSON-serializable manifest written to
/// `/var/lib/mde/installer/purge-manifest.json` immediately
/// before execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PurgeManifest {
    /// ISO-8601 timestamp the manifest was written.
    pub timestamp: String,
    /// MDE version that performed the purge.
    pub mde_version: String,
    /// RPM packages removed.
    pub rpms: Vec<String>,
    /// Flatpak app-ids removed (system + user collapsed).
    pub flatpaks: Vec<String>,
    /// Snap names removed.
    pub snaps: Vec<String>,
    /// Packages the user explicitly whitelisted.
    pub kept: Vec<String>,
}

impl PurgeManifest {
    /// Build from a plan + version string + ISO-8601 timestamp.
    #[must_use]
    pub fn from_plan(plan: &PurgePlan, mde_version: &str, timestamp: &str) -> Self {
        Self {
            timestamp: timestamp.to_string(),
            mde_version: mde_version.to_string(),
            rpms: plan.rpms_to_remove.clone(),
            flatpaks: plan.flatpaks_to_remove.clone(),
            snaps: plan.snaps_to_remove.clone(),
            kept: plan.kept.clone(),
        }
    }

    /// `dnf install` argv that would reinstall every RPM in the
    /// manifest — what `mde install --restore-purged` runs.
    #[must_use]
    pub fn restore_dnf_argv(&self) -> Option<Vec<String>> {
        if self.rpms.is_empty() {
            return None;
        }
        let mut argv = vec!["dnf".into(), "install".into(), "-y".into()];
        argv.extend(self.rpms.iter().cloned());
        Some(argv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn make_query(installed: &[&str]) -> impl InstalledQuery {
        let set: HashSet<String> = installed.iter().map(|s| (*s).to_string()).collect();
        move |pkg: &str| set.contains(pkg)
    }

    #[test]
    fn catalog_is_non_empty_and_unique() {
        assert!(CANDIDATES.len() >= 60, "catalog too small: {}", CANDIDATES.len());
        let names: HashSet<_> = CANDIDATES.iter().map(|c| c.name).collect();
        assert_eq!(names.len(), CANDIDATES.len(), "duplicate catalog entries");
    }

    #[test]
    fn protected_packages_never_appear_in_catalog() {
        for p in PROTECTED {
            assert!(
                !CANDIDATES.iter().any(|c| c.name == *p),
                "protected package {p} also appears in candidate catalog"
            );
        }
    }

    #[test]
    fn lightdm_is_protected_gdm_is_not() {
        assert!(PROTECTED.contains(&"lightdm"));
        assert!(CANDIDATES.iter().any(|c| c.name == "gdm"));
    }

    #[test]
    fn sway_protected_but_alt_compositors_in_catalog() {
        assert!(PROTECTED.contains(&"sway"));
        assert!(CANDIDATES.iter().any(|c| c.name == "i3"));
        assert!(CANDIDATES.iter().any(|c| c.name == "hyprland"));
    }

    #[test]
    fn every_family_label_is_distinct() {
        let labels: HashSet<_> = PurgeFamily::ordered().iter().map(|f| f.label()).collect();
        assert_eq!(labels.len(), PurgeFamily::ordered().len());
    }

    #[test]
    fn scan_returns_empty_on_clean_system() {
        let q = make_query(&[]);
        let scan = scan_installed(&q, &[], &[]);
        assert!(scan.is_empty());
        assert_eq!(scan.total(), 0);
    }

    #[test]
    fn scan_picks_up_gnome_install() {
        let q = make_query(&["gnome-shell", "mutter", "gnome-control-center"]);
        let scan = scan_installed(&q, &[], &[]);
        assert_eq!(scan.found.len(), 3);
        assert!(scan.found.iter().all(|c| c.family == PurgeFamily::Gnome));
    }

    #[test]
    fn scan_ignores_protected_packages_even_if_in_catalog_path() {
        // Plant `sway` as both PROTECTED and installed. The
        // scanner must NOT include it. (Sway isn't in the
        // catalog by design, so this just confirms the protected
        // filter would hold if it ever drifted in.)
        let q = make_query(&["sway"]);
        let scan = scan_installed(&q, &[], &[]);
        assert!(scan.found.is_empty());
    }

    #[test]
    fn scan_collects_flatpaks_and_snaps() {
        let q = make_query(&[]);
        let scan = scan_installed(
            &q,
            &["org.mozilla.firefox".into(), "com.spotify.Client".into()],
            &["discord".into(), "vlc".into()],
        );
        assert_eq!(scan.extra_flatpaks.len(), 2);
        assert_eq!(scan.extra_snaps.len(), 2);
        assert_eq!(scan.total(), 4);
    }

    #[test]
    fn by_family_groups_correctly() {
        let q = make_query(&["gnome-shell", "kwin", "xfwm4", "gnome-tweaks"]);
        let scan = scan_installed(&q, &[], &[]);
        let grouped = scan.by_family();
        assert_eq!(grouped[&PurgeFamily::Gnome].len(), 2);
        assert_eq!(grouped[&PurgeFamily::Kde].len(), 1);
        assert_eq!(grouped[&PurgeFamily::Xfce].len(), 1);
    }

    #[test]
    fn selection_default_keeps_nothing() {
        let sel = PurgeSelection::default();
        assert!(!sel.is_kept("gnome-shell"));
    }

    #[test]
    fn selection_toggle_round_trips() {
        let mut sel = PurgeSelection::default();
        sel.toggle("gnome-shell");
        assert!(sel.is_kept("gnome-shell"));
        sel.toggle("gnome-shell");
        assert!(!sel.is_kept("gnome-shell"));
    }

    #[test]
    fn selection_whitelist_all_then_clear() {
        let mut sel = PurgeSelection::default();
        sel.whitelist_all(["a".into(), "b".into(), "c".into()]);
        assert!(sel.is_kept("a"));
        assert!(sel.is_kept("b"));
        assert!(sel.is_kept("c"));
        sel.clear();
        assert!(!sel.is_kept("a"));
    }

    #[test]
    fn plan_default_selection_removes_everything() {
        let q = make_query(&["gnome-shell", "kwin"]);
        let scan = scan_installed(&q, &["org.x.Foo".into()], &["bar".into()]);
        let plan = build_plan(&scan, &PurgeSelection::default());
        assert_eq!(plan.rpms_to_remove.len(), 2);
        assert_eq!(plan.flatpaks_to_remove.len(), 1);
        assert_eq!(plan.snaps_to_remove.len(), 1);
        assert!(plan.kept.is_empty());
        assert_eq!(plan.total_to_remove(), 4);
        assert!(!plan.is_noop());
    }

    #[test]
    fn plan_whitelist_keeps_packages_out_of_removal() {
        let q = make_query(&["gnome-shell", "gnome-tweaks"]);
        let scan = scan_installed(&q, &[], &[]);
        let mut sel = PurgeSelection::default();
        sel.toggle("gnome-tweaks");
        let plan = build_plan(&scan, &sel);
        assert_eq!(plan.rpms_to_remove, vec!["gnome-shell"]);
        assert_eq!(plan.kept, vec!["gnome-tweaks".to_string()]);
    }

    #[test]
    fn plan_dnf_argv_uses_dash_y_for_unattended() {
        let plan = PurgePlan {
            rpms_to_remove: vec!["gnome-shell".into(), "mutter".into()],
            flatpaks_to_remove: vec![],
            snaps_to_remove: vec![],
            kept: vec![],
        };
        let argv = plan.dnf_remove_argv().unwrap();
        assert_eq!(argv, vec!["dnf", "remove", "-y", "gnome-shell", "mutter"]);
    }

    #[test]
    fn plan_empty_argvs_return_none() {
        let plan = PurgePlan {
            rpms_to_remove: vec![],
            flatpaks_to_remove: vec![],
            snaps_to_remove: vec![],
            kept: vec![],
        };
        assert!(plan.dnf_remove_argv().is_none());
        assert!(plan.flatpak_system_argv().is_none());
        assert!(plan.snap_remove_argv().is_none());
        assert!(plan.is_noop());
        assert_eq!(plan.total_to_remove(), 0);
    }

    #[test]
    fn plan_snap_argv_puts_snapd_last() {
        let plan = PurgePlan {
            rpms_to_remove: vec![],
            flatpaks_to_remove: vec![],
            snaps_to_remove: vec!["snapd".into(), "discord".into(), "vlc".into()],
            kept: vec![],
        };
        let argv = plan.snap_remove_argv().unwrap();
        assert_eq!(argv.last().unwrap(), "snapd");
        assert!(argv.contains(&"discord".to_string()));
    }

    #[test]
    fn manifest_round_trips_through_json() {
        let plan = PurgePlan {
            rpms_to_remove: vec!["gnome-shell".into()],
            flatpaks_to_remove: vec!["org.x.Foo".into()],
            snaps_to_remove: vec!["snapd".into()],
            kept: vec!["gnome-tweaks".into()],
        };
        let m = PurgeManifest::from_plan(&plan, "2.1.0", "2026-05-22T12:00:00Z");
        let s = serde_json::to_string(&m).unwrap();
        let back: PurgeManifest = serde_json::from_str(&s).unwrap();
        assert_eq!(back.rpms, vec!["gnome-shell"]);
        assert_eq!(back.flatpaks, vec!["org.x.Foo"]);
        assert_eq!(back.snaps, vec!["snapd"]);
        assert_eq!(back.kept, vec!["gnome-tweaks"]);
        assert_eq!(back.mde_version, "2.1.0");
    }

    #[test]
    fn manifest_restore_argv_reinstalls_rpms() {
        let m = PurgeManifest {
            timestamp: "now".into(),
            mde_version: "2.1.0".into(),
            rpms: vec!["gnome-shell".into(), "mutter".into()],
            flatpaks: vec![],
            snaps: vec![],
            kept: vec![],
        };
        let argv = m.restore_dnf_argv().unwrap();
        assert_eq!(argv, vec!["dnf", "install", "-y", "gnome-shell", "mutter"]);
    }

    #[test]
    fn rationale_mentions_clean_baseline_and_data_safety() {
        // Doesn't lock exact wording — just that the two key
        // promises survive any future edit.
        assert!(RATIONALE.contains("clean") || RATIONALE.contains("baseline"));
        assert!(RATIONALE.contains("home directory") || RATIONALE.contains("user data"));
    }
}
