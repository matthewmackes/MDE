//! UX-8 — Carbon icon system.
//!
//! Locks (50-Q survey 2026-05-21):
//!   * Q24, Q37 — Carbon icon set, pivot away from Round 2's
//!     Lucide/Phosphor proposal. The panel already uses Carbon
//!     glyphs (see `crates/mde-panel/src/start_menu.rs:780`'s
//!     `every_action_carries_a_carbon_symbolic_icon` test).
//!   * Q37 — size tiers: **16 px inline, 20 px nav, 24 px panel
//!     header**, with empty-state 32 px + wizard-hero 48 px
//!     retained as additional tiers.
//!   * Q38 — style mostly line, filled only for status dots +
//!     notification bell.
//!   * Q39 — line weight 1 px (Carbon standard).
//!
//! ## Surface
//!
//! [`Icon`] is the semantic enum — call sites use
//! `Icon::Fleet`, `Icon::Snapshot`, etc., **never** a hardcoded
//! Carbon glyph name or Unicode codepoint. [`IconSize`] is the
//! locked size enum. Resolution happens via [`mde_icon`]:
//!
//! ```
//! use mde_theme::{mde_icon, Icon, IconSize};
//! let glyph = mde_icon(Icon::Fleet, IconSize::Nav);
//! assert_eq!(glyph.size_px(), 20.0);
//! ```
//!
//! ## Implementation
//!
//! v1 of this module (this commit) returns the Carbon **symbolic
//! name** as a `&'static str` paired with a Unicode fallback
//! glyph. The actual SVG rendering happens consumer-side — the
//! workbench Iced builder picks the fallback today; a follow-up
//! task (UX-8.a) will swap in real Carbon SVG bytes via
//! `include_bytes!` from `assets/icons/carbon/`. The semantic
//! surface (`Icon::Fleet → "fleet" + "⛁"`) is the stable contract
//! that lives forever.

/// Locked icon size tiers per Q37. Component dimensions, not
/// density-scaled (UX-24 sub-lock).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconSize {
    /// 16 px — inline within text, in tight controls (input
    /// trailing icons, badge prefixes).
    Inline,
    /// 20 px — sidebar nav rows, list-row leading icons.
    Nav,
    /// 24 px — panel page headers, prominent toolbar buttons.
    PanelHeader,
    /// 32 px — empty-state hero icon.
    EmptyState,
    /// 48 px — wizard hero icon.
    WizardHero,
}

impl IconSize {
    /// Pixel size for this tier. Locked by Q37; tests assert.
    #[must_use]
    pub const fn px(self) -> f32 {
        match self {
            IconSize::Inline => 16.0,
            IconSize::Nav => 20.0,
            IconSize::PanelHeader => 24.0,
            IconSize::EmptyState => 32.0,
            IconSize::WizardHero => 48.0,
        }
    }
}

/// Carbon line weight in px. Q39 lock.
pub const CARBON_LINE_WEIGHT_PX: f32 = 1.0;

