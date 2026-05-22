//! KDC2-3 host integration crate for the v2.1 native KDE Connect
//! re-implementation.
//!
//! Wraps the pure-library `mde-kdc-proto` with the wiring it
//! needs to be useful in a running MDE session:
//!
//!   * [`pairing`] — on-disk pairing store at
//!     `~/.config/mde/connect/` (devices.toml + identity.pem),
//!     wrapping `mde_kdc_proto::crypto::RingKeyStore` with cross-
//!     restart persistence.
//!   * [`transport`] — `mackes_transport::Transport` impl. Routes
//!     wire packets between the local KDC protocol layer + the
//!     mesh router.
//!   * `dbus` (KDC2-3.4 follow-up) — zbus host exposing
//!     `dev.mackes.MDE.Connect.*`.
//!   * `worker` (KDC2-3.5 follow-up) — `mackesd` `Worker` impl
//!     so the daemon lifecycle owns the KDC subsystem.
//!
//! ## Replacement of the v13.0 facade
//!
//! v2.0.x shipped this crate as a one-line `pub use mackes_kdc::*;`
//! re-export over the Phase 13 Option A wrapper crate. v2.1 KDC2
//! retires that approach — neither `mackes_kdc` nor the upstream
//! `kdeconnectd` daemon participates in v2.1 KDE Connect. The
//! `pub use` is gone; consumers that imported from `mde_kdc::*`
//! get a compile error pointing them at the new module layout
//! per the v2.1 KDC2 supersession (CHANGELOG v2.1.0 calls this
//! out as a Breaking Change).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod keygen;
pub mod pairing;
pub mod transport;

// Re-export the most commonly-used types from the protocol crate
// at the host-crate's top level so consumers don't need to import
// mde_kdc_proto separately for the basics.
pub use mde_kdc_proto::wire::{CapabilitiesHeader, Packet};
pub use mde_kdc_proto::PROTOCOL_VERSION;
