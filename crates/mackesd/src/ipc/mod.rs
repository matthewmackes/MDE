//! v2.0.0 Phase A.3 (locked 2026-05-19) — DBus surface served by
//! `mackesd` (and one cross-process consumer at `mackes-session`).
//!
//! Five services live on the session bus:
//!
//! | Object path                | Interface                          | Owner       |
//! |----------------------------|------------------------------------|-------------|
//! | `/org/mackes/Shell`        | `org.mackes.Shell`                 | mackesd     |
//! | `/org/mackes/Settings`     | `org.mackes.Settings`              | mackesd     |
//! | `/org/freedesktop/Notifications` | `org.freedesktop.Notifications` | mackesd     |
//! | `/org/mackes/Session`      | `org.mackes.Session`               | mackes-session |
//! | `/org/mackes/Fleet`        | `org.mackes.Fleet`                 | mackesd     |
//!
//! Phase A: every service struct compiles + the `#[interface]`
//! decoration is in place, but the handler bodies return
//! [`crate::settings::UNIMPLEMENTED`] (settings/fleet) or `()`
//! (notifications/session). Phase B fills them in.
//!
//! `Notifications` deliberately matches the spec object path
//! `/org/freedesktop/Notifications` so existing apps (notify-send,
//! libnotify, etc.) reach mackesd transparently.

#![cfg(feature = "async-services")]
// zbus's #[interface] macro expands to additional dispatch methods
// that don't carry doc comments; the workspace's #[warn(missing_docs)]
// would otherwise flag every one. Silence at the module level so the
// rest of the crate's missing_docs hygiene stays loud.
#![allow(missing_docs)]

pub mod files;
pub mod fleet;
pub mod nebula;
pub mod notifications;
pub mod session;
pub mod settings;
pub mod shell;

/// Convenience: the well-known bus name mackesd registers on the
/// session bus.
pub const MACKESD_BUS_NAME: &str = "org.mackes.mackesd";

/// Convenience: the well-known bus name mackes-session registers on
/// the session bus. Lives here (not in the mackes-session crate) so
/// every consumer (panel applets, Workbench panels) imports it from
/// one place.
pub const MACKES_SESSION_BUS_NAME: &str = "org.mackes.session";

/// Convenience: the canonical object path for each service.
pub mod paths {
    /// `/org/mackes/Shell`
    pub const SHELL: &str = "/org/mackes/Shell";
    /// `/org/mackes/Settings`
    pub const SETTINGS: &str = "/org/mackes/Settings";
    /// `/org/freedesktop/Notifications` — matches the freedesktop
    /// spec so libnotify clients work unchanged.
    pub const NOTIFICATIONS: &str = "/org/freedesktop/Notifications";
    /// `/org/mackes/Session`
    pub const SESSION: &str = "/org/mackes/Session";
    /// `/org/mackes/Fleet`
    pub const FLEET: &str = "/org/mackes/Fleet";
}