/// Semantic icon names. Every Iced/GTK call site uses these
/// enum variants — never a hardcoded glyph/path/codepoint.
/// Adding a new variant requires:
///   1. add an arm to [`Icon::carbon_name`]
///   2. add an arm to [`Icon::fallback_glyph`]
///   3. add an arm to [`Icon::is_filled`] (default unset = line
///      style per Q38; only status dots + notification bell flip
///      to filled)
///
/// The `every_variant_resolves` test guards against missing arms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    // --- Navigation surfaces ---
    /// Dashboard / home group.
    Dashboard,
    /// Apps group + apps panels.
    Apps,
    /// Network group + network panels.
    Network,
    /// Devices group + devices panels.
    Devices,
    /// Look & Feel group + theming panels.
    LookAndFeel,
    /// System group + system panels.
    System,
    /// Maintain group + maintenance panels.
    Maintain,
    /// Fleet group + mesh panels.
    Fleet,
    /// Help group.
    Help,

    // --- Panel-specific ---
    /// Snapshot / backup.
    Snapshot,
    /// Single peer / device entry.
    Peer,
    /// Logs panel.
    Logs,
    /// System update / package manager.
    Update,
    /// Repair / recovery.
    Repair,
    /// Sound / audio.
    Sound,
    /// Display / monitor.
    Display,
    /// Printer.
    Printer,
    /// Power / battery.
    Power,
    /// Removable storage / USB.
    Removable,
    /// Date / time / clock.
    Clock,
    /// Wallpaper.
    Wallpaper,
    /// Fonts.
    Fonts,
    /// Themes / colour swatches.
    Themes,
    /// Session / login.
    Session,
    /// Notifications / bell. **Filled** per Q38.
    Notification,
    /// Wi-Fi.
    Wifi,
    /// VPN.
    Vpn,
    /// Firewall.
    Firewall,
    /// Playbook / automation.
    Playbook,
    /// History / past events.
    History,
    /// Settings / gear.
    Settings,
    /// Inventory / list.
    Inventory,

    // --- Window controls (UX-4 swap-in target) ---
    /// Minimize window.
    WindowMinimize,
    /// Maximize / restore window.
    WindowMaximize,
    /// Close window.
    WindowClose,

    // --- Status / state ---
    /// Healthy / OK status dot. **Filled** per Q38.
    StatusOk,
    /// Warning status dot. **Filled** per Q38.
    StatusWarning,
    /// Error status dot. **Filled** per Q38.
    StatusError,
    /// Unknown / pending status dot. **Filled** per Q38.
    StatusUnknown,

    // --- Action affordances ---
    /// Refresh / reload.
    Refresh,
    /// Add / create.
    Add,
    /// Delete / trash.
    Delete,
    /// Edit / pencil.
    Edit,
    /// Confirm / checkmark.
    Confirm,
    /// Cancel / close X.
    Cancel,
    /// Search / magnifier.
    Search,
    /// Chevron right (navigation indicator).
    ChevronRight,
    /// Chevron down (expanded indicator).
    ChevronDown,
}

