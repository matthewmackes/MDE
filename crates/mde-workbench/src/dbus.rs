//! `dev.mackes.MDE.Shell.Workbench` D-Bus surface — single-
//! instance contract + deep-link router.
//!
//! CB-1.13 lock: "`dev.mackes.MDE.Shell.Workbench` interface
//! (new) ships `Focus(slug: str)` so `mde --focus <slug>` opens
//! the running workbench at the named panel, or launches one if
//! none. Replaces the 1.x WM_CLASS-based single-instance hack."
//!
//! The bus name [`crate::single_instance::BUS_NAME`]
//! (`dev.mackes.MDE.Workbench`) is acquired by the primary
//! workbench process with `DoNotQueue` so a sibling launch sees
//! `Exists` immediately and hands off via [`WorkbenchProxy`].

use std::sync::{Mutex, OnceLock};

use zbus::interface;

/// Interface name CB-1.13 ships — sits under the `Shell.*`
/// namespace because Focus is workbench-side state mutation
/// driven by the shell IPC family (mirrors how `Shell.Settings`
/// sits under the same parent).
pub const INTERFACE_NAME: &str = "dev.mackes.MDE.Shell.Workbench";

/// Method name on [`INTERFACE_NAME`]. Lifted to a constant so
/// the client helper + the interface attribute agree without
/// duplicating literals across files.
pub const METHOD_FOCUS: &str = "Focus";

/// Server-side handler for [`INTERFACE_NAME`].
///
/// Holds no state directly — Focus requests push a slug into the
/// process-wide [`PendingFocus`] slot which the Iced subscription
/// drains on its tick. The split keeps the handler tokio-only
/// (no Iced types) so the interface attribute compiles standalone
/// for unit tests.
#[derive(Debug, Default, Clone)]
pub struct WorkbenchService;

#[interface(name = "dev.mackes.MDE.Shell.Workbench")]
impl WorkbenchService {
    /// Open the running workbench window at the named panel.
    /// Accepts the same `<group>.<panel>` slug grammar as
    /// [`crate::model::view_from_focus_slug`]. Unknown slugs
    /// short-circuit to the Dashboard landing (matches the
    /// 1.x `mackes --focus` fallback behaviour).
    async fn focus(&self, slug: &str) -> zbus::fdo::Result<()> {
        // Empty string means "just raise the window, no view
        // change" — same contract the 1.x panel honoured for
        // taskbar click-throughs.
        let trimmed = slug.trim();
        if trimmed.is_empty() {
            PendingFocus::submit(String::new());
        } else {
            PendingFocus::submit(trimmed.to_string());
        }
        Ok(())
    }

    /// Crate version for sanity-check probes
    /// (`busctl --user call … Version`).
    async fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
}

/// Cross-task focus channel — the zbus handler writes; the Iced
/// subscription reads on a 200 ms tick (cheap given Focus is a
/// user-action surface, not real-time data).
///
/// A `Mutex<Option<String>>` over a poll loop is deliberately
/// simpler than a tokio `mpsc::UnboundedReceiver` shipped through
/// a `OnceLock<Mutex<_>>` — Focus requests coalesce naturally
/// (only the latest slug matters), and the poll-tick is the same
/// rate `iced::time::every` already runs subscriptions at.
pub struct PendingFocus;

static PENDING: OnceLock<Mutex<Option<String>>> = OnceLock::new();

