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
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{DigitallySignedStruct, DistinguishedName, SignatureScheme};
use sha2::{Digest, Sha256};

use crate::pairing::{PairedDevice, PairingStore};

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

// ──────────────────────────────────────────────────────────────────
// UC-3 — server-side TLS accept with pinned client-cert fingerprint.
//
// Symmetric to the outbound `connect_pinned_tls` path: where the
// client uses `PinnedFingerprintVerifier` to pin the *server*'s
// cert, the server uses `PinnedClientCertVerifier` to require + pin
// the *client*'s cert against the pairing store. The verifier
// accepts a client cert iff its SHA-256 fingerprint matches an
// upserted `PairedDevice`.
//
// After the handshake, `accept_pinned_tls` re-derives the matched
// device from the connection's peer-cert chain and returns it
// alongside the live TlsStream — the inbound packet-dispatch
// loop (UC-4) uses that device for the `PluginContext.peer_id`
// passed into every plugin's `process()`.
// ──────────────────────────────────────────────────────────────────

/// rustls `ClientCertVerifier` that accepts ONLY a client whose
/// cert fingerprint resolves to a paired device.
///
/// Constructed with an `Arc<PairingStore>` so the verifier and
/// the post-handshake device-resolution path read the same
/// canonical set. The verifier holds an `Arc` rather than a
/// raw reference so it satisfies rustls's `'static` bound.
#[derive(Debug)]
pub struct PinnedClientCertVerifier {
    store: Arc<PairingStore>,
}

impl PinnedClientCertVerifier {
    /// Wrap a pairing store into the verifier.
    #[must_use]
    pub fn new(store: Arc<PairingStore>) -> Self {
        Self { store }
    }
}

impl ClientCertVerifier for PinnedClientCertVerifier {
    fn offer_client_auth(&self) -> bool {
        // KDC's mutual-TLS model requires the client to present
        // its identity cert. Without it we can't pin to a paired
        // device.
        true
    }

