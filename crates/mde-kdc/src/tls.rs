//! KDC2-2.8 — TLS layer with fingerprint pinning.
//!
//! KDE Connect's identity model bypasses the conventional CA
//! chain: peers self-sign + the recipient pins the cert
//! fingerprint at first pair. Any later connection that
//! presents a different fingerprint is rejected, surfacing as
//! `PairingState::KeyMismatch` in the UI.
//!
//! Implementation:
//!
//!   * `compute_fingerprint(cert_der)` — SHA-256 of the cert
//!     DER, hex-uppercase with `:` between bytes
//!     (`AB:CD:EF:...`). Matches upstream KDC's UI / settings
//!     dialog format.
//!   * `PinnedFingerprintVerifier` — implements rustls'
//!     `ServerCertVerifier`. Accepts ANY presented cert whose
//!     fingerprint matches the pinned value; rejects every-
//!     thing else. Bypasses the standard chain validation
//!     since self-signed by design.
//!   * `unpinned_verifier()` — used during first-pair (before
//!     the recipient knows what to pin). Accepts every cert
//!     (no CA chain check). Pair flow records the cert
//!     fingerprint into `devices.toml` so subsequent
//!     connections use the pinned verifier.
//!
//! `tokio-rustls`-backed `TlsStream` wrapping lands in
//! KDC2-3.2.a (real network in `KdcHost::open`); this module
//! ships the verifier + config builders + fingerprint helper.

use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use sha2::{Digest, Sha256};

/// Compute the KDC-style cert fingerprint: SHA-256 of the DER
/// bytes, formatted as upper-case hex with `:` between every
/// byte. Matches the format upstream KDE Connect's settings
/// dialog renders.
///
/// Pure deterministic — given the same DER input, returns the
/// same string. Used both at pair-time (to record the
/// fingerprint in `devices.toml`) and at handshake-time
/// (to compare against the pinned value via
/// `PinnedFingerprintVerifier`).
#[must_use]
pub fn compute_fingerprint(cert_der: &[u8]) -> String {
    let digest = Sha256::digest(cert_der);
    let mut out = String::with_capacity(95); // 32 bytes × 3 chars - 1 separator
    for (i, b) in digest.iter().enumerate() {
        if i > 0 {
            out.push(':');
        }
        out.push_str(&format!("{b:02X}"));
    }
    out
}

/// rustls `ServerCertVerifier` that accepts ONLY the cert whose
/// SHA-256 fingerprint matches the pinned value.
///
/// Constructed by the host integration with the value from
/// `PairedDevice.fingerprint` (KDC2-3.7).
#[derive(Debug)]
pub struct PinnedFingerprintVerifier {
    pinned: String,
}

impl PinnedFingerprintVerifier {
    /// Wrap a known fingerprint into the verifier.
    #[must_use]
    pub fn new(pinned_fingerprint: impl Into<String>) -> Self {
        Self {
            pinned: pinned_fingerprint.into(),
        }
    }
}

impl ServerCertVerifier for PinnedFingerprintVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let observed = compute_fingerprint(end_entity.as_ref());
        if observed == self.pinned {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General(format!(
                "kdc-fingerprint-mismatch: expected={} observed={}",
                self.pinned, observed,
            )))
        }
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        // Mirror upstream KDC's allowed schemes — RSA-PKCS1
        // with SHA-256/384/512 covers the self-signed RSA-2048
        // identity certs.
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
        ]
    }
}

/// First-pair verifier — accepts ANY presented cert without
/// checking pin or CA chain. The pair-flow records the cert's
/// fingerprint into `devices.toml`; subsequent connections use
/// [`PinnedFingerprintVerifier`].
///
/// **Do not** use this verifier outside the first-pair path.
/// Anywhere else, fingerprint pinning is what makes KDC's TLS
/// trust model meaningful.
#[derive(Debug)]
pub struct FirstPairVerifier;

impl ServerCertVerifier for FirstPairVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // Pair-flow records the fingerprint AFTER handshake;
        // any cert is acceptable at this stage.
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
        ]
    }
}

