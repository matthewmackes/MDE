//! mde-wizard — first-run provisioning wizard for the Mackes
//! Desktop Environment.
//!
//! CB-1.10 port of `mackes/wizard/` — the v1.x GTK3 PyGObject
//! wizard becomes 8 Iced pages walking the user through:
//!
//! 1. **Welcome** — branded splash + start button.
//! 2. **Scan** — environment probe (CPU/RAM/disk/distro/Wayland).
//! 3. **Legacy import** — opt-in detection of XFCE/v1.x configs.
//! 4. **Preset** — pick one of the 4 shipped presets (hashbang
//!    is default).
//! 5. **Mesh passcode** — accept the 16-char shared passcode +
//!    enrol via `mded enroll`.
//! 6. **Network** — first-run NM bring-up.
//! 7. **Snapshot** — pre-apply snapshot via mackesd.
//! 8. **Apply** — run every selected birthright step.
//!
//! State is gated by `~/.config/mde/state.json`'s `provisioned`
//! flag — the binary short-circuits to "already provisioned"
//! when that's true unless `--rerun` is passed.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod pages;

/// Locked page order matching the v1.x wizard flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WizardPage {
    Welcome,
    Scan,
    LegacyImport,
    Preset,
    MeshPasscode,
    Network,
    Snapshot,
    Apply,
}

impl WizardPage {
    /// All pages in their canonical order.
    #[must_use]
    pub const fn ordered() -> [WizardPage; 8] {
        [
            WizardPage::Welcome,
            WizardPage::Scan,
            WizardPage::LegacyImport,
            WizardPage::Preset,
            WizardPage::MeshPasscode,
            WizardPage::Network,
            WizardPage::Snapshot,
            WizardPage::Apply,
        ]
    }

    /// Display label (used in the breadcrumb + browser title).
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            WizardPage::Welcome => "Welcome",
            WizardPage::Scan => "Environment scan",
            WizardPage::LegacyImport => "Legacy import",
            WizardPage::Preset => "Pick a preset",
            WizardPage::MeshPasscode => "Mesh passcode",
            WizardPage::Network => "Network",
            WizardPage::Snapshot => "Snapshot",
            WizardPage::Apply => "Apply",
        }
    }

    /// One-based index (1..=8) shown in the page header.
    #[must_use]
    pub fn index(&self) -> usize {
        Self::ordered()
            .iter()
            .position(|p| p == self)
            .map_or(0, |i| i + 1)
    }

    /// Total page count (8).
    #[must_use]
    pub const fn total() -> usize {
        Self::ordered().len()
    }

    /// Next page in the flow. `None` after Apply.
    #[must_use]
    pub fn next(&self) -> Option<WizardPage> {
        let order = Self::ordered();
        let pos = order.iter().position(|p| p == self)?;
        order.get(pos + 1).copied()
    }

    /// Previous page in the flow. `None` before Welcome.
    #[must_use]
    pub fn prev(&self) -> Option<WizardPage> {
        let order = Self::ordered();
        let pos = order.iter().position(|p| p == self)?;
        if pos == 0 {
            None
        } else {
            order.get(pos - 1).copied()
        }
    }
}

/// Persistence state for the wizard. Mirrors the v1.x
/// `~/.config/mackes-shell/state.json` shape, now keyed under
/// `~/.config/mde/state.json` per Phase 0.5.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WizardState {
    /// Whether the wizard has completed at least once on this
    /// account. The binary skips first-run mode when true.
    pub provisioned: bool,
    /// Chosen preset name (default `hashbang`).
    pub preset: String,
    /// Mesh passcode (16 chars, shared per fleet).
    pub mesh_passcode: String,
    /// Whether the user opted into legacy import.
    pub legacy_import_opted_in: bool,
    /// Whether a pre-apply snapshot was created.
    pub snapshot_created: bool,
}

impl WizardState {
    /// Default config path: `$XDG_CONFIG_HOME/mde/state.json`.
    #[must_use]
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .map(|d| d.join("mde/state.json"))
            .unwrap_or_else(|| PathBuf::from("/tmp/mde-state.json"))
    }

    /// Load from disk; returns the default on missing / malformed.
    #[must_use]
    pub fn load(path: &std::path::Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Persist to disk (creates parent dir if missing).
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn eight_pages_in_locked_order() {
        let pages = WizardPage::ordered();
        assert_eq!(pages.len(), 8);
        assert_eq!(WizardPage::total(), 8);
    }

    #[test]
    fn index_is_one_based() {
        assert_eq!(WizardPage::Welcome.index(), 1);
        assert_eq!(WizardPage::Apply.index(), 8);
    }

    #[test]
    fn next_walks_forward_to_apply() {
        let mut p = WizardPage::Welcome;
        let mut count = 1;
        while let Some(next) = p.next() {
            p = next;
            count += 1;
        }
        assert_eq!(count, 8);
        assert_eq!(p, WizardPage::Apply);
    }

    #[test]
    fn prev_walks_back_to_welcome() {
        let mut p = WizardPage::Apply;
        while let Some(prev) = p.prev() {
            p = prev;
        }
        assert_eq!(p, WizardPage::Welcome);
    }

    #[test]
    fn welcome_has_no_prev() {
        assert!(WizardPage::Welcome.prev().is_none());
    }

    #[test]
    fn apply_has_no_next() {
        assert!(WizardPage::Apply.next().is_none());
    }

    #[test]
    fn every_page_has_distinct_label() {
        let labels: std::collections::HashSet<_> =
            WizardPage::ordered().iter().map(|p| p.label()).collect();
        assert_eq!(labels.len(), 8);
    }

    #[test]
    fn default_state_is_unprovisioned() {
        let s = WizardState::default();
        assert!(!s.provisioned);
        assert!(s.preset.is_empty());
        assert!(s.mesh_passcode.is_empty());
    }

    #[test]
    fn save_then_load_round_trips() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("state.json");
        let s = WizardState {
            provisioned: true,
            preset: "hashbang".into(),
            mesh_passcode: "0123456789ABCDEF".into(),
            legacy_import_opted_in: false,
            snapshot_created: true,
        };
        s.save(&path).unwrap();
        let loaded = WizardState::load(&path);
        assert!(loaded.provisioned);
        assert_eq!(loaded.preset, "hashbang");
        assert_eq!(loaded.mesh_passcode, "0123456789ABCDEF");
        assert!(loaded.snapshot_created);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let tmp = tempdir().unwrap();
        let loaded = WizardState::load(&tmp.path().join("absent.json"));
        assert!(!loaded.provisioned);
    }

    #[test]
    fn load_malformed_returns_default() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("bad.json");
        std::fs::write(&path, "not json").unwrap();
        assert!(!WizardState::load(&path).provisioned);
    }

    #[test]
    fn default_path_ends_with_state_json() {
        let p = WizardState::default_path();
        assert!(p.ends_with("state.json"));
    }
}
