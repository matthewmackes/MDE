//! KDC2-6.7 — re-pair card shown on the v2.0.x → v2.1.0+
//! (and v2.0.x → v3.0.0) first-boot path.
//!
//! v2.1 dropped the upstream `kdeconnectd` wrapper for a native
//! Rust re-implementation. The pair-store format changed
//! (`~/.config/kdeconnect/` → `~/.config/mde/connect/`) and the
//! handshake keypair is generated fresh, so every previously
//! paired phone needs to be re-paired exactly once.
//!
//! The wizard detects this state by checking for the legacy
//! `~/.config/kdeconnect/` directory + the absence of
//! `~/.config/mde/connect/identity.pem`. When detected, this
//! card appears between the welcome page + the preset page;
//! when not, the card is skipped silently (fresh installs see
//! no extra page).
//!
//! Card UI is deliberately minimal — a paragraph of locked
//! copy + one "Got it" CTA. The actual re-pair flow happens
//! inside the Workbench Connect panel (KDC2-5.4-5.7); this
//! card just sets the expectation.

use std::path::{Path, PathBuf};

/// Locked card copy (single source of truth for tests + future
/// i18n).
pub const HEADLINE: &str = "Phones need re-pairing";
pub const BODY: &str = "\
MDE v2.1 replaces the upstream KDE Connect daemon with a built-in \
Rust implementation. Your previous phone pairings don't carry over \
— please re-pair each phone once from Workbench > Connect after \
finishing setup. New pairings are quick (under 30 seconds per \
phone) and the cert fingerprints are pinned for security.";
pub const CTA: &str = "Got it";

/// Sub-path of `XDG_CONFIG_HOME` (default `~/.config/`) where
/// the legacy kdeconnectd kept its store.
pub const LEGACY_KDC_DIR: &str = "kdeconnect";

/// Sub-path of `XDG_CONFIG_HOME` where the v2.1+ native store
/// lives. Matches `mde_kdc::pairing::PairingStore`'s default.
pub const NATIVE_KDC_DIR: &str = "mde/connect";

/// Identity filename inside the native store. Presence of this
/// file means we've already booted the native pair store at
/// least once + don't need to re-warn the user.
pub const NATIVE_IDENTITY_FILE: &str = "identity.pem";

/// Detect the "needs re-pair card" state from a config root.
///
///   * `config_root` is `XDG_CONFIG_HOME` (or `~/.config/`).
///   * Returns `true` only when:
///       1. `config_root/kdeconnect/` exists (legacy store present), AND
///       2. `config_root/mde/connect/identity.pem` does NOT exist
///          (native store hasn't been initialized yet).
///   * Otherwise returns `false`:
///       - Fresh install (neither dir): no warning needed.
///       - Already-migrated (native exists): user already saw
///         the card or hit the panel directly.
#[must_use]
pub fn should_show_card(config_root: &Path) -> bool {
    let legacy_exists = config_root.join(LEGACY_KDC_DIR).is_dir();
    let native_already_initialized = config_root
        .join(NATIVE_KDC_DIR)
        .join(NATIVE_IDENTITY_FILE)
        .is_file();
    legacy_exists && !native_already_initialized
}

/// Build the live `XDG_CONFIG_HOME` path the wizard runs
/// against. Used by `main.rs` so the production codepath picks
/// up `dirs::config_dir()`; tests pass an explicit path.
#[must_use]
pub fn live_config_root() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| {
        std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".config"))
            .unwrap_or_else(|_| PathBuf::from("/tmp"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn copy_is_non_empty() {
        assert!(!HEADLINE.is_empty());
        assert!(!BODY.is_empty());
        assert!(!CTA.is_empty());
    }

    #[test]
    fn body_mentions_the_re_pair_action() {
        // Lock — the user needs to understand the ask. Body
        // text must say "re-pair" somewhere; tests guard against
        // a copy edit that drops the actionable phrase.
        assert!(
            BODY.to_lowercase().contains("re-pair"),
            "body must instruct the user to re-pair: {BODY}",
        );
    }

    #[test]
    fn should_show_card_false_for_fresh_install() {
        // Empty config root → nothing to migrate, no card.
        let tmp = tempdir().unwrap();
        assert!(!should_show_card(tmp.path()));
    }

    #[test]
    fn should_show_card_true_when_legacy_present_and_native_absent() {
        let tmp = tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(LEGACY_KDC_DIR)).unwrap();
        assert!(should_show_card(tmp.path()));
    }

    #[test]
    fn should_show_card_false_when_native_already_initialized() {
        // Even if the legacy dir is still around (user didn't
        // clean up), once the native store exists we've moved on
        // — don't keep nagging.
        let tmp = tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(LEGACY_KDC_DIR)).unwrap();
        let native = tmp.path().join(NATIVE_KDC_DIR);
        std::fs::create_dir_all(&native).unwrap();
        std::fs::write(native.join(NATIVE_IDENTITY_FILE), b"key").unwrap();
        assert!(!should_show_card(tmp.path()));
    }

    #[test]
    fn should_show_card_false_when_only_native_exists() {
        // Brand-new MDE install ran the native store on first
        // launch — no legacy directory, no card.
        let tmp = tempdir().unwrap();
        let native = tmp.path().join(NATIVE_KDC_DIR);
        std::fs::create_dir_all(&native).unwrap();
        std::fs::write(native.join(NATIVE_IDENTITY_FILE), b"key").unwrap();
        assert!(!should_show_card(tmp.path()));
    }
}