/// Build a rustls `ClientConfig` configured for KDC's pinning
/// model. The ring crypto provider is wired explicitly so the
/// audit closure agrees with mde-kdc-proto's ring usage.
///
/// `pinned_fingerprint = None` → uses [`FirstPairVerifier`]
/// (first-pair path). `Some` → uses [`PinnedFingerprintVerifier`].
///
/// KDC2-3.2.a: this builder is reused by
/// [`connect_pinned_tls`] for the live network connect path.
#[must_use]
pub fn build_client_config(pinned_fingerprint: Option<String>) -> rustls::ClientConfig {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("rustls default protocol versions installed");
    let verifier: Arc<dyn ServerCertVerifier> = if let Some(pin) = pinned_fingerprint {
        Arc::new(PinnedFingerprintVerifier::new(pin))
    } else {
        Arc::new(FirstPairVerifier)
    };
    builder
        .dangerous() // KDC self-signed model
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth()
}

// ──────────────────────────────────────────────────────────────────
// KDC2-3.2.a — Real TLS-wrapped TCP connect.
//
// `KdcHost::open(peer_id)` previously returned a stub Connection;
// this connector closes the loop by actually opening a
// `tokio::net::TcpStream` to the peer's address and wrapping it
// with `tokio_rustls::TlsConnector` + the pinned-fingerprint
// verifier built above.
//
// The peer-address resolution (peer_id → SocketAddr) lives a
// layer up — the DiscoveryRegistry caches the source address of
// every received UDP announce. This helper takes an explicit
// `SocketAddr` so it stays testable without booting the full
// discovery layer.
// ──────────────────────────────────────────────────────────────────

/// Errors from the live TLS connect path.
#[derive(Debug)]
pub enum ConnectError {
    /// TCP `connect` failed (host unreachable, no route, refused).
    Tcp(std::io::Error),
    /// TLS handshake failed (peer cert mismatch, bad cert, etc.).
    Tls(std::io::Error),
    /// Peer-id couldn't be parsed as a `ServerName`.
    BadPeerName(String),
}

impl std::fmt::Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectError::Tcp(e) => write!(f, "tcp: {e}"),
            ConnectError::Tls(e) => write!(f, "tls: {e}"),
            ConnectError::BadPeerName(s) => write!(f, "bad_peer_name: {s}"),
        }
    }
}

impl std::error::Error for ConnectError {}

