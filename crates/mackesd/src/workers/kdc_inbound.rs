//! UC-5 — KDC inbound TLS accept worker.
//!
//! Wraps `mde_kdc::transport::KdcHost::serve()` in the standard
//! `Worker` lifecycle. Owns the cached server cert + private key
//! bytes (issued once at construction from the pairing store's
//! identity) so per-accept work doesn't repeat the PEM→DER decode.
//!
//! Drives the lifecycle-event channel: every event from
//! `serve()` lands here and is translated into a
//! `tracing::{info,debug,warn}` line. The mesh-router and audit
//! channels are downstream consumers that will subscribe via a
//! broadcast fan-out in a follow-up — for now, logs are the
//! observable surface.

#![cfg(feature = "async-services")]

use std::sync::Arc;

use mde_kdc::dispatch::PluginAuthority;
use mde_kdc::transport::{KdcHost, ServeConfig, ServeEvent, KDC_TLS_PORT};
use mde_kdc_proto::plugins::Registry;
use tokio::sync::Mutex as AsyncMutex;
use tracing::{debug, info, warn};

use super::{ShutdownToken, Worker};

/// UC-5 worker: drives `KdcHost::serve()` for the daemon's
/// lifetime. Restart-on-failure so a transient bind error (port
/// reuse during a fast restart) gets a retry; the supervisor's
/// fixed 250 ms back-off keeps a hard failure from hot-looping.
pub struct KdcInboundWorker {
    host: Arc<KdcHost>,
    server_cert_der: Vec<u8>,
    server_key_pkcs8_der: Vec<u8>,
    registry: Arc<AsyncMutex<Registry>>,
    authority: Arc<dyn PluginAuthority + Send + Sync>,
    /// Bind address. Defaults to `0.0.0.0:1716` (the KDC stock
    /// port); tests pass `127.0.0.1:0` to grab a random loopback.
    bind_addr: std::net::SocketAddr,
}

impl KdcInboundWorker {
    /// Construct with the canonical production bind
    /// (`0.0.0.0:1716`).
    #[must_use]
    pub fn new(
        host: Arc<KdcHost>,
        server_cert_der: Vec<u8>,
        server_key_pkcs8_der: Vec<u8>,
        registry: Arc<AsyncMutex<Registry>>,
        authority: Arc<dyn PluginAuthority + Send + Sync>,
    ) -> Self {
        Self {
            host,
            server_cert_der,
            server_key_pkcs8_der,
            registry,
            authority,
            bind_addr: std::net::SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
                KDC_TLS_PORT,
            ),
        }
    }

    /// Construct with a custom bind address — for tests.
    #[must_use]
    pub fn with_bind(mut self, bind_addr: std::net::SocketAddr) -> Self {
        self.bind_addr = bind_addr;
        self
    }
}

