//! NF-1 — TCP/443 covert transport for the v2.5 Nebula fabric.
//!
//! Wraps Nebula UDP frames in a long-lived rustls TLS 1.3
//! stream. ALPN advertises `h2,http/1.1` so a passive observer
//! sees what looks like an HTTP/2 long-poll session against an
//! nginx-style host. Wire format inside the TLS stream is the
//! 4-byte big-endian length-prefixed framing locked in
//! [`framing`]; payload bytes pass through unmodified.
//!
//! ## Components
//!
//! * [`listen`] — server-side TCP/443 listener that completes the
//!   rustls handshake against an operator-supplied cert + key
//!   (Let's Encrypt in production; self-signed for bench /
//!   loopback tests).
//! * [`dial`] — client-side dialer that pins the SNI against the
//!   operator's CA bundle and completes the TLS 1.3 handshake.
//! * [`framing`] — pure-fn 4-byte length-prefixed framing.
//! * [`activation`] — activation state machine ported from
//!   `mackesd::https_fallback`; the connectivity worker reads
//!   [`activation::State::is_active`] to decide when to spray
//!   packets at this transport.
//! * [`accept_demuxed`] — server-side demux helper; accepts one
//!   TLS connection, unwraps each Nebula frame, forwards the raw
//!   bytes to a Unix domain socket where the lighthouse's native
//!   Nebula process is listening.
//!
//! ## Wire-protocol locks (`docs/design/v2.5-nebula-fabric.md` Q4)
//!
//! * **TLS 1.3 only.** `Versions::Tls13` pin; no down-negotiation
//!   to 1.2. A DPI middlebox seeing 1.2 cipher suites would be
//!   the obvious fingerprint to avoid.
//! * **ALPN `h2,http/1.1`.** The same set nginx ships by default.
//!   We advertise both so the negotiated proto matches whatever
//!   the observer expects to see; the inside of the stream
//!   carries our own framing regardless.
//! * **No client cert.** The cover identity is "any HTTPS client" —
//!   asking for one would draw attention.
//! * **Frame size cap 1408 bytes** (Nebula's default MTU). Inner
//!   frames exceeding that are dropped at the framing layer.
//!
//! ## Where this slots into the v2.5 fabric
//!
//! ```text
//!   peer A's nebula                          peer B's nebula
//!   (native UDP socket)                      (native UDP socket)
//!         │                                            ▲
//!         ▼                                            │
//!   [activation::State::Active]                  [accept_demuxed]
//!         │                                            ▲
//!         ▼                                            │
//!     [framing::encode]                          [framing::decode]
//!         │                                            ▲
//!         └──── rustls 1.3 stream over TCP/443 ────────┘
//!                  (ALPN: h2 / http/1.1)
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt;
use std::io;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bytes::BytesMut;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UnixStream};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use tracing::{error, info, warn};

pub mod activation;
pub mod framing;

pub use activation::{
    transition, transition_at, FailureWindow, ProbePairOutcome, State, TransitionInput,
    FAILURE_THRESHOLD, FAILURE_WINDOW,
};
pub use framing::{decode_frame, encode_frame, FrameError, MAX_FRAME_BYTES};

/// ALPN protocols advertised on both sides of the tunnel.
///
/// `h2` first (matches nginx's default `http2 on;` preference)
/// then `http/1.1` for graceful fallback. Mirrors the v2.5 lock
/// "passive observer sees HTTP/2 long-poll shape."
pub const ALPN_PROTOCOLS: &[&[u8]] = &[b"h2", b"http/1.1"];

/// Default TCP port. RFC-2818 — the whole point is to look like
/// real HTTPS, which means 443.
pub const DEFAULT_PORT: u16 = 443;

