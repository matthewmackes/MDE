//! KDC2-3.3 `mackes_transport::Transport` impl for the KDC wire.
//!
//! Bridges the protocol crate (`mde-kdc-proto`) into the router
//! trait (`mackes-transport`). Concrete `Transport` impl lands in
//! KDC2-3.3 — this module is currently a declaration-only
//! placeholder so the crate compiles while the surface is being
//! designed.

#![allow(missing_docs)] // skeleton only

/// Placeholder for the future `KdcTransport` struct.
///
/// KDC2-3.3 fills this in with:
///   * Concrete impl of `mackes_transport::Transport` for the
///     KDC wire, kind = `TransportKind::KdcTls`.
///   * Holds a reference to the pairing store + an outbound
///     framing buffer per active peer.
///   * `probe()` cheap reachability via a `ping` packet round-
///     trip with a 200ms budget.
///   * `open()` returns a `Connection` boxed handle that the
///     router keeps alive across sends.
#[derive(Debug, Default)]
pub struct KdcTransport;