/// Open a TLS-wrapped TCP connection to `addr`, presenting
/// `server_name` in the ClientHello, with the cert pinned to
/// `pinned_fingerprint` (None = first-pair / accept any).
///
/// Returns a `tokio_rustls::client::TlsStream<TcpStream>` that
/// callers wrap with the codec framer + payload-channel
/// handshake.
pub async fn connect_pinned_tls(
    addr: std::net::SocketAddr,
    server_name: &str,
    pinned_fingerprint: Option<String>,
) -> Result<tokio_rustls::client::TlsStream<tokio::net::TcpStream>, ConnectError> {
    let server_name_owned = ServerName::try_from(server_name.to_string())
        .map_err(|e| ConnectError::BadPeerName(format!("{e}")))?;
    let tcp = tokio::net::TcpStream::connect(addr)
        .await
        .map_err(ConnectError::Tcp)?;
    let config = Arc::new(build_client_config(pinned_fingerprint));
    let connector = tokio_rustls::TlsConnector::from(config);
    connector
        .connect(server_name_owned, tcp)
        .await
        .map_err(ConnectError::Tls)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_now() -> UnixTime {
        UnixTime::since_unix_epoch(std::time::Duration::from_secs(1_700_000_000))
    }

    #[test]
    fn fingerprint_is_deterministic() {
        let bytes = b"identical cert bytes";
        assert_eq!(compute_fingerprint(bytes), compute_fingerprint(bytes));
    }

    #[test]
    fn fingerprint_changes_on_input_change() {
        assert_ne!(compute_fingerprint(b"a"), compute_fingerprint(b"b"));
    }

    #[test]
    fn fingerprint_format_matches_upstream_kdc() {
        // Upper-case hex, colon-separated, 32 bytes → 95 chars
        // (32 × 2 hex chars + 31 colons).
        let fp = compute_fingerprint(b"abc");
        assert_eq!(fp.len(), 95);
        // Every third char from index 2 is a colon.
        assert_eq!(&fp[2..3], ":");
        assert_eq!(&fp[5..6], ":");
        // Upper-case hex.
        for c in fp.chars() {
            assert!(
                c.is_ascii_hexdigit() && c.to_ascii_uppercase() == c || c == ':',
                "non-uppercase non-colon char {c:?} in fingerprint",
            );
        }
    }

    #[test]
    fn pinned_verifier_accepts_matching_fingerprint() {
        let cert_bytes = b"some cert der";
        let fp = compute_fingerprint(cert_bytes);
        let verifier = PinnedFingerprintVerifier::new(fp);
        let result = verifier.verify_server_cert(
            &CertificateDer::from(cert_bytes.to_vec()),
            &[],
            &ServerName::try_from("device").unwrap(),
            &[],
            dummy_now(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn pinned_verifier_rejects_mismatched_fingerprint() {
        let verifier = PinnedFingerprintVerifier::new("00:11:22:33");
        let result = verifier.verify_server_cert(
            &CertificateDer::from(b"some cert der".to_vec()),
            &[],
            &ServerName::try_from("device").unwrap(),
            &[],
            dummy_now(),
        );
        let err = result.expect_err("mismatch must reject");
        let msg = format!("{err}");
        assert!(
            msg.contains("kdc-fingerprint-mismatch"),
            "error must include the kdc-fingerprint-mismatch tag: {msg}",
        );
    }

    #[test]
    fn first_pair_verifier_accepts_any_cert() {
        let verifier = FirstPairVerifier;
        let result = verifier.verify_server_cert(
            &CertificateDer::from(b"random bytes".to_vec()),
            &[],
            &ServerName::try_from("device").unwrap(),
            &[],
            dummy_now(),
        );
        assert!(result.is_ok(), "first-pair must accept any cert");
    }

    #[test]
    fn build_client_config_constructs_with_pinning() {
        // Builds a ClientConfig without panicking. The
        // verifier is internalized; we can't readily introspect
        // which path got chosen — but the test confirms the
        // builder doesn't fail to install the ring provider +
        // custom verifier.
        let _cfg = build_client_config(Some("AA:BB:CC".to_string()));
    }

    #[test]
    fn build_client_config_constructs_first_pair() {
        let _cfg = build_client_config(None);
    }

    #[test]
    fn fingerprint_against_real_kdc_cert_round_trip() {
        // Integration with KDC2-2.7's issue_identity_cert: a
        // freshly-issued cert has a stable fingerprint that
        // can be matched.
        let pkcs8 = crate::keygen::generate_pkcs8().unwrap();
        let cert = crate::keygen::issue_identity_cert(&pkcs8, "device-A").unwrap();
        let fp = compute_fingerprint(&cert);
        // Two computations on the same DER bytes must agree.
        assert_eq!(fp, compute_fingerprint(&cert));
        // Pinned verifier accepts this exact cert.
        let v = PinnedFingerprintVerifier::new(fp);
        let r = v.verify_server_cert(
            &CertificateDer::from(cert.clone()),
            &[],
            &ServerName::try_from("device-A").unwrap(),
            &[],
            dummy_now(),
        );
        assert!(r.is_ok());
    }

    // ─────────────────────────────────────────────────────────
    // KDC2-3.2.a — connect_pinned_tls error-path tests
    // ─────────────────────────────────────────────────────────

    #[tokio::test(flavor = "current_thread")]
    async fn connect_pinned_tls_returns_bad_peer_name_for_invalid_name() {
        // An empty string isn't a valid DNS name or IP literal —
        // ServerName::try_from rejects it. We surface BadPeerName
        // instead of letting it leak through as a panic.
        let r = connect_pinned_tls("127.0.0.1:0".parse().unwrap(), "", None).await;
        assert!(matches!(r, Err(ConnectError::BadPeerName(_))));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn connect_pinned_tls_returns_tcp_error_for_unreachable_addr() {
        // Bind a TCP listener so we get a real port, then drop
        // it so connect refuses. Avoids relying on a port the
        // host might actually use.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let r = connect_pinned_tls(addr, "device-X", None).await;
        match r {
            Err(ConnectError::Tcp(_)) => { /* expected */ }
            other => panic!("expected Tcp error, got {other:?}"),
        }
    }

    #[test]
    fn connect_error_display_uses_stable_tokens() {
        assert!(format!(
            "{}",
            ConnectError::Tcp(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "x"
            ),)
        )
        .starts_with("tcp: "));
        assert!(format!("{}", ConnectError::BadPeerName("x".into())).starts_with("bad_peer_name: "));
    }
}
