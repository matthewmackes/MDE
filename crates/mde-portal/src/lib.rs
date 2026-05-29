//! Public library surface for the `mde-portal` crate.
//!
//! Most of the portal lives inside the `mde-portal` binary (`main.rs`)
//! and the `mde-portal-full` binary (`portal_full_main.rs`).  This
//! library file exists so external binaries — currently just
//! `mde-open` (Portal-35) — can reuse the URI parser and the
//! generated `dev.mackes.MDE.Portal` D-Bus proxy without duplicating
//! the source.

#![forbid(unsafe_code)]

/// `mde://` URI scheme parser (Portal-35).
pub mod uri;

/// Workspace template → swaymsg batch emitter (SWAY-5 / Portal-51).
pub mod template;

/// Generated async proxy for `dev.mackes.MDE.Portal`.
///
/// Kept separate from the server-side `dbus.rs` (which lives in the
/// `mde-portal` binary tree) so `mde-open` can link against the
/// proxy without pulling in the server's PortalState + tokio runtime
/// dependencies.
pub mod dbus_proxy {
    /// zbus-generated client proxy.
    #[zbus::proxy(
        interface = "dev.mackes.MDE.Portal",
        default_service = "dev.mackes.MDE.Portal",
        default_path = "/dev/mackes/MDE/Portal"
    )]
    pub trait Portal {
        async fn open_uri(&self, uri: &str) -> zbus::Result<String>;
        async fn goto(&self, layer: &str) -> zbus::Result<()>;
        async fn lock(&self) -> zbus::Result<()>;
        async fn focus(&self) -> zbus::Result<()>;
        async fn toggle_dnd(&self) -> zbus::Result<bool>;
        async fn restart(&self) -> zbus::Result<()>;
    }
}
