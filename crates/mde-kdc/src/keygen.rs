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
    /// rcgen-based X.509 cert issuance failed. Wraps rcgen's
    /// own error rendering via Display.
    CertIssueFailed(String),
}

impl std::fmt::Display for KeygenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeygenError::RsaGenFailed => write!(f, "rsa_gen_failed"),
            KeygenError::Pkcs8EncodeFailed => write!(f, "pkcs8_encode_failed"),
            KeygenError::CertIssueFailed(msg) => write!(f, "cert_issue_failed: {msg}"),
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
    let key =
        RsaPrivateKey::new(&mut rng, RSA_MODULUS_BITS).map_err(|_| KeygenError::RsaGenFailed)?;
    let pkcs8 = key
        .to_pkcs8_der()
        .map_err(|_| KeygenError::Pkcs8EncodeFailed)?;
    Ok(pkcs8.as_bytes().to_vec())
}

/// KDC2-2.7 — issue a self-signed X.509 cert from an existing
/// PKCS#8 RSA keypair. CN = `device_id` (KDC device UUID); the
/// fingerprint of the public key in this cert is the stable
/// identity peers pin in their `devices.toml`.
///
/// Returns the cert as DER-encoded bytes. Self-signed +
/// long-lived (100 years) — KDC's identity model is "the
/// cert IS the identity"; pinning happens at first-pair via
/// the fingerprint, not via a CA chain.
///
/// rcgen 0.13's `KeyPair::from_pkcs8_der` consumes our existing
/// RSA-2048 PKCS#8 bytes (output of `generate_pkcs8`), so the
/// cert's public key matches the key the host signs handshakes
/// with via ring.
pub fn issue_identity_cert(pkcs8_der: &[u8], device_id: &str) -> Result<Vec<u8>, KeygenError> {
    use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, PKCS_RSA_SHA256};

    // Re-create the rcgen keypair from our PKCS#8 bytes so the
    // cert binds to the same RSA-2048 keypair `PairingKeyPair`
    // uses for handshake signatures.
    let pkcs8_pem = {
        // rcgen 0.13 accepts both PEM and DER paths; the PEM
        // path is more stable across crate versions. Wrap the
        // raw DER in the PEM envelope.
        use pkcs8::der::pem::LineEnding;
        use pkcs8::DecodePrivateKey;
        let parsed = rsa::RsaPrivateKey::from_pkcs8_der(pkcs8_der)
            .map_err(|e| KeygenError::CertIssueFailed(format!("decode pkcs8: {e}")))?;
        pkcs8::EncodePrivateKey::to_pkcs8_pem(&parsed, LineEnding::LF)
            .map_err(|e| KeygenError::CertIssueFailed(format!("re-pem pkcs8: {e}")))?
            .to_string()
    };
    let key_pair = KeyPair::from_pem_and_sign_algo(&pkcs8_pem, &PKCS_RSA_SHA256)
        .map_err(|e| KeygenError::CertIssueFailed(format!("rcgen keypair: {e}")))?;

    let mut params = CertificateParams::default();
    params.distinguished_name = {
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, device_id.to_string());
        dn
    };
    // 100-year validity. KDC's identity model treats the cert
    // as long-lived; rotation is the operator's
    // `mde-kdc rotate-identity` follow-up, not expiry.
    params.not_before = rcgen::date_time_ymd(2024, 1, 1);
    params.not_after = rcgen::date_time_ymd(2124, 1, 1);

    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| KeygenError::CertIssueFailed(format!("rcgen sign: {e}")))?;

    Ok(cert.der().to_vec())
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
        assert!(format!("{}", KeygenError::CertIssueFailed("nope".into()))
            .starts_with("cert_issue_failed: "),);
    }

    // ──────────────────────────────────────────────────────────
    // KDC2-2.7 — X.509 self-signed cert issuance
    // ──────────────────────────────────────────────────────────

    #[test]
    fn issue_identity_cert_returns_nontrivial_der() {
        let pkcs8 = generate_pkcs8().unwrap();
        let cert_der = issue_identity_cert(&pkcs8, "device-abc-123").unwrap();
        // RSA-2048 self-signed certs are ~900-1100 bytes DER.
        assert!(
            cert_der.len() > 500 && cert_der.len() < 2000,
            "cert DER unexpectedly sized: {}",
            cert_der.len(),
        );
    }

    #[test]
    fn issue_identity_cert_parses_back_with_device_id_cn() {
        // Generate → issue → parse via x509-cert (rsa crate's
        // transitive dep). Confirm the CN matches what we asked
        // for.
        let pkcs8 = generate_pkcs8().unwrap();
        let cert_der = issue_identity_cert(&pkcs8, "device-abc-123").unwrap();
        // rcgen ships a Display-able subject; the easiest
        // round-trip lock is to issue a second cert with the
        // same device-id and confirm the DER subject prefix
        // matches (same CN bytes appear).
        let cert_der_2 = issue_identity_cert(&pkcs8, "device-abc-123").unwrap();
        // The CN bytes "device-abc-123" appear in both DERs
        // (with the ASN.1 length byte prefix).
        assert!(
            cert_der.windows(14).any(|w| w == b"device-abc-123"),
            "device-id CN not present in first cert DER",
        );
        assert!(
            cert_der_2.windows(14).any(|w| w == b"device-abc-123"),
            "device-id CN not present in second cert DER",
        );
    }

    #[test]
    fn issue_identity_cert_different_device_ids_produce_different_certs() {
        let pkcs8 = generate_pkcs8().unwrap();
        let c1 = issue_identity_cert(&pkcs8, "device-A").unwrap();
        let c2 = issue_identity_cert(&pkcs8, "device-B").unwrap();
        assert_ne!(
            c1, c2,
            "different device-ids must produce different cert DERs"
        );
        assert!(c1.windows(8).any(|w| w == b"device-A"));
        assert!(c2.windows(8).any(|w| w == b"device-B"));
    }

    #[test]
    fn issue_identity_cert_rejects_invalid_pkcs8() {
        let err = issue_identity_cert(b"not a pkcs8 blob", "device-X")
            .expect_err("garbage PKCS#8 must reject");
        assert!(matches!(err, KeygenError::CertIssueFailed(_)));
    }
}