#[async_trait::async_trait]
impl Worker for KdcInboundWorker {
    fn name(&self) -> &'static str {
        "kdc-inbound"
    }

    async fn run(&mut self, mut shutdown: ShutdownToken) -> anyhow::Result<()> {
        let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
        let cfg = ServeConfig {
            bind_addr: self.bind_addr,
            server_cert_der: self.server_cert_der.clone(),
            server_key_pkcs8_der: self.server_key_pkcs8_der.clone(),
            registry: Arc::clone(&self.registry),
            authority: Arc::clone(&self.authority),
        };
        let host = Arc::clone(&self.host);
        // Spawn the serve loop in its own task so the worker's
        // run() can drive both shutdown + event drain.
        let serve_handle = tokio::spawn(async move { host.serve(cfg, event_tx).await });

        loop {
            tokio::select! {
                biased;
                _ = shutdown.wait() => {
                    info!("kdc-inbound: shutdown requested");
                    // The serve task spins on listener.accept(); we
                    // can't gracefully cancel it from here, so abort.
                    // The OS reclaims the listener fd on drop, so a
                    // restart immediately after will re-bind.
                    serve_handle.abort();
                    return Ok(());
                }
                ev = event_rx.recv() => {
                    match ev {
                        Some(ev) => translate_event(ev),
                        None => {
                            // serve task ended (likely a bind error
                            // surfaced via the JoinHandle). Surface
                            // for the supervisor to restart.
                            match serve_handle.await {
                                Ok(Ok(())) => {
                                    info!("kdc-inbound: serve returned clean; exiting");
                                    return Ok(());
                                }
                                Ok(Err(e)) => {
                                    warn!(error = %e, "kdc-inbound: serve errored");
                                    return Err(anyhow::anyhow!("kdc-inbound serve: {e}"));
                                }
                                Err(e) if e.is_cancelled() => {
                                    return Ok(());
                                }
                                Err(e) => {
                                    warn!(error = %e, "kdc-inbound: serve task panicked");
                                    return Err(anyhow::anyhow!("kdc-inbound panic"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn translate_event(ev: ServeEvent) {
    match ev {
        ServeEvent::Listening { addr } => {
            info!(addr = %addr, "kdc-inbound: listening");
        }
        ServeEvent::PeerConnected { peer_id } => {
            info!(peer_id = %peer_id, "kdc-inbound: peer connected");
        }
        ServeEvent::PeerDisconnected { peer_id } => {
            debug!(peer_id = %peer_id, "kdc-inbound: peer disconnected");
        }
        ServeEvent::AcceptRejected { remote, reason } => {
            warn!(
                remote = ?remote,
                reason = %reason,
                "kdc-inbound: accept rejected",
            );
        }
        ServeEvent::PacketDispatched {
            peer_id,
            kind,
            allowed,
        } => {
            if allowed {
                debug!(
                    peer_id = %peer_id,
                    kind = %kind,
                    "kdc-inbound: packet dispatched",
                );
            } else {
                warn!(
                    peer_id = %peer_id,
                    kind = %kind,
                    "kdc-inbound: packet denied by policy",
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mde_kdc::keygen;
    use mde_kdc::pairing::PairingStore;
    use tempfile::tempdir;

    #[derive(Debug)]
    struct AllowAll;
    impl PluginAuthority for AllowAll {
        fn plugin_allowed(&self, _name: &str) -> bool {
            true
        }
    }

    fn make_worker(bind_addr: std::net::SocketAddr) -> KdcInboundWorker {
        let tmp = tempdir().unwrap();
        let store = Arc::new(PairingStore::open_or_init(tmp.path()).unwrap());
        std::mem::forget(tmp);
        let pkcs8 = store.identity().pkcs8_bytes().to_vec();
        let host_id = store.host_id();
        let cert = keygen::issue_identity_cert(&pkcs8, &host_id).unwrap();
        let host = Arc::new(KdcHost::pairing_only(Arc::clone(&store)));
        let registry = Arc::new(AsyncMutex::new(Registry::new()));
        let authority: Arc<dyn PluginAuthority + Send + Sync> = Arc::new(AllowAll);
        KdcInboundWorker::new(host, cert, pkcs8, registry, authority).with_bind(bind_addr)
    }

    #[test]
    fn worker_name_matches_module() {
        let w = make_worker("127.0.0.1:0".parse().unwrap());
        assert_eq!(w.name(), "kdc-inbound");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn worker_exits_clean_on_shutdown() {
        let mut w = make_worker("127.0.0.1:0".parse().unwrap());
        let (tx, rx) = tokio::sync::watch::channel(false);
        let token = ShutdownToken::from_receiver(rx);
        let handle = tokio::spawn(async move { w.run(token).await });
        // Let the bind + Listening event fire.
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        let _ = tx.send(true);
        let result = tokio::time::timeout(std::time::Duration::from_secs(3), handle)
            .await
            .expect("worker joins within timeout")
            .expect("join");
        assert!(result.is_ok(), "worker must exit Ok on shutdown");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn worker_surfaces_bind_error_for_unavailable_port() {
        // Bind a TCP listener first to occupy a port, then try to
        // start the worker on the same port — the inbound worker
        // should surface the bind error via Err(...) so the
        // supervisor's restart policy can act.
        let occupier = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = occupier.local_addr().unwrap();
        let mut w = make_worker(addr);
        let (_tx, rx) = tokio::sync::watch::channel(false);
        let token = ShutdownToken::from_receiver(rx);
        let result = tokio::time::timeout(std::time::Duration::from_secs(3), w.run(token))
            .await
            .expect("worker returns within timeout");
        assert!(result.is_err(), "bind to occupied port must surface error");
        drop(occupier);
    }
}