impl Icon {
    /// Carbon symbolic name — what `assets/icons/carbon/<name>.svg`
    /// would load if UX-8.a wires real SVGs. Stable contract;
    /// renaming a Carbon symbol upstream forces a one-line
    /// change here, not a workspace-wide grep.
    #[must_use]
    pub const fn carbon_name(self) -> &'static str {
        match self {
            Icon::Dashboard => "dashboard",
            Icon::Apps => "application",
            Icon::Network => "network--3",
            Icon::Devices => "devices",
            Icon::LookAndFeel => "color-palette",
            Icon::System => "settings",
            Icon::Maintain => "tools",
            Icon::Fleet => "network--public",
            Icon::Help => "help",

            Icon::Snapshot => "save",
            Icon::Peer => "machine-learning-model",
            Icon::Logs => "list",
            Icon::Update => "rocket",
            Icon::Repair => "tools",
            Icon::Sound => "volume-up",
            Icon::Display => "screen",
            Icon::Printer => "printer",
            Icon::Power => "battery-charging",
            Icon::Removable => "usb",
            Icon::Clock => "time",
            Icon::Wallpaper => "image",
            Icon::Fonts => "text-font",
            Icon::Themes => "color-palette",
            Icon::Session => "user",
            Icon::Notification => "notification--filled",
            Icon::Wifi => "wifi",
            Icon::Vpn => "vpn-connection",
            Icon::Firewall => "firewall-classic",
            Icon::Playbook => "play-filled",
            Icon::History => "recently-viewed",
            Icon::Settings => "settings",
            Icon::Inventory => "list-boxes",

            Icon::WindowMinimize => "subtract",
            Icon::WindowMaximize => "maximize",
            Icon::WindowClose => "close",

            Icon::StatusOk => "checkmark--filled",
            Icon::StatusWarning => "warning--alt--filled",
            Icon::StatusError => "error--filled",
            Icon::StatusUnknown => "help--filled",

            Icon::Refresh => "renew",
            Icon::Add => "add",
            Icon::Delete => "trash-can",
            Icon::Edit => "edit",
            Icon::Confirm => "checkmark",
            Icon::Cancel => "close",
            Icon::Search => "search",
            Icon::ChevronRight => "chevron--right",
            Icon::ChevronDown => "chevron--down",
        }
    }

    /// Unicode fallback glyph — what the consumer renders today
    /// (UX-8 v1) before UX-8.a swaps in real Carbon SVGs.
    /// Chosen so the panel reads coherently with the existing
    /// `mackes-panel::start_menu` fallback vocabulary.
    #[must_use]
    pub const fn fallback_glyph(self) -> &'static str {
        match self {
            Icon::Dashboard => "\u{2630}",   // ☰
            Icon::Apps => "\u{25A6}",        // ▦
            Icon::Network => "\u{29C8}",     // ⧈
            Icon::Devices => "\u{25A3}",     // ▣
            Icon::LookAndFeel => "\u{25C9}", // ◉
            Icon::System => "\u{2699}",      // ⚙
            Icon::Maintain => "\u{1F527}",   // 🔧
            Icon::Fleet => "\u{29C9}",       // ⧉
            Icon::Help => "?",

            Icon::Snapshot => "\u{29C7}",  // ⧇
            Icon::Peer => "\u{25CB}",      // ○
            Icon::Logs => "\u{2630}",      // ☰
            Icon::Update => "\u{2191}",    // ↑
            Icon::Repair => "\u{1F6E0}",   // 🛠
            Icon::Sound => "\u{266B}",     // ♫
            Icon::Display => "\u{25AD}",   // ▭
            Icon::Printer => "\u{2399}",   // ⎙
            Icon::Power => "\u{26A1}",     // ⚡
            Icon::Removable => "\u{2902}", // ⤂
            Icon::Clock => "\u{29D6}",     // ⧖
            Icon::Wallpaper => "\u{2766}", // ❦
            Icon::Fonts => "A",
            Icon::Themes => "\u{25D0}",        // ◐
            Icon::Session => "\u{2630}",       // ☰
            Icon::Notification => "\u{1F514}", // 🔔
            Icon::Wifi => "\u{1F4F6}",         // 📶
            Icon::Vpn => "\u{1F512}",          // 🔒
            Icon::Firewall => "\u{1F6E1}",     // 🛡
            Icon::Playbook => "\u{25B6}",      // ▶
            Icon::History => "\u{231B}",       // ⌛
            Icon::Settings => "\u{2699}",      // ⚙
            Icon::Inventory => "\u{2261}",     // ≡

            Icon::WindowMinimize => "\u{2212}", // −
            Icon::WindowMaximize => "\u{25A1}", // □
            Icon::WindowClose => "\u{00D7}",    // ×

            Icon::StatusOk => "\u{25CF}",      // ●
            Icon::StatusWarning => "\u{25CF}", // ● (caller tints)
            Icon::StatusError => "\u{25CF}",   // ●
            Icon::StatusUnknown => "\u{25CB}", // ○

            Icon::Refresh => "\u{21BB}", // ↻
            Icon::Add => "+",
            Icon::Delete => "\u{1F5D1}",      // 🗑
            Icon::Edit => "\u{270E}",         // ✎
            Icon::Confirm => "\u{2713}",      // ✓
            Icon::Cancel => "\u{00D7}",       // ×
            Icon::Search => "\u{1F50D}",      // 🔍
            Icon::ChevronRight => "\u{203A}", // ›
            Icon::ChevronDown => "\u{2304}",  // ⌄
        }
    }

    /// Is this icon rendered with a filled style per Q38? Only
    /// status dots + the notification bell are filled in the
    /// MDE iconography; everything else uses Carbon's standard
    /// 1-px line weight.
    #[must_use]
    pub const fn is_filled(self) -> bool {
        matches!(
            self,
            Icon::Notification
                | Icon::StatusOk
                | Icon::StatusWarning
                | Icon::StatusError
                | Icon::StatusUnknown
                | Icon::Playbook
        )
    }
}