/// Errors surfaced by the listener / dialer / demux entry points.
///
/// Modeled after `mackesd::https_fallback`'s error shape: a small
/// closed set of `&'static str` codes, `Debug` + `Display` (no
/// anyhow). Code strings double as metric / audit labels.
#[derive(Debug)]
pub enum TunnelError {
    /// Reading the server cert, server key, or CA bundle file
    /// failed (missing, unreadable, permission denied).
    Io {
        /// Stable code for log lines + metric labels.
        code: &'static str,
        /// Underlying I/O error.
        source: io::Error,
    },
    /// The PEM file parsed but contained no usable certs or keys.
    EmptyPem {
        /// Stable code identifying which input was empty.
        code: &'static str,
    },
    /// Configuration error from `rustls` — usually "cert chain
    /// doesn't match private key," "invalid CA cert encoding,"
    /// or "TLS version selection rejected."
    BadConfig {
        /// Stable code identifying which configuration step failed.
        code: &'static str,
        /// Human-readable detail.
        detail: String,
    },
    /// The TLS handshake failed (peer rejected cert, ALPN
    /// mismatch, protocol downgrade attempt, etc.).
    HandshakeFailed {
        /// Stable code for metric labels.
        code: &'static str,
        /// Underlying I/O / rustls error rendered as a string.
        detail: String,
    },
    /// One side received an oversized frame; the connection is
    /// torn down. The wire-protocol lock at `MAX_FRAME_BYTES` is
    /// fail-closed: nonconformant peers don't get a second chance.
    Frame {
        /// Stable code for metric labels.
        code: &'static str,
        /// Underlying framing error.
        source: FrameError,
    },
    /// SNI parsing rejected the supplied hostname (not a valid
    /// DNS label per RFC 6066). Misconfiguration on the operator's
    /// side; bench / test paths can also trip this when they pass
    /// a bare IP literal.
    BadSni {
        /// The string that failed to parse.
        sni: String,
    },
}

impl fmt::Display for TunnelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { code, source } => write!(f, "tunnel io ({code}): {source}"),
            Self::EmptyPem { code } => write!(f, "tunnel empty PEM ({code})"),
            Self::BadConfig { code, detail } => {
                write!(f, "tunnel bad config ({code}): {detail}")
            }
            Self::HandshakeFailed { code, detail } => {
                write!(f, "tunnel handshake failed ({code}): {detail}")
            }
            Self::Frame { code, source } => write!(f, "tunnel framing ({code}): {source}"),
            Self::BadSni { sni } => write!(f, "tunnel bad sni: {sni:?}"),
        }
    }
}

impl std::error::Error for TunnelError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Frame { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl TunnelError {
    /// Stable string code — useful for log lines + audit /
    /// metric labels without exposing the full error variant.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Io { code, .. }
            | Self::EmptyPem { code }
            | Self::BadConfig { code, .. }
            | Self::HandshakeFailed { code, .. }
            | Self::Frame { code, .. } => code,
            Self::BadSni { .. } => "bad_sni",
        }
    }
}

/// Crate-internal result alias.
pub type Result<T> = std::result::Result<T, TunnelError>;

// ---------------------------------------------------------------
// PEM loaders — small wrappers around rustls-pemfile that
// produce structured TunnelError codes the caller can switch on.
// ---------------------------------------------------------------

async fn read_pem_bytes(path: &Path, code: &'static str) -> Result<Vec<u8>> {
    tokio::fs::read(path).await.map_err(|source| TunnelError::Io {
        code,
        source,
    })
}

fn parse_cert_chain(pem: &[u8], code: &'static str) -> Result<Vec<CertificateDer<'static>>> {
    let mut cursor = std::io::Cursor::new(pem);
    let mut chain = Vec::new();
    for cert in rustls_pemfile::certs(&mut cursor) {
        let cert = cert.map_err(|source| TunnelError::Io { code, source })?;
        chain.push(cert);
    }
    if chain.is_empty() {
        return Err(TunnelError::EmptyPem { code });
    }
    Ok(chain)
}

fn parse_private_key(pem: &[u8], code: &'static str) -> Result<PrivateKeyDer<'static>> {
    let mut cursor = std::io::Cursor::new(pem);
    // rustls-pemfile's `private_key` reads the first PKCS#8,
    // SEC1, or RSA private key it finds, in that order. The
    // returned iterator is `Option`-like.
    let key = rustls_pemfile::private_key(&mut cursor)
        .map_err(|source| TunnelError::Io { code, source })?
        .ok_or(TunnelError::EmptyPem { code })?;
    Ok(key)
}

fn parse_root_store(pem: &[u8], code: &'static str) -> Result<RootCertStore> {
    let mut cursor = std::io::Cursor::new(pem);
    let mut roots = RootCertStore::empty();
    let mut added = 0usize;
    for cert in rustls_pemfile::certs(&mut cursor) {
        let cert = cert.map_err(|source| TunnelError::Io { code, source })?;
        // RootCertStore::add() validates DER structure; reject
        // the whole bundle if any entry is malformed (operator
        // wouldn't want to silently trust a partial bundle).
        roots.add(cert).map_err(|e| TunnelError::BadConfig {
            code,
            detail: format!("rustls root add: {e}"),
        })?;
        added += 1;
    }
    if added == 0 {
        return Err(TunnelError::EmptyPem { code });
    }
    Ok(roots)
}

// ---------------------------------------------------------------
// rustls config builders — pin TLS 1.3 only + ALPN h2/http1.1.
// ---------------------------------------------------------------

fn build_server_config(
    chain: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> Result<ServerConfig> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| TunnelError::BadConfig {
            code: "tls13_pin",
            detail: format!("{e}"),
        })?
        .with_no_client_auth();
    let mut config = builder
        .with_single_cert(chain, key)
        .map_err(|e| TunnelError::BadConfig {
            code: "cert_key_mismatch",
            detail: format!("{e}"),
        })?;
    config.alpn_protocols = ALPN_PROTOCOLS.iter().map(|p| p.to_vec()).collect();
    Ok(config)
}

fn build_client_config(roots: RootCertStore) -> Result<ClientConfig> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| TunnelError::BadConfig {
            code: "tls13_pin",
            detail: format!("{e}"),
        })?
        .with_root_certificates(roots)
        .with_no_client_auth();
    let mut config = builder;
    config.alpn_protocols = ALPN_PROTOCOLS.iter().map(|p| p.to_vec()).collect();
    Ok(config)
}

