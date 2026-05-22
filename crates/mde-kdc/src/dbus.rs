//! KDC2-3.3 — D-Bus host scaffold.
//!
//! Exposes the `dev.mackes.MDE.Connect` interface on the user
//! session bus at `/dev/mackes/MDE/Connect`. Concrete methods
//! land in KDC2-3.4 (`ListDevices` + `GetDevice`), 3.5
//! (`PairDevice` + `UnpairDevice`), 3.6 (`RingDevice` +
//! `SendSms` + `SendFile`), 3.9 (`DeviceAdded` / `Removed` /
//! `Updated` signals).
//!
//! Single-instance guard via the standard zbus name-request
//! flow: the bus refuses the name if another mde-kdc instance
//! already owns it, surfacing as
//! `DbusError::NameAlreadyAcquired`. The mackesd supervisor
//! treats this as a fatal startup error (no point running two
//! Connect hosts on the same session bus).
//!
//! Bus + interface naming follows the freedesktop conventions
//! the v2.1 KDC2 lock pinned:
//!   * Bus name:   `dev.mackes.MDE.Connect`
//!   * Object path: `/dev/mackes/MDE/Connect`
//!   * Interface:  `dev.mackes.MDE.Connect1` (version-suffixed
//!     per freedesktop best practice so a v2 rev can coexist).

use std::sync::Arc;

use crate::pairing::PairingStore;

/// Bus name MDE acquires on the user session bus.
pub const BUS_NAME: &str = "dev.mackes.MDE.Connect";

/// Object path the Connect interface is hosted at.
pub const OBJECT_PATH: &str = "/dev/mackes/MDE/Connect";

/// Interface name (version-suffixed so a future v2 can
/// register `dev.mackes.MDE.Connect2` alongside).
pub const INTERFACE_NAME: &str = "dev.mackes.MDE.Connect1";

/// D-Bus host errors. Stable Display tokens for audit-log
/// entries.
#[derive(Debug)]
pub enum DbusError {
    /// zbus connection-time error (couldn't reach the session
    /// bus, no DBUS_SESSION_BUS_ADDRESS, etc.).
    Connect(String),
    /// Object registration failed.
    ObjectRegister(String),
    /// Bus-name request failed — either rejected by the bus or
    /// already acquired by another process.
    NameAlreadyAcquired,
}

impl std::fmt::Display for DbusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbusError::Connect(s) => write!(f, "connect: {s}"),
            DbusError::ObjectRegister(s) => write!(f, "object_register: {s}"),
            DbusError::NameAlreadyAcquired => write!(f, "name_already_acquired"),
        }
    }
}

impl std::error::Error for DbusError {}

/// The Connect interface implementation. Backed by the
/// shared `PairingStore`; methods + signals land in KDC2-3.4
/// onwards.
///
/// `zbus`'s derive macro at this scaffold stage exposes a
/// no-method interface so the bus-name acquisition + object
/// registration are testable independently from the eventual
/// method implementations. KDC2-3.4 fills in `ListDevices` +
/// `GetDevice`; 3.5 adds the pair-flow methods, etc.
#[allow(dead_code)] // pairing_store consumed by KDC2-3.4+
pub struct ConnectInterface {
    pairing_store: Arc<PairingStore>,
}

#[zbus::interface(name = "dev.mackes.MDE.Connect1")]
impl ConnectInterface {
    /// Stub method — the host's own version string. Used by
    /// `gdbus introspect` smoke tests + ad-hoc operator probes
    /// (`gdbus call --session --dest dev.mackes.MDE.Connect
    /// --object-path /dev/mackes/MDE/Connect --method
    /// dev.mackes.MDE.Connect1.Version`).
    ///
    /// Methods that mutate state (Pair/Unpair/RingDevice/
    /// SendSms/SendFile) land in KDC2-3.4 onwards.
    async fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

/// Live D-Bus host handle. Holds the zbus Connection so it
/// stays alive for the daemon's lifetime; dropping the handle
/// surrenders the bus name + un-registers the object.
pub struct DbusServer {
    _connection: zbus::Connection,
}

impl DbusServer {
    /// Acquire the Connect bus name + register the
    /// ConnectInterface at `/dev/mackes/MDE/Connect` on the
    /// user session bus.
    ///
    /// Errors:
    ///   * `Connect` — couldn't reach the session bus.
    ///   * `ObjectRegister` — registering the interface failed.
    ///   * `NameAlreadyAcquired` — another mde-kdc is already
    ///     running (or another process owns the name).
    pub async fn start(pairing: Arc<PairingStore>) -> Result<Self, DbusError> {
        let interface = ConnectInterface {
            pairing_store: pairing,
        };
        let connection = zbus::connection::Builder::session()
            .map_err(|e| DbusError::Connect(format!("{e}")))?
            .serve_at(OBJECT_PATH, interface)
            .map_err(|e| DbusError::ObjectRegister(format!("{e}")))?
            .name(BUS_NAME)
            .map_err(|_e| {
                // zbus surfaces name-acquisition failures from
                // both validation + bus-side rejection through
                // the same Result. We classify the rejection
                // case as `NameAlreadyAcquired`; validation
                // errors (invalid bus name) wedge here too but
                // shouldn't fire because BUS_NAME is hard-coded
                // + matches the freedesktop format. (No
                // tracing dep in this crate; the caller can
                // log around this if needed.)
                DbusError::NameAlreadyAcquired
            })?
            .build()
            .await
            .map_err(|e| {
                let msg = format!("{e}");
                if msg.contains("NameInUse") || msg.contains("already") {
                    DbusError::NameAlreadyAcquired
                } else {
                    DbusError::Connect(msg)
                }
            })?;
        Ok(Self {
            _connection: connection,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bus_name_matches_freedesktop_convention() {
        // Reverse-DNS form, no slashes, alphanumeric + dots.
        assert!(BUS_NAME.starts_with("dev.mackes.MDE."));
        assert!(BUS_NAME
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_'));
    }

    #[test]
    fn object_path_matches_bus_name_shape() {
        // Object path mirrors the bus name with `.` → `/` +
        // leading `/`. Allows operators to derive one from the
        // other without consulting docs.
        assert_eq!(OBJECT_PATH, "/dev/mackes/MDE/Connect");
        let derived = format!("/{}", BUS_NAME.replace('.', "/"));
        assert_eq!(derived, OBJECT_PATH);
    }

    #[test]
    fn interface_name_includes_version_suffix() {
        // Interface name carries a numeric version suffix
        // (`1`) so a future v2 rev can coexist via
        // `dev.mackes.MDE.Connect2` without breaking v1
        // clients. Lock the current value.
        assert_eq!(INTERFACE_NAME, "dev.mackes.MDE.Connect1");
        assert!(INTERFACE_NAME.ends_with('1'));
    }

    #[test]
    fn dbus_error_display_uses_stable_tokens() {
        assert_eq!(
            format!("{}", DbusError::NameAlreadyAcquired),
            "name_already_acquired",
        );
        assert!(format!("{}", DbusError::Connect("x".into())).starts_with("connect: "));
        assert!(format!("{}", DbusError::ObjectRegister("y".into()))
            .starts_with("object_register: "));
    }
}