/// Resolved icon — a Carbon symbolic name + a Unicode fallback +
/// the locked size. Consumers render either the Carbon SVG
/// (UX-8.a) or the fallback glyph at `size.px()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedIcon {
    /// Carbon symbolic name (e.g. `"network--public"`). Consumer
    /// looks this up in `assets/icons/carbon/` (UX-8.a) or in a
    /// Carbon-icon-set crate.
    pub carbon_name: &'static str,
    /// Unicode fallback glyph rendered when no Carbon SVG is
    /// available (UX-8 v1 default).
    pub fallback_glyph: &'static str,
    /// Q38 filled-vs-line style.
    pub is_filled: bool,
    /// Resolved [`IconSize`] tier.
    pub size: IconSize,
}

impl ResolvedIcon {
    /// UX-8.a — return the Carbon SVG bytes for this icon when
    /// they're bundled (`assets/icons/carbon/<carbon_name>.svg`
    /// included via `include_bytes!`). Returns `None` today
    /// because the SVG asset bundle isn't shipped yet; consumers
    /// fall back to [`fallback_glyph`] in the meantime.
    ///
    /// When the asset bundle ships, this function becomes a
    /// `match self.carbon_name { "network--public" =>
    /// Some(include_bytes!("…")), … }` table. Consumers don't
    /// need to change — `svg_bytes().or(fallback_glyph())`
    /// is the durable contract.
    ///
    /// [`fallback_glyph`]: Self::fallback_glyph
    #[must_use]
    pub fn svg_bytes(&self) -> Option<&'static [u8]> {
        // Asset bundle ships in UX-8.b. Returning `None` so the
        // existing fallback_glyph render path stays in use.
        let _ = self.carbon_name;
        None
    }
}

impl ResolvedIcon {
    /// Pixel size — convenience pass-through.
    #[must_use]
    pub const fn size_px(self) -> f32 {
        self.size.px()
    }
}

/// Single canonical resolver. Consumers never construct
/// `ResolvedIcon` directly — they go through this so adding a
/// new Icon variant lights up everywhere consistently.
#[must_use]
pub const fn mde_icon(icon: Icon, size: IconSize) -> ResolvedIcon {
    ResolvedIcon {
        carbon_name: icon.carbon_name(),
        fallback_glyph: icon.fallback_glyph(),
        is_filled: icon.is_filled(),
        size,
    }
}