// ---------------------------------------------------------------
// listen / dial entry points
// ---------------------------------------------------------------

/// Server-side handle returned by [`listen`]. Wraps a bound
/// `TcpListener` + the cached `TlsAcceptor` so subsequent
/// `accept()` calls reuse the parsed cert chain.
pub struct TunnelListener {
    tcp: TcpListener,
    acceptor: TlsAcceptor,
    local_addr: SocketAddr,
}

impl fmt::Debug for TunnelListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TunnelListener")
            .field("local_addr", &self.local_addr)
            .finish_non_exhaustive()
    }
}

impl TunnelListener {
    /// Local socket the listener is bound to. Useful for tests
    /// that pass `:0` and need to recover the kernel-assigned port.
    #[must_use]
    pub const fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Accept one connection + complete the TLS handshake.
    /// Returns a [`TunnelStream`] ready for framed I/O.
    ///
    /// # Errors
    ///
    /// * `Io { code: "tcp_accept" }` — `accept(2)` failed (peer
    ///   reset, listener closed, kernel ran out of fds).
    /// * `HandshakeFailed { code: "tls_accept" }` — peer either
    ///   bailed mid-handshake or presented a TLS version below
    ///   the locked 1.3 floor.
    pub async fn accept(&self) -> Result<TunnelStream> {
        let (tcp, peer) = self
            .tcp
            .accept()
            .await
            .map_err(|source| TunnelError::Io {
                code: "tcp_accept",
                source,
            })?;
        let tls = self
            .acceptor
            .accept(tcp)
            .await
            .map_err(|e| TunnelError::HandshakeFailed {
                code: "tls_accept",
                detail: format!("{e}"),
            })?;
        info!(peer = %peer, "nebula-https-tunnel: accepted");
        Ok(TunnelStream::server(tls))
    }
}

/// Client / server-agnostic framed stream over a rustls 1.3 connection.
///
/// Reads pull through the `decode_frame` decoder; writes go
/// through `encode_frame`. The underlying TLS bytes are never
/// exposed to the caller — keeps the framing invariant at the
/// type system.
pub struct TunnelStream {
    inner: TunnelStreamInner,
    rx_buf: BytesMut,
}

enum TunnelStreamInner {
    Server(tokio_rustls::server::TlsStream<TcpStream>),
    Client(tokio_rustls::client::TlsStream<TcpStream>),
}

impl fmt::Debug for TunnelStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match &self.inner {
            TunnelStreamInner::Server(_) => "Server",
            TunnelStreamInner::Client(_) => "Client",
        };
        f.debug_struct("TunnelStream")
            .field("side", &kind)
            .field("rx_buf_len", &self.rx_buf.len())
            .finish()
    }
}

impl TunnelStream {
    fn server(s: tokio_rustls::server::TlsStream<TcpStream>) -> Self {
        Self {
            inner: TunnelStreamInner::Server(s),
            rx_buf: BytesMut::with_capacity(MAX_FRAME_BYTES * 4),
        }
    }

    fn client(s: tokio_rustls::client::TlsStream<TcpStream>) -> Self {
        Self {
            inner: TunnelStreamInner::Client(s),
            rx_buf: BytesMut::with_capacity(MAX_FRAME_BYTES * 4),
        }
    }

