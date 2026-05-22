//! KDC2-3.1 RSA-2048 keypair generation.
//!
//! mde-kdc-proto delegates keygen here because ring 0.17.x does
//! not expose stable RSA generation. We use the pure-Rust `rsa`
//! crate just for this one-shot operation; the hot sign / verify
//! path stays on ring (via mde-kdc-proto's `PairingKeyPair`).
//!
//! Output is PKCS#8 DER bytes — the same format
//! `PairingKeyPair::from_pkcs8` accepts.
//!
//! ## When this fires
//!
//! Once per peer-identity lifetime. The mde-kdc pairing store
//! (KDC2-3.2) calls this on first launch when no
//! `~/.config/mde/connect/identity.pem` exists, persists the
//! generated PKCS#8 to disk, and never calls keygen again unless
//! the operator explicitly rotates identity via `mde-kdc rotate`.

use rand::rngs::OsRng;
use rsa::pkcs8::EncodePrivateKey;
use rsa::RsaPrivateKey;

/// RSA modulus size in bits. Matches upstream KDE Connect's
/// 2048-bit identity — lower would break stock-client interop;
/// higher is wasteful for a session-handshake key.
pub const RSA_MODULUS_BITS: usize = 2048;

/// Errors keygen may surface. Stable Display tokens for
/// audit-log entries.
#[derive(Debug)]
pub enum KeygenError {
    /// `rsa::RsaPrivateKey::new` failed. Practically only happens
    /// when the OS RNG is broken — a panic-class condition we
    /// surface as an error rather than `expect()` so callers can
    /// decide whether to panic or retry.
    RsaGenFailed,
    /// PKCS#8 serialization failed. Defensive — would imply the
    /// `rsa` crate produced an unserializable key.
    Pkcs8EncodeFailed,
}

impl std::fmt::Display for KeygenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeygenError::RsaGenFailed => write!(f, "rsa_gen_failed"),
            KeygenError::Pkcs8EncodeFailed => write!(f, "pkcs8_encode_failed"),
        }
    }
}

impl std::error::Error for KeygenError {}

/// Generate a fresh RSA-2048 keypair and return its PKCS#8 DER
/// encoding. Feed the bytes into
/// `mde_kdc_proto::crypto::PairingKeyPair::from_pkcs8` to get a
/// signable handle backed by ring.
pub fn generate_pkcs8() -> Result<Vec<u8>, KeygenError> {
    let mut rng = OsRng;
    let key = RsaPrivateKey::new(&mut rng, RSA_MODULUS_BITS)
        .map_err(|_| KeygenError::RsaGenFailed)?;
    let pkcs8 = key
        .to_pkcs8_der()
        .map_err(|_| KeygenError::Pkcs8EncodeFailed)?;
    Ok(pkcs8.as_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mde_kdc_proto::crypto::{verify_signature, PairingKeyPair};
    use rsa::pkcs8::DecodePrivateKey;

    #[test]
    fn generate_pkcs8_returns_loadable_keypair() {
        // Round-trip: generate → load into ring via mde-kdc-proto's
        // `PairingKeyPair::from_pkcs8` → sign → verify with a
        // public key derived from the same private. This is the
        // bridge between the rsa crate (keygen) and ring (sign /
        // verify) that the KDC2 split depends on.
        let pkcs8 = generate_pkcs8().expect("keygen succeeds");
        let kp = PairingKeyPair::from_pkcs8(&pkcs8).expect("ring accepts our PKCS#8");
        let signature = kp.sign(b"hello").expect("sign succeeds");
        assert!(!signature.is_empty());

        // Extract the public key (PKCS#1 RSAPublicKey DER) for
        // ring's verifier — same path the live host does after
        // exchanging public keys with a peer.
        let private = RsaPrivateKey::from_pkcs8_der(&pkcs8).unwrap();
        let public = private.to_public_key();
        let pub_der = rsa::pkcs1::EncodeRsaPublicKey::to_pkcs1_der(&public)
            .expect("public key to PKCS#1 DER");

        verify_signature(pub_der.as_bytes(), b"hello", &signature)
            .expect("signature verifies against derived public key");
    }

    #[test]
    fn generate_pkcs8_returns_nontrivial_bytes() {
        // 2048-bit RSA PKCS#8 DER is roughly 1190-1218 bytes —
        // sanity-check we didn't return an empty / tiny blob.
        let pkcs8 = generate_pkcs8().unwrap();
        assert!(
            pkcs8.len() > 1000,
            "PKCS#8 DER should be ~1200 bytes; got {}",
            pkcs8.len(),
        );
        assert!(
            pkcs8.len() < 1500,
            "PKCS#8 DER should be ~1200 bytes; got {}",
            pkcs8.len(),
        );
    }

    #[test]
    fn two_consecutive_keygen_calls_produce_different_keys() {
        let k1 = generate_pkcs8().unwrap();
        let k2 = generate_pkcs8().unwrap();
        assert_ne!(k1, k2, "RNG must not repeat across consecutive calls");
    }

    #[test]
    fn keygen_error_display_is_machine_token() {
        assert_eq!(format!("{}", KeygenError::RsaGenFailed), "rsa_gen_failed");
        assert_eq!(
            format!("{}", KeygenError::Pkcs8EncodeFailed),
            "pkcs8_encode_failed",
        );
    }
}