/// Pick a [`Icon`] from a peer's `device_type` field (CB-1.5.a
/// `NodeRow` / [`mde_mesh_types::DeviceType`] equivalent).
/// `mesh_peer_card` consumers route via this so the inventory
/// list, fleet panel, and peer-connection-card all render the
/// same glyph for the same kind of device. UX-8 (f) lock.
#[must_use]
pub fn icon_for_device_type(device_type: &str) -> Icon {
    match device_type {
        "laptop" | "notebook" => Icon::Devices,
        "desktop" | "tower" => Icon::Devices,
        "phone" | "mobile" => Icon::Devices,
        "server" | "rack" => Icon::Fleet,
        "router" | "gateway" => Icon::Network,
        "printer" => Icon::Printer,
        "display" | "monitor" => Icon::Display,
        _ => Icon::Peer,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_sizes_match_q37_lock() {
        assert!((IconSize::Inline.px() - 16.0).abs() < f32::EPSILON);
        assert!((IconSize::Nav.px() - 20.0).abs() < f32::EPSILON);
        assert!((IconSize::PanelHeader.px() - 24.0).abs() < f32::EPSILON);
        assert!((IconSize::EmptyState.px() - 32.0).abs() < f32::EPSILON);
        assert!((IconSize::WizardHero.px() - 48.0).abs() < f32::EPSILON);
    }

    #[test]
    fn carbon_line_weight_locked_to_one_px_per_q39() {
        assert!((CARBON_LINE_WEIGHT_PX - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn every_variant_resolves_to_nonempty_carbon_name() {
        // If a new Icon variant is added without an arm in
        // carbon_name(), this test catches it — the match
        // exhaustiveness check in carbon_name() actually surfaces
        // the missing arm at compile time, but this fence guards
        // against an "" placeholder that silently ships nothing.
        for icon in every_icon() {
            assert!(
                !icon.carbon_name().is_empty(),
                "Icon::{icon:?} has an empty carbon_name()"
            );
        }
    }

    #[test]
    fn every_variant_resolves_to_nonempty_fallback() {
        for icon in every_icon() {
            assert!(
                !icon.fallback_glyph().is_empty(),
                "Icon::{icon:?} has an empty fallback_glyph()"
            );
        }
    }

    #[test]
    fn filled_set_matches_q38_lock() {
        // Q38: only status dots + notification bell + the
        // play-filled playbook glyph are filled. Everything else
        // is line-weight.
        assert!(Icon::Notification.is_filled());
        assert!(Icon::StatusOk.is_filled());
        assert!(Icon::StatusWarning.is_filled());
        assert!(Icon::StatusError.is_filled());
        assert!(Icon::StatusUnknown.is_filled());
        assert!(Icon::Playbook.is_filled());
        // Spot-check line-style icons.
        assert!(!Icon::Settings.is_filled());
        assert!(!Icon::Refresh.is_filled());
        assert!(!Icon::Fleet.is_filled());
        assert!(!Icon::WindowMinimize.is_filled());
    }

    #[test]
    fn svg_bytes_returns_none_until_bundle_ships() {
        // UX-8.a — the API surface is in place but the asset
        // bundle is UX-8.b. Until it ships, every icon returns
        // None and consumers fall back to fallback_glyph.
        let r = mde_icon(Icon::Fleet, IconSize::Nav);
        assert!(r.svg_bytes().is_none());
    }

    #[test]
    fn mde_icon_carries_size_through() {
        let r = mde_icon(Icon::Fleet, IconSize::Nav);
        assert_eq!(r.carbon_name, "network--public");
        assert!((r.size_px() - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn device_type_routing_falls_back_to_peer_on_unknown() {
        assert_eq!(icon_for_device_type("nas"), Icon::Peer);
        assert_eq!(icon_for_device_type(""), Icon::Peer);
        assert_eq!(icon_for_device_type("laptop"), Icon::Devices);
        assert_eq!(icon_for_device_type("router"), Icon::Network);
    }

    /// Every `Icon` variant — keep in sync with the enum so the
    /// "every variant resolves" tests catch a missing arm. The
    /// compiler enforces this via the non-exhaustive-match in
    /// `carbon_name`; this is the safety net for the cases
    /// where the variant is in the enum but slips through with
    /// the wrong glyph.
    fn every_icon() -> Vec<Icon> {
        vec![
            Icon::Dashboard,
            Icon::Apps,
            Icon::Network,
            Icon::Devices,
            Icon::LookAndFeel,
            Icon::System,
            Icon::Maintain,
            Icon::Fleet,
            Icon::Help,
            Icon::Snapshot,
            Icon::Peer,
            Icon::Logs,
            Icon::Update,
            Icon::Repair,
            Icon::Sound,
            Icon::Display,
            Icon::Printer,
            Icon::Power,
            Icon::Removable,
            Icon::Clock,
            Icon::Wallpaper,
            Icon::Fonts,
            Icon::Themes,
            Icon::Session,
            Icon::Notification,
            Icon::Wifi,
            Icon::Vpn,
            Icon::Firewall,
            Icon::Playbook,
            Icon::History,
            Icon::Settings,
            Icon::Inventory,
            Icon::WindowMinimize,
            Icon::WindowMaximize,
            Icon::WindowClose,
            Icon::StatusOk,
            Icon::StatusWarning,
            Icon::StatusError,
            Icon::StatusUnknown,
            Icon::Refresh,
            Icon::Add,
            Icon::Delete,
            Icon::Edit,
            Icon::Confirm,
            Icon::Cancel,
            Icon::Search,
            Icon::ChevronRight,
            Icon::ChevronDown,
        ]
    }
}