    /// Send one Nebula frame over the tunnel. `payload.len()`
    /// must be ≤ [`MAX_FRAME_BYTES`]; oversized payloads return
    /// `BadConfig { code: "oversized_send" }` so the caller can't
    /// silently drop bytes.
    ///
    /// # Errors
    ///
    /// * `BadConfig { code: "oversized_send" }` — payload exceeds
    ///   [`MAX_FRAME_BYTES`]; no bytes hit the wire.
    /// * `Io { code: "tls_write" }` — peer closed or the TLS
    ///   write failed.
    pub async fn send_frame(&mut self, payload: &[u8]) -> Result<()> {
        if payload.len() > MAX_FRAME_BYTES {
            return Err(TunnelError::BadConfig {
                code: "oversized_send",
                detail: format!(
                    "payload {} > MAX_FRAME_BYTES {}",
                    payload.len(),
                    MAX_FRAME_BYTES
                ),
            });
        }
        let mut out = BytesMut::with_capacity(framing::LENGTH_PREFIX_BYTES + payload.len());
        encode_frame(payload, &mut out);
        match &mut self.inner {
            TunnelStreamInner::Server(s) => s
                .write_all(&out)
                .await
                .map_err(|source| TunnelError::Io {
                    code: "tls_write",
                    source,
                })?,
            TunnelStreamInner::Client(s) => s
                .write_all(&out)
                .await
                .map_err(|source| TunnelError::Io {
                    code: "tls_write",
                    source,
                })?,
        }
        Ok(())
    }

    /// Read one Nebula frame from the tunnel. Returns:
    ///
    ///   * `Ok(Some(payload))` — one full frame; payload bytes
    ///     are an owned `Vec<u8>` so the caller doesn't have to
    ///     hold a borrow against the stream.
    ///   * `Ok(None)` — peer closed the stream cleanly with no
    ///     partial frame in flight (EOF on a frame boundary).
    ///
    /// # Errors
    ///
    /// * `Frame { code: "oversized_recv" }` — peer sent a frame
    ///   exceeding [`MAX_FRAME_BYTES`]; the caller should drop
    ///   the stream.
    /// * `Io { code: "tls_read" }` — TCP / TLS read failed.
    /// * `Io { code: "short_read" }` — peer closed mid-frame
    ///   (unexpected EOF with a partial header / body buffered).
    pub async fn recv_frame(&mut self) -> Result<Option<Vec<u8>>> {
        loop {
            if let Some(frame) =
                decode_frame(&mut self.rx_buf).map_err(|source| TunnelError::Frame {
                    code: "oversized_recv",
                    source,
                })?
            {
                return Ok(Some(frame.to_vec()));
            }
            // Need more bytes. Read into a fixed scratch buffer +
            // extend the rx buffer; the size cap on the inner
            // BytesMut bounds memory.
            let mut scratch = [0u8; 4096];
            let n = match &mut self.inner {
                TunnelStreamInner::Server(s) => s.read(&mut scratch).await.map_err(|source| {
                    TunnelError::Io {
                        code: "tls_read",
                        source,
                    }
                })?,
                TunnelStreamInner::Client(s) => s.read(&mut scratch).await.map_err(|source| {
                    TunnelError::Io {
                        code: "tls_read",
                        source,
                    }
                })?,
            };
            if n == 0 {
                // Clean EOF. If the rx buffer is non-empty the
                // peer truncated mid-frame — surface as
                // Io(short_read) so the caller can log.
                if self.rx_buf.is_empty() {
                    return Ok(None);
                }
                return Err(TunnelError::Io {
                    code: "short_read",
                    source: io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "peer closed mid-frame",
                    ),
                });
            }
            self.rx_buf.extend_from_slice(&scratch[..n]);
        }
    }

    /// Close the underlying TLS stream cleanly. The peer sees a
    /// `close_notify` alert + a TCP FIN. Subsequent send/recv
    /// calls return Io errors.
    ///
    /// # Errors
    ///
    /// `Io { code: "tls_shutdown" }` when the close alert / TCP
    /// FIN couldn't be written (peer already gone, etc.).
    pub async fn shutdown(&mut self) -> Result<()> {
        match &mut self.inner {
            TunnelStreamInner::Server(s) => s
                .shutdown()
                .await
                .map_err(|source| TunnelError::Io {
                    code: "tls_shutdown",
                    source,
                }),
            TunnelStreamInner::Client(s) => s
                .shutdown()
                .await
                .map_err(|source| TunnelError::Io {
                    code: "tls_shutdown",
                    source,
                }),
        }
    }
}