impl PendingFocus {
    fn slot() -> &'static Mutex<Option<String>> {
        PENDING.get_or_init(|| Mutex::new(None))
    }

    /// Write the latest focus request — overwriting any earlier
    /// unread slug (latest-wins coalescing). Returns `true`
    /// when the write happened (always true today; the Result
    /// shape leaves room for future rate-limiting).
    pub fn submit(slug: String) -> bool {
        if let Ok(mut guard) = Self::slot().lock() {
            *guard = Some(slug);
            true
        } else {
            false
        }
    }

    /// Take whatever pending slug is in the slot, leaving
    /// `None`. The Iced subscription calls this each tick.
    pub fn drain() -> Option<String> {
        Self::slot().lock().ok().and_then(|mut g| g.take())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serializes every test that touches the `PendingFocus`
    /// global. Tests in this module hold a single shared
    /// `[u8; 0]` value-less guard for their full body so
    /// concurrent runs (the default `cargo test` topology)
    /// observe sequential `submit` / `drain` interleavings.
    /// Without this guard the six `pending_focus_*` and
    /// `focus_handler_*` tests race on the process-wide slot
    /// and `cargo test` fails intermittently
    /// (OV-test-flake-1).
    static FOCUS_LOCK: Mutex<()> = Mutex::new(());

    /// Acquire the focus-test lock. Recovers from poisoning so
    /// a panicking earlier test doesn't block the rest of the
    /// suite — every test calls `reset_pending()` immediately
    /// after to scrub state.
    fn lock_focus() -> std::sync::MutexGuard<'static, ()> {
        FOCUS_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Drop the process-wide slot between tests so they don't
    /// observe each other's writes. Safe because we hold the
    /// global mutex for the swap.
    fn reset_pending() {
        if let Some(slot) = PENDING.get() {
            if let Ok(mut guard) = slot.lock() {
                *guard = None;
            }
        }
    }

    #[test]
    fn interface_name_is_under_shell_namespace() {
        assert_eq!(INTERFACE_NAME, "dev.mackes.MDE.Shell.Workbench");
        assert!(INTERFACE_NAME.starts_with("dev.mackes.MDE.Shell."));
    }

    #[test]
    fn focus_method_constant_matches_zbus_attribute_pascal_case() {
        assert_eq!(METHOD_FOCUS, "Focus");
    }

    #[test]
    fn pending_focus_drain_returns_none_on_empty_slot() {
        let _guard = lock_focus();
        reset_pending();
        assert_eq!(PendingFocus::drain(), None);
    }

    #[test]
    fn pending_focus_round_trip_through_submit_and_drain() {
        let _guard = lock_focus();
        reset_pending();
        assert!(PendingFocus::submit("network.mesh_ssh".into()));
        assert_eq!(PendingFocus::drain(), Some("network.mesh_ssh".into()));
        assert_eq!(PendingFocus::drain(), None, "drain should clear the slot");
    }

    #[test]
    fn pending_focus_coalesces_to_latest_submit() {
        let _guard = lock_focus();
        reset_pending();
        PendingFocus::submit("apps".into());
        PendingFocus::submit("network".into());
        PendingFocus::submit("help".into());
        // Only the latest survives — Focus is a user-action
        // hand-off, not an event queue.
        assert_eq!(PendingFocus::drain(), Some("help".into()));
    }

    #[tokio::test]
    async fn focus_handler_writes_into_pending_slot() {
        let _guard = lock_focus();
        reset_pending();
        let svc = WorkbenchService;
        svc.focus("look_and_feel").await.expect("focus ok");
        assert_eq!(PendingFocus::drain(), Some("look_and_feel".to_string()));
    }

    #[tokio::test]
    async fn focus_handler_normalises_whitespace_only_slug_to_empty() {
        let _guard = lock_focus();
        reset_pending();
        let svc = WorkbenchService;
        svc.focus("   ").await.expect("focus ok");
        // Whitespace-only is interpreted as "no view change,
        // just raise" — matches the 1.x taskbar contract.
        assert_eq!(PendingFocus::drain(), Some(String::new()));
    }

    #[tokio::test]
    async fn focus_handler_trims_surrounding_whitespace() {
        let _guard = lock_focus();
        reset_pending();
        let svc = WorkbenchService;
        svc.focus("  network.mesh_ssh  ").await.expect("focus ok");
        assert_eq!(PendingFocus::drain(), Some("network.mesh_ssh".to_string()));
    }

    #[tokio::test]
    async fn version_method_returns_crate_version() {
        let svc = WorkbenchService;
        assert_eq!(svc.version().await, env!("CARGO_PKG_VERSION"));
    }
}
