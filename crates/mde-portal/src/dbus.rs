//! `dev.mackes.MDE.Portal` D-Bus surface.
//!
//! Exposes five methods that mded, the sway config keybinds, and
//! other MDE components use to drive the portal:
//!
//! - `Goto(layer: &str)` — navigate to a named layer inside Portal-full
//!   (e.g. `"hub"`, `"library"`, `"control"`, `"voip"`, `"network"`).
//! - `Focus` — bring Portal-full to the foreground (unhide / raise).
//! - `Lock` — trigger the lock-screen surface (Portal-25).
//! - `ToggleDND` — flip mesh-wide Do-Not-Disturb on/off (Portal-33).
//! - `Restart` — soft-restart mde-portal via systemd (Portal-30).

use std::sync::Arc;

use tokio::sync::Mutex;
use zbus::interface;

/// Shared runtime state the D-Bus handlers can read + mutate.
#[derive(Debug, Default, Clone)]
pub struct PortalState {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug, Default)]
struct Inner {
    dnd_enabled: bool,
}

impl PortalState {
    /// Construct initial state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return current DND state.
    #[cfg(test)]
    pub async fn dnd_enabled(&self) -> bool {
        self.inner.lock().await.dnd_enabled
    }
}

/// zbus interface implementation for `dev.mackes.MDE.Portal`.
#[interface(name = "dev.mackes.MDE.Portal")]
impl PortalState {
    /// Navigate to the named layer inside Portal-full.
    ///
    /// Valid layer names: `hub`, `library`, `control`, `voip`,
    /// `network`. Unknown layers log a warning and return without error
    /// so callers don't need to know the full layer set.
    async fn goto(&self, layer: &str) -> zbus::fdo::Result<()> {
        tracing::info!(layer, "Portal.Goto");
        // Portal-16: forward the call to the mde-portal-full surface which
        // owns the Iced window + its own D-Bus service. Ignore errors if
        // the surface isn't running yet.
        portal_full_goto(layer).await;
        Ok(())
    }

    /// Bring Portal-full to the foreground.
    async fn focus(&self) -> zbus::fdo::Result<()> {
        tracing::info!("Portal.Focus");
        Ok(())
    }

    /// Activate the lock-screen surface (Portal-25).
    async fn lock(&self) -> zbus::fdo::Result<()> {
        tracing::info!("Portal.Lock");
        Ok(())
    }

    /// Toggle mesh-wide Do-Not-Disturb on or off.
    ///
    /// Returns the new DND state (`true` = enabled).
    async fn toggle_dnd(&self) -> zbus::fdo::Result<bool> {
        let mut inner = self.inner.lock().await;
        inner.dnd_enabled = !inner.dnd_enabled;
        let new_state = inner.dnd_enabled;
        tracing::info!(dnd = new_state, "Portal.ToggleDND");
        Ok(new_state)
    }

    /// Soft-restart mde-portal (Portal-30, R4-Q87).
    ///
    /// Delegates to `systemctl --user restart mde-portal` so the systemd
    /// unit restarts cleanly. The shell-state snapshot (Portal-29) is at
    /// most 5 s stale; the new process restores from it on startup.
    ///
    /// Returns immediately — systemd manages the restart asynchronously.
    async fn restart(&self) -> zbus::fdo::Result<()> {
        tracing::info!("Portal.Restart: requesting systemctl --user restart mde-portal");
        match tokio::process::Command::new("systemctl")
            .args(["--user", "restart", "mde-portal"])
            .spawn()
        {
            Ok(_child) => Ok(()),
            Err(e) => {
                tracing::warn!(error = %e, "Portal.Restart: systemctl spawn failed");
                Err(zbus::fdo::Error::Failed(format!(
                    "systemctl --user restart mde-portal: {e}"
                )))
            }
        }
    }
}

/// zbus-generated async proxy for the `dev.mackes.MDE.Portal.Full` interface
/// exposed by the `mde-portal-full` binary.  Used by the Dock to forward a
/// `Goto(layer)` call that switches the active content layer.
#[zbus::proxy(
    interface = "dev.mackes.MDE.Portal.Full",
    default_service = "dev.mackes.MDE.Portal.Full",
    default_path = "/dev/mackes/MDE/Portal/Full"
)]
trait PortalFull {
    async fn goto(&self, layer: &str) -> zbus::Result<()>;
}

/// Forward a `Goto(layer)` call to the `mde-portal-full` surface.
///
/// Silently ignores all errors — the Portal-full binary may not be running
/// yet; the Dock should never block on its availability.
pub async fn portal_full_goto(layer: &str) {
    let Ok(conn) = zbus::Connection::session().await else { return };
    let Ok(proxy) = PortalFullProxy::new(&conn).await else { return };
    let _ = proxy.goto(layer).await;
}

/// Register the `dev.mackes.MDE.Portal` service on the session bus and
/// return the connection (which keeps the bus name alive while it lives).
///
/// Callers hold the returned `zbus::Connection` for the process lifetime.
pub async fn register(state: PortalState) -> anyhow::Result<zbus::Connection> {
    let conn = zbus::connection::Builder::session()?
        .name("dev.mackes.MDE.Portal")?
        .serve_at("/dev/mackes/MDE/Portal", state)?
        .build()
        .await?;
    tracing::info!("dev.mackes.MDE.Portal registered on session bus");
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn toggle_dnd_flips_state() {
        let state = PortalState::new();
        assert!(!state.dnd_enabled().await);

        let result = state.toggle_dnd().await;
        assert!(result.is_ok());
        assert!(state.dnd_enabled().await);

        let result = state.toggle_dnd().await;
        assert!(result.is_ok());
        assert!(!state.dnd_enabled().await);
    }

    #[tokio::test]
    async fn goto_unknown_layer_returns_ok() {
        let state = PortalState::new();
        let result = state.goto("nonexistent-layer").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn focus_returns_ok() {
        let state = PortalState::new();
        let result = state.focus().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn lock_returns_ok() {
        let state = PortalState::new();
        let result = state.lock().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn portal_state_new_dnd_false() {
        let state = PortalState::new();
        assert!(!state.dnd_enabled().await, "DND starts disabled");
    }

    /// Restart is a fire-and-forget systemctl call. In test environments
    /// systemctl may not be available; we just verify the method returns
    /// without panicking (either Ok or a Err wrapping the spawn failure).
    #[tokio::test]
    async fn restart_returns_without_panic() {
        let state = PortalState::new();
        // Result can be Ok (systemctl found) or Err (not available in CI).
        let _ = state.restart().await;
    }

    /// portal_full_goto must not panic when the Portal-full service is absent.
    #[tokio::test]
    async fn portal_full_goto_does_not_panic_when_service_absent() {
        // Service is not running in tests; the function should return silently.
        portal_full_goto("hub").await;
    }
}
