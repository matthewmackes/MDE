//! Nebula CA subsystem (NF-2.x, v2.5 fabric rebuild).
//!
//! This module implements the seven NF-2.* tasks of the v2.5 Nebula
//! fabric design (`docs/design/v2.5-nebula-fabric.md`, Q3 PKI lock):
//!
//! - **mint** — generate a fresh CA via `nebula-cert ca` and seal
//!   the keypair to `/var/lib/mackesd/nebula-ca/` (NF-2.2).
//! - **sign** — verify a peer's Ed25519 enrollment signature,
//!   allocate an overlay IP, and sign the peer's host cert
//!   (NF-2.3).
//! - **seal** — atomic write + permission-strict read helpers for
//!   the sealed CA on-disk artifacts (NF-2.4).
//! - **epoch** — leader-failover-driven CA rotation that bumps the
//!   epoch counter and re-signs every active peer cert (NF-2.5).
//! - **bundle** — atomic writer for the per-peer
//!   `nebula-bundle.json` payload the joining peer's NF-3
//!   supervisor reads (NF-2.7).
//!
//! The CLI surface (NF-2.6) lives in `bin/mackesd.rs::Cmd::Ca`.
//!
//! No async code lives in this module. The reconcile loop's
//! supervisor (NF-3) reads the sealed artifacts + SQL rows
//! independently; that crate is the runtime consumer.

pub mod bundle;
pub mod epoch;
pub mod error;
pub mod ipv4_net;
pub mod mint;
pub mod seal;
pub mod sign;

pub use bundle::{bundle_path, read_bundle, write_bundle, NebulaBundle};
pub use epoch::{bump_epoch, current_epoch, EpochBump};
pub use error::{CaError, CaResult};
pub use ipv4_net::Ipv4Network;
pub use mint::{mint_ca, nebula_cert_bin, CaArtifacts};
pub use seal::{ca_dir, seal, unseal};
pub use sign::{
    default_mesh_cidr, list_active_peer_certs, sealed_ca_cert_path, sealed_ca_key_path,
    sign_peer_cert, verify_enrollment_signature, SignedCert,
};
