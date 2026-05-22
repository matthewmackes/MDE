//! KDC2-3.2 file-backed pairing store.
//!
//! Wraps `mde_kdc_proto::crypto::RingKeyStore` with persistence at
//! `~/.config/mde/connect/` (per the v2.1 KDC2 lock: fresh
//! identity store, NOT importing from `~/.config/kdeconnect/`).
//!
//! On-disk layout:
//!
//! ```text
//! ~/.config/mde/connect/
//!   ├── identity.pem        # PKCS#8 RSA-2048 private key
//!   └── devices.toml        # paired peers + public keys
//! ```
//!
//! Implementation skeleton lands in KDC2-3.2 — this module is
//! currently a declaration-only placeholder so the crate compiles
//! while the surface is being designed.

#![allow(missing_docs)] // skeleton only

/// Placeholder for the future `PairingStore` struct.
///
/// KDC2-3.2 fills this in with:
///   * `PairingStore::open(config_dir)` — load or initialize.
///   * `PairingStore::install_peer(peer_id, public_key)` — record
///     a freshly-paired peer.
///   * `PairingStore::forget_peer(peer_id)` — un-pair.
///   * Implements `mde_kdc_proto::crypto::KeyStore` so the wire
///     layer can dispatch through it.
#[derive(Debug, Default)]
pub struct PairingStore;