/// Bind a TCP listener at `addr` + wrap incoming sockets in a rustls 1.3 TLS server.
///
/// `server_cert` and `server_key` are PEM files — the cert chain
/// and the matching PKCS#8 / SEC1 / RSA private key the rustls
/// handshake will present.
///
/// # Errors
///
/// * `Io { code: "tcp_bind" }` — `addr` was already bound or
///   the process lacks the privilege to bind it (TCP/443 needs
///   `CAP_NET_BIND_SERVICE` or a `socat`-style port forward).
/// * `Io { code: "read_server_cert" / "read_server_key" }` — the
///   PEM file is missing / unreadable.
/// * `EmptyPem` — the file parsed but contained no certs / no
///   private key.
/// * `BadConfig { code: "cert_key_mismatch" }` — rustls rejected
///   the cert / key pair (mismatched, expired self-sig, bad
///   encoding, etc.).
/// * `BadConfig { code: "tls13_pin" }` — the workspace's ring
///   provider doesn't support TLS 1.3 (won't happen on the
///   pinned 0.23 + ring 0.17 versions; surface in case a future
///   provider swap goes wrong).
pub async fn listen(
    addr: SocketAddr,
    server_cert: &Path,
    server_key: &Path,
) -> Result<TunnelListener> {
    let cert_pem = read_pem_bytes(server_cert, "read_server_cert").await?;
    let key_pem = read_pem_bytes(server_key, "read_server_key").await?;
    let chain = parse_cert_chain(&cert_pem, "parse_server_cert")?;
    let key = parse_private_key(&key_pem, "parse_server_key")?;
    let config = build_server_config(chain, key)?;
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let tcp = TcpListener::bind(addr)
        .await
        .map_err(|source| TunnelError::Io {
            code: "tcp_bind",
            source,
        })?;
    let local_addr = tcp.local_addr().map_err(|source| TunnelError::Io {
        code: "tcp_local_addr",
        source,
    })?;
    info!(addr = %local_addr, "nebula-https-tunnel: listening");
    Ok(TunnelListener {
        tcp,
        acceptor,
        local_addr,
    })
}

/// Dial a remote tunnel server at `addr` and complete the rustls 1.3 handshake.
///
/// Presents `sni` for SNI + cert verification, with the rustls
/// root store loaded from the PEM file at `ca_bundle`. Returns
/// after the TLS handshake completes; subsequent framed I/O
/// happens through the returned [`TunnelStream`].
///
/// # Errors
///
/// * `BadSni` — `sni` isn't a valid DNS label (RFC 6066 only
///   allows DNS names, not IP literals).
/// * `Io { code: "read_ca_bundle" }` — CA file missing /
///   unreadable.
/// * `EmptyPem { code: "parse_ca_bundle" }` — file had no CA
///   certs.
/// * `BadConfig` — the parsed CA bundle was malformed or the
///   rustls protocol-version pin failed.
/// * `Io { code: "tcp_connect" }` — TCP/443 unreachable
///   (refused, timeout, ICMP unreachable, etc.).
/// * `HandshakeFailed { code: "tls_connect" }` — rustls rejected
///   the server's cert chain (untrusted root, SNI mismatch,
///   expired cert, ALPN downgrade, etc.).
pub async fn dial(addr: SocketAddr, sni: &str, ca_bundle: &Path) -> Result<TunnelStream> {
    let ca_pem = read_pem_bytes(ca_bundle, "read_ca_bundle").await?;
    let roots = parse_root_store(&ca_pem, "parse_ca_bundle")?;
    let config = build_client_config(roots)?;
    let connector = TlsConnector::from(Arc::new(config));
    let server_name = ServerName::try_from(sni.to_string()).map_err(|_| TunnelError::BadSni {
        sni: sni.to_string(),
    })?;
    let tcp = TcpStream::connect(addr)
        .await
        .map_err(|source| TunnelError::Io {
            code: "tcp_connect",
            source,
        })?;
    let tls = connector
        .connect(server_name, tcp)
        .await
        .map_err(|e| TunnelError::HandshakeFailed {
            code: "tls_connect",
            detail: format!("{e}"),
        })?;
    info!(addr = %addr, sni = %sni, "nebula-https-tunnel: dialed");
    Ok(TunnelStream::client(tls))
}

// ---------------------------------------------------------------
// NF-1.5 — server-side demux
// ---------------------------------------------------------------