    fn client_auth_mandatory(&self) -> bool {
        // Mandatory — a client without a cert can't possibly be
        // paired, so the handshake should fail at the TLS layer
        // (not later at the dispatch layer).
        true
    }

    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        // We don't advertise any CA subjects because KDC's trust
        // model is fingerprint-pinning, not CA-chain validation.
        // Stock KDE Connect clients send their self-signed cert
        // regardless of the server's hint list, so an empty hint
        // is interoperable.
        &[]
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, rustls::Error> {
        let observed = compute_fingerprint(end_entity.as_ref());
        if self.store.find_by_fingerprint(&observed).is_some() {
            Ok(ClientCertVerified::assertion())
        } else {
            Err(rustls::Error::General(format!(
                "kdc-client-untrusted: fingerprint={observed}",
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

/// Errors from the inbound TLS accept path.
#[derive(Debug)]
pub enum AcceptError {
    /// TLS handshake failed at the rustls layer (cert rejected,
    /// protocol violation, etc.). Wraps the I/O error rustls
    /// surfaces through tokio_rustls.
    Tls(std::io::Error),
    /// Handshake completed but the presented cert's fingerprint
    /// doesn't resolve to any paired device. Should be unreachable
    /// in normal operation because `PinnedClientCertVerifier`
    /// rejects unpaired clients during the handshake; surfaces
    /// here only if the store mutates between verify + post-
    /// handshake re-derive.
    Untrusted {
        /// The unrecognized fingerprint, surfaced for audit logging.
        fingerprint: String,
    },
    /// The accepted connection didn't expose a client cert
    /// (shouldn't happen under `client_auth_mandatory()`).
    NoPeerCert,
    /// Building the rustls `ServerConfig` failed (bad cert chain
    /// or unsupported private-key form).
    BadCertChain(String),
}

impl std::fmt::Display for AcceptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AcceptError::Tls(e) => write!(f, "tls: {e}"),
            AcceptError::Untrusted { fingerprint } => {
                write!(f, "untrusted: fingerprint={fingerprint}")
            }
            AcceptError::NoPeerCert => f.write_str("no_peer_cert"),
            AcceptError::BadCertChain(s) => write!(f, "bad_cert_chain: {s}"),
        }
    }
}

impl std::error::Error for AcceptError {}

/// Build the rustls `ServerConfig` used by `accept_pinned_tls`.
///
/// `server_cert_der` is the host's own X.509 cert DER (issued
/// via `keygen::issue_identity_cert`); `server_key_pkcs8_der`
/// is the matching PKCS#8 private key DER. The pairing-store
/// reference flows through into the `PinnedClientCertVerifier`
/// — the verifier holds a clone of the same `Arc`, so a
/// concurrent re-pair on another worker is visible immediately.
pub fn build_server_config(
    server_cert_der: Vec<u8>,
    server_key_pkcs8_der: Vec<u8>,
    store: Arc<PairingStore>,
) -> Result<rustls::ServerConfig, AcceptError> {
    let cert_chain = vec![CertificateDer::from(server_cert_der)];
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(server_key_pkcs8_der));
    let verifier = Arc::new(PinnedClientCertVerifier::new(store));
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| AcceptError::BadCertChain(format!("protocol_versions: {e}")))?
        .with_client_cert_verifier(verifier)
        .with_single_cert(cert_chain, key)
        .map_err(|e| AcceptError::BadCertChain(format!("single_cert: {e}")))
}

/// Accept a TLS handshake on the given TCP socket, requiring +
/// pinning the client cert against the pairing store. On
/// success, returns the live TLS stream alongside the
/// `PairedDevice` the client cert resolved to.
///
/// The cert + key bytes are passed in (rather than loaded from
/// the store internally) so the inbound worker can boot once at
/// startup with cached bytes — avoids re-PEM-parsing
/// `identity.pem` on every accept.
pub async fn accept_pinned_tls(
    tcp: tokio::net::TcpStream,
    server_cert_der: Vec<u8>,
    server_key_pkcs8_der: Vec<u8>,
    store: Arc<PairingStore>,
) -> Result<
    (
        tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
        PairedDevice,
    ),
    AcceptError,
> {
    let config = build_server_config(server_cert_der, server_key_pkcs8_der, Arc::clone(&store))?;
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let tls = acceptor.accept(tcp).await.map_err(AcceptError::Tls)?;
    // Re-derive the matched device from the post-handshake peer-cert
    // chain. The verifier already enforced presence + pinning, but
    // we re-look up to attach the canonical `PairedDevice` record
    // (id, name, capabilities) to the connection.
    let device = {
        let (_, conn_state) = tls.get_ref();
        let peer_certs = conn_state
            .peer_certificates()
            .ok_or(AcceptError::NoPeerCert)?;
        let leaf = peer_certs.first().ok_or(AcceptError::NoPeerCert)?;
        let fp = compute_fingerprint(leaf.as_ref());
        store
            .find_by_fingerprint(&fp)
            .ok_or(AcceptError::Untrusted { fingerprint: fp })?
    };
    Ok((tls, device))
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

    // ─────────────────────────────────────────────────────────
    // UC-3 — accept_pinned_tls server-side loopback tests
    // ─────────────────────────────────────────────────────────

    use crate::keygen;
    use crate::pairing::{PairedDevice, PairingStore};
    use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
    use std::sync::Arc;
    use tempfile::tempdir;

    /// Build a rustls ClientConfig that presents the given client
    /// cert + key + accepts any server cert (FirstPairVerifier).
    /// Used by UC-3 tests to drive the client side of the mutual-
    /// TLS handshake.
    fn make_test_client_config_with_auth(
        client_cert_der: Vec<u8>,
        client_key_pkcs8_der: Vec<u8>,
    ) -> rustls::ClientConfig {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let cert_chain = vec![CertificateDer::from(client_cert_der)];
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(client_key_pkcs8_der));
        let verifier: Arc<dyn ServerCertVerifier> = Arc::new(FirstPairVerifier);
        rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("client protocol versions")
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_client_auth_cert(cert_chain, key)
            .expect("client config with auth cert")
    }