/// Server-side demux helper for the v2.5 fabric's TCP/443 lighthouse listener.
///
/// Accepts one connection on `listener`, completes its TLS
/// handshake, then unwraps each Nebula frame off the wire and
/// forwards the **raw frame payload bytes** to a Unix-domain
/// socket at `downstream`. The inner Nebula process listens on
/// that UDS — the frame demux happens before any Nebula crypto
/// layer sees the packet, so the upstream Nebula stack is
/// unmodified.
///
/// ## Contract
///
/// * **One connection per call.** Returns after the connection's
///   read loop ends (clean EOF, oversized frame, or `downstream`
///   close). The listener is consumed; for multi-connection
///   service the lighthouse process owns the listener directly
///   and spawns a per-stream worker after each `accept()` call
///   (this helper is the worker body, not the accept loop).
/// * **Half-duplex forward.** This helper forwards *tunnel → UDS*
///   only. The reverse direction (UDS → tunnel) is the caller's
///   responsibility — the lighthouse process writes Nebula
///   responses back through the same TLS stream by holding the
///   [`TunnelStream`]. NF-1.5 ships forward-only; the supervisor
///   that owns both sides lives in the future lighthouse process.
/// * **Frame-aligned writes.** Each `decode_frame` result is
///   written to the UDS as one `write_all` call. The downstream
///   Nebula process MUST be able to handle the raw frame body
///   without the 4-byte prefix — the prefix is a tunnel-only
///   artifact.
/// * **Oversized frame is fatal.** A frame exceeding
///   [`MAX_FRAME_BYTES`] tears the connection down with
///   `TunnelError::Frame { code: "oversized_recv" }`. The UDS is
///   closed cleanly on the way out.
/// * **No backpressure smoothing.** If the UDS write would
///   block, the tunnel read stalls until the downstream drains.
///   This is by design — applying buffering here would let the
///   tunnel out-pace the inside-Nebula crypto loop and bloat
///   memory.
///
/// # Errors
///
/// * `Io { code: "uds_connect" }` — `downstream` doesn't exist
///   or the process lacks permission to write to it.
/// * `Io { code: "uds_write" }` — UDS peer closed mid-frame.
/// * Any error [`TunnelListener::accept`] or
///   [`TunnelStream::recv_frame`] can raise.
pub async fn accept_demuxed(listener: TunnelListener, downstream: PathBuf) -> Result<()> {
    let mut stream = listener.accept().await?;
    let mut uds = UnixStream::connect(&downstream)
        .await
        .map_err(|source| TunnelError::Io {
            code: "uds_connect",
            source,
        })?;
    info!(
        downstream = %downstream.display(),
        "nebula-https-tunnel: demux forwarding tunnel → uds"
    );
    loop {
        match stream.recv_frame().await {
            Ok(Some(payload)) => {
                if let Err(source) = uds.write_all(&payload).await {
                    warn!(error = %source, "nebula-https-tunnel: uds write failed; closing");
                    return Err(TunnelError::Io {
                        code: "uds_write",
                        source,
                    });
                }
            }
            Ok(None) => {
                info!("nebula-https-tunnel: peer closed cleanly; demux exiting");
                return Ok(());
            }
            Err(e) => {
                error!(code = e.code(), error = %e, "nebula-https-tunnel: demux tearing down");
                return Err(e);
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::struct_field_names
)]
mod tests {
    use super::*;
    use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
    use std::io::Write;
    use std::net::Ipv4Addr;
    use tempfile::NamedTempFile;
    use tokio::net::UnixListener;

    // ---------------------------------------------------------
    // Test helpers — issue a self-signed cert + write it as
    // PEM files we can pass to listen() / dial().
    // ---------------------------------------------------------

    struct TestCert {
        cert_pem_path: NamedTempFile,
        key_pem_path: NamedTempFile,
        ca_pem_path: NamedTempFile,
    }

    fn issue_test_cert(host: &str) -> TestCert {
        let key_pair = KeyPair::generate().expect("rcgen keypair");
        let mut params = CertificateParams::default();
        params.distinguished_name = {
            let mut dn = DistinguishedName::new();
            dn.push(DnType::CommonName, host);
            dn
        };
        params.subject_alt_names = vec![SanType::DnsName(
            rcgen::Ia5String::try_from(host.to_string()).expect("valid dns"),
        )];
        let cert = params.self_signed(&key_pair).expect("self-sign");
        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        let mut cert_file = NamedTempFile::new().expect("tempfile cert");
        cert_file
            .write_all(cert_pem.as_bytes())
            .expect("write cert");
        let mut key_file = NamedTempFile::new().expect("tempfile key");
        key_file.write_all(key_pem.as_bytes()).expect("write key");
        let mut ca_file = NamedTempFile::new().expect("tempfile ca");
        ca_file.write_all(cert_pem.as_bytes()).expect("write ca");
        TestCert {
            cert_pem_path: cert_file,
            key_pem_path: key_file,
            ca_pem_path: ca_file,
        }
    }

    // ---------------------------------------------------------
    // TunnelError surface
    // ---------------------------------------------------------

    #[test]
    fn tunnel_error_code_round_trips() {
        let e = TunnelError::EmptyPem {
            code: "parse_server_cert",
        };
        assert_eq!(e.code(), "parse_server_cert");

        let e = TunnelError::BadSni {
            sni: "127.0.0.1".into(),
        };
        assert_eq!(e.code(), "bad_sni");
    }

    #[test]
    fn locked_alpn_protocols_match_nginx_default() {
        assert_eq!(ALPN_PROTOCOLS, &[b"h2".as_slice(), b"http/1.1".as_slice()]);
    }

    #[test]
    fn pem_loaders_round_trip_self_signed_cert() {
        let tc = issue_test_cert("loopback.test");
        let cert_pem =
            std::fs::read(tc.cert_pem_path.path()).expect("read cert pem");
        let key_pem = std::fs::read(tc.key_pem_path.path()).expect("read key pem");
        let chain = parse_cert_chain(&cert_pem, "parse_server_cert").expect("chain");
        assert_eq!(chain.len(), 1);
        let _ = parse_private_key(&key_pem, "parse_server_key").expect("key");
    }

    #[test]
    fn parse_cert_chain_empty_pem_surfaces_empty() {
        let err = parse_cert_chain(b"", "parse_server_cert").unwrap_err();
        assert!(matches!(err, TunnelError::EmptyPem { code: "parse_server_cert" }));
    }

    #[test]
    fn parse_private_key_empty_pem_surfaces_empty() {
        let err = parse_private_key(b"", "parse_server_key").unwrap_err();
        assert!(matches!(err, TunnelError::EmptyPem { code: "parse_server_key" }));
    }

    #[test]
    fn parse_root_store_empty_pem_surfaces_empty() {
        let err = parse_root_store(b"", "parse_ca_bundle").unwrap_err();
        assert!(matches!(err, TunnelError::EmptyPem { code: "parse_ca_bundle" }));
    }

    // ---------------------------------------------------------
    // listen() + dial() + send/recv round trip
    // ---------------------------------------------------------

    #[tokio::test(flavor = "current_thread")]
    async fn listen_then_dial_round_trip_a_frame() {
        let tc = issue_test_cert("loopback.test");
        let bind: SocketAddr = (Ipv4Addr::LOCALHOST, 0).into();
        let listener = listen(bind, tc.cert_pem_path.path(), tc.key_pem_path.path())
            .await
            .expect("listen");
        let bound = listener.local_addr();

        let ca_path = tc.ca_pem_path.path().to_path_buf();
        let server = tokio::spawn(async move {
            let mut s = listener.accept().await.expect("accept");
            let frame = s.recv_frame().await.expect("recv").expect("frame");
            assert_eq!(&frame, b"hello from client");
            s.send_frame(b"hi back").await.expect("send");
            s.shutdown().await.expect("shutdown");
        });

        let mut client = dial(bound, "loopback.test", &ca_path).await.expect("dial");
        client.send_frame(b"hello from client").await.expect("send");
        let reply = client.recv_frame().await.expect("recv").expect("frame");
        assert_eq!(&reply, b"hi back");
        client.shutdown().await.expect("shutdown");
        server.await.expect("server join");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn dial_with_bad_sni_returns_bad_sni() {
        let tc = issue_test_cert("loopback.test");
        let bind: SocketAddr = (Ipv4Addr::LOCALHOST, 0).into();
        let listener = listen(bind, tc.cert_pem_path.path(), tc.key_pem_path.path())
            .await
            .expect("listen");
        let bound = listener.local_addr();
        // " " is not a valid DNS label per RFC 6066.
        let err = dial(bound, " ", tc.ca_pem_path.path()).await.unwrap_err();
        assert_eq!(err.code(), "bad_sni");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn dial_with_untrusted_ca_fails_handshake() {
        // Server cert signed by tc.ca, but the dialer is handed
        // a different (unrelated) CA bundle — handshake must
        // fail with HandshakeFailed.
        let server_tc = issue_test_cert("loopback.test");
        let unrelated_tc = issue_test_cert("other.test");
        let bind: SocketAddr = (Ipv4Addr::LOCALHOST, 0).into();
        let listener = listen(
            bind,
            server_tc.cert_pem_path.path(),
            server_tc.key_pem_path.path(),
        )
        .await
        .expect("listen");
        let bound = listener.local_addr();
        // Accept in the background so the connect doesn't hang
        // before rustls negotiates.
        let _server = tokio::spawn(async move {
            let _ = listener.accept().await;
        });
        let err = dial(bound, "loopback.test", unrelated_tc.ca_pem_path.path())
            .await
            .unwrap_err();
        match err {
            TunnelError::HandshakeFailed { code, .. } => {
                assert_eq!(code, "tls_connect");
            }
            other => panic!("expected HandshakeFailed, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn listen_with_missing_cert_returns_io_error() {
        let tc = issue_test_cert("loopback.test");
        let missing = Path::new("/nonexistent/path/to/cert.pem");
        let err = listen(
            (Ipv4Addr::LOCALHOST, 0).into(),
            missing,
            tc.key_pem_path.path(),
        )
        .await
        .unwrap_err();
        match err {
            TunnelError::Io { code, .. } => assert_eq!(code, "read_server_cert"),
            other => panic!("expected Io(read_server_cert), got {other:?}"),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn send_frame_oversized_payload_rejected_locally() {
        // Build a stream by pairing listener + dialer just so we
        // have a real TunnelStream; the test only exercises the
        // send-side size check before any bytes hit the wire.
        let tc = issue_test_cert("loopback.test");
        let bind: SocketAddr = (Ipv4Addr::LOCALHOST, 0).into();
        let listener = listen(bind, tc.cert_pem_path.path(), tc.key_pem_path.path())
            .await
            .expect("listen");
        let bound = listener.local_addr();
        let _server = tokio::spawn(async move {
            let _ = listener.accept().await;
        });
        let mut client = dial(bound, "loopback.test", tc.ca_pem_path.path())
            .await
            .expect("dial");
        let over = vec![0u8; MAX_FRAME_BYTES + 1];
        let err = client.send_frame(&over).await.unwrap_err();
        match err {
            TunnelError::BadConfig { code, .. } => assert_eq!(code, "oversized_send"),
            other => panic!("expected BadConfig(oversized_send), got {other:?}"),
        }
    }

    // ---------------------------------------------------------
    // NF-1.5 — accept_demuxed forwards to UDS
    // ---------------------------------------------------------

    #[tokio::test(flavor = "current_thread")]
    async fn accept_demuxed_forwards_frames_to_uds() {
        let tc = issue_test_cert("loopback.test");
        let bind: SocketAddr = (Ipv4Addr::LOCALHOST, 0).into();
        let listener = listen(bind, tc.cert_pem_path.path(), tc.key_pem_path.path())
            .await
            .expect("listen");
        let bound = listener.local_addr();

        // Allocate a UDS path in tempdir and bind a listener.
        let uds_dir = tempfile::tempdir().expect("tempdir");
        let uds_path = uds_dir.path().join("nebula.sock");
        let uds_listener = UnixListener::bind(&uds_path).expect("uds bind");

        // Server side — accept on UDS + read whatever the demux
        // forwarder writes.
        let uds_path_clone = uds_path.clone();
        let demux = tokio::spawn(async move {
            accept_demuxed(listener, uds_path_clone).await
        });

        let uds_collector = tokio::spawn(async move {
            let (mut s, _) = uds_listener.accept().await.expect("uds accept");
            let mut buf = Vec::new();
            // Read until EOF — collects every frame the demux
            // forwarded across the whole client session.
            let _ = s.read_to_end(&mut buf).await;
            buf
        });

        let mut client = dial(bound, "loopback.test", tc.ca_pem_path.path())
            .await
            .expect("dial");
        client.send_frame(b"frame-a").await.expect("send a");
        client.send_frame(b"frame-b").await.expect("send b");
        client.send_frame(b"frame-c").await.expect("send c");
        client.shutdown().await.expect("shutdown");

        // demux exits on clean EOF.
        demux.await.expect("join demux").expect("demux ok");
        let collected = uds_collector.await.expect("uds join");
        assert_eq!(collected, b"frame-aframe-bframe-c");
    }
}