    /// Boots a UC-3 inbound listener on a random loopback port,
    /// pre-pairs the given client cert into the store, accepts
    /// one connection, and returns the (resolved peer-id,
    /// listener-addr, store) tuple via a channel.
    ///
    /// Used by the success-path test below. The listener thread
    /// drops the connection after the resolve so the client's
    /// `connector.connect` future resolves with the handshake
    /// result.
    async fn spawn_uc3_loopback_server(
        server_cert_der: Vec<u8>,
        server_key_pkcs8_der: Vec<u8>,
        store: Arc<PairingStore>,
    ) -> (
        std::net::SocketAddr,
        tokio::sync::oneshot::Receiver<Result<String, String>>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.expect("accept");
            let result = accept_pinned_tls(tcp, server_cert_der, server_key_pkcs8_der, store).await;
            let payload = match result {
                Ok((_stream, device)) => Ok(device.id.clone()),
                Err(e) => Err(format!("{e}")),
            };
            let _ = tx.send(payload);
        });
        (addr, rx)
    }

    #[tokio::test(flavor = "current_thread")]
    async fn accept_pinned_tls_completes_mutual_handshake_for_paired_client() {
        // Server side: generate identity, issue cert.
        let server_pkcs8 = keygen::generate_pkcs8().unwrap();
        let server_cert = keygen::issue_identity_cert(&server_pkcs8, "server-A").unwrap();
        // Client side: generate identity, issue cert.
        let client_pkcs8 = keygen::generate_pkcs8().unwrap();
        let client_cert = keygen::issue_identity_cert(&client_pkcs8, "client-B").unwrap();
        let client_fp = compute_fingerprint(&client_cert);
        // Pre-pair the client into the server's store.
        let tmp = tempdir().unwrap();
        let store = Arc::new(PairingStore::open_or_init(tmp.path()).unwrap());
        store
            .upsert(PairedDevice {
                id: "client-B".into(),
                name: "Pixel".into(),
                kind: "phone".into(),
                fingerprint: client_fp.clone(),
                public_key_b64: "AA==".into(),
                capabilities: vec!["kdeconnect.clipboard".into()],
                paired_at: 1_700_000_000,
                last_seen_at: 1_700_000_500,
            })
            .unwrap();

        let (addr, server_result_rx) =
            spawn_uc3_loopback_server(server_cert, server_pkcs8, Arc::clone(&store)).await;

        // Drive the client side of the handshake.
        let client_config = make_test_client_config_with_auth(client_cert, client_pkcs8);
        let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
        let tcp = tokio::net::TcpStream::connect(addr).await.expect("tcp");
        let server_name = ServerName::try_from("server-A").unwrap();
        let _client_tls = connector
            .connect(server_name, tcp)
            .await
            .expect("client TLS handshake completes");

        // The server task resolves with the device id.
        let resolved = server_result_rx.await.expect("server completed");
        assert_eq!(resolved, Ok("client-B".to_string()));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn accept_pinned_tls_rejects_unpaired_client() {
        let server_pkcs8 = keygen::generate_pkcs8().unwrap();
        let server_cert = keygen::issue_identity_cert(&server_pkcs8, "server-A").unwrap();
        let unpaired_pkcs8 = keygen::generate_pkcs8().unwrap();
        let unpaired_cert = keygen::issue_identity_cert(&unpaired_pkcs8, "unpaired").unwrap();
        // Empty store — no devices paired.
        let tmp = tempdir().unwrap();
        let store = Arc::new(PairingStore::open_or_init(tmp.path()).unwrap());

        let (addr, server_result_rx) =
            spawn_uc3_loopback_server(server_cert, server_pkcs8, Arc::clone(&store)).await;

        // Client attempts to connect. Under TLS 1.3 the server's
        // cert-verifier rejection arrives as a post-handshake alert,
        // so client_result may surface as either an immediate
        // handshake error OR an Ok stream that errors on first read.
        // Force the alert delivery by attempting to read a byte
        // post-connect; either path is acceptable for the test.
        let client_config = make_test_client_config_with_auth(unpaired_cert, unpaired_pkcs8);
        let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
        let tcp = tokio::net::TcpStream::connect(addr).await.expect("tcp");
        let server_name = ServerName::try_from("server-A").unwrap();
        let connect_result = connector.connect(server_name, tcp).await;
        if let Ok(mut tls) = connect_result {
            // Drive a read to force the server's alert into our
            // path. With an unpaired client cert the server has
            // already aborted, so this will error.
            use tokio::io::AsyncReadExt;
            let mut buf = [0u8; 1];
            let read_result = tls.read(&mut buf).await;
            let surfaced = match &read_result {
                Err(_) => true,
                Ok(0) => true,
                Ok(_) => false,
            };
            assert!(
                surfaced,
                "post-handshake read must surface the server's rejection, got {read_result:?}",
            );
        }

        // Server side surfaces an AcceptError::Tls (the cert-verifier
        // rejection is signaled to the client as an alert, which
        // surfaces server-side as an I/O error from the TLS layer).
        let resolved = server_result_rx.await.expect("server completed");
        assert!(
            matches!(&resolved, Err(s) if s.starts_with("tls: ")),
            "server must surface tls error for unpaired client, got {resolved:?}",
        );
    }

    #[test]
    fn pinned_client_cert_verifier_accepts_paired_fingerprint() {
        let tmp = tempdir().unwrap();
        let store = Arc::new(PairingStore::open_or_init(tmp.path()).unwrap());
        // Pre-pair a fake cert: any bytes work for fingerprint-only
        // lookup since the verifier doesn't do chain validation.
        let fake_cert = b"fake client cert der".to_vec();
        let fp = compute_fingerprint(&fake_cert);
        store
            .upsert(PairedDevice {
                id: "alice".into(),
                name: "alice".into(),
                kind: "phone".into(),
                fingerprint: fp.clone(),
                public_key_b64: "AA==".into(),
                capabilities: vec![],
                paired_at: 0,
                last_seen_at: 0,
            })
            .unwrap();
        let verifier = PinnedClientCertVerifier::new(store);
        let r = verifier.verify_client_cert(
            &CertificateDer::from(fake_cert),
            &[],
            dummy_now(),
        );
        assert!(r.is_ok(), "verifier must accept paired cert");
    }

    #[test]
    fn pinned_client_cert_verifier_rejects_unpaired_fingerprint() {
        let tmp = tempdir().unwrap();
        let store = Arc::new(PairingStore::open_or_init(tmp.path()).unwrap());
        let verifier = PinnedClientCertVerifier::new(store);
        let r = verifier.verify_client_cert(
            &CertificateDer::from(b"never seen before".to_vec()),
            &[],
            dummy_now(),
        );
        let err = r.expect_err("unpaired must reject");
        assert!(
            format!("{err}").contains("kdc-client-untrusted"),
            "error must carry kdc-client-untrusted tag",
        );
    }

    #[test]
    fn pinned_client_cert_verifier_demands_client_auth() {
        let tmp = tempdir().unwrap();
        let store = Arc::new(PairingStore::open_or_init(tmp.path()).unwrap());
        let verifier = PinnedClientCertVerifier::new(store);
        assert!(verifier.offer_client_auth());
        assert!(verifier.client_auth_mandatory());
    }

    #[test]
    fn accept_error_display_uses_stable_tokens() {
        assert!(format!(
            "{}",
            AcceptError::Untrusted {
                fingerprint: "AB:CD".into(),
            },
        )
        .starts_with("untrusted: "));
        assert_eq!(format!("{}", AcceptError::NoPeerCert), "no_peer_cert");
        assert!(format!(
            "{}",
            AcceptError::BadCertChain("bad".into()),
        )
        .starts_with("bad_cert_chain: "));
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
