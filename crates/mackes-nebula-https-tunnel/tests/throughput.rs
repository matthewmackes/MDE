//! NF-1.6 — Throughput floor integration test.
//!
//! Spins up a loopback tunnel listener + dialer pair, pushes
//! 100 MB through it in `MAX_FRAME_BYTES`-sized frames, asserts
//! the wall-clock throughput clears the Q10 floor (≥ 5 Mbps).
//!
//! Gated behind `--features bench` so CI doesn't allocate the
//! 100 MB working set on every PR. Also restricted to Linux
//! targets — the tunnel uses Unix-domain socket demux helpers
//! elsewhere in the crate; throughput numbers from non-Linux
//! kernels would be apples-to-oranges versus the Fedora-on-x86_64
//! production target.

#![cfg(all(feature = "bench", target_os = "linux"))]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::single_match_else,
    clippy::struct_field_names
)]

use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::time::Instant;

use mackes_nebula_https_tunnel::{dial, listen, MAX_FRAME_BYTES};
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
use tempfile::NamedTempFile;

/// Q10 lock — covert TCP/443 path must clear 5 Mbps.
const MIN_THROUGHPUT_MBPS: f64 = 5.0;
/// 100 MB payload — what the worklist NF-1.6 entry calls for.
const PAYLOAD_BYTES: usize = 100 * 1024 * 1024;

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
    let mut cert_file = NamedTempFile::new().expect("cert tempfile");
    cert_file.write_all(cert_pem.as_bytes()).expect("write cert");
    let mut key_file = NamedTempFile::new().expect("key tempfile");
    key_file.write_all(key_pem.as_bytes()).expect("write key");
    let mut ca_file = NamedTempFile::new().expect("ca tempfile");
    ca_file.write_all(cert_pem.as_bytes()).expect("write ca");
    TestCert {
        cert_pem_path: cert_file,
        key_pem_path: key_file,
        ca_pem_path: ca_file,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn covert_tunnel_clears_five_mbps_floor() {
    let tc = issue_test_cert("loopback.bench");
    let bind: SocketAddr = (Ipv4Addr::LOCALHOST, 0).into();
    let listener = listen(bind, tc.cert_pem_path.path(), tc.key_pem_path.path())
        .await
        .expect("listen");
    let bound = listener.local_addr();
    let ca_path = tc.ca_pem_path.path().to_path_buf();

    let server = tokio::spawn(async move {
        let mut s = listener.accept().await.expect("accept");
        let mut received: usize = 0;
        while received < PAYLOAD_BYTES {
            match s.recv_frame().await.expect("recv") {
                Some(frame) => received += frame.len(),
                None => break,
            }
        }
        received
    });

    let mut client = dial_with_retry(bound, "loopback.bench", &ca_path).await;
    let frame_payload = vec![0xAB; MAX_FRAME_BYTES];
    let frames = PAYLOAD_BYTES.div_ceil(MAX_FRAME_BYTES);

    let start = Instant::now();
    for _ in 0..frames {
        client.send_frame(&frame_payload).await.expect("send");
    }
    client.shutdown().await.expect("shutdown");
    let elapsed = start.elapsed();

    let received = server.await.expect("server join");
    let bytes_sent = frames * MAX_FRAME_BYTES;
    assert!(
        received >= PAYLOAD_BYTES,
        "server received {received} < {PAYLOAD_BYTES} expected"
    );

    let mbps = (bytes_sent as f64 * 8.0) / (elapsed.as_secs_f64() * 1_000_000.0);
    eprintln!(
        "NF-1.6 throughput: {mbps:.2} Mbps over {:.2} s (sent {bytes_sent} bytes)",
        elapsed.as_secs_f64()
    );
    assert!(
        mbps >= MIN_THROUGHPUT_MBPS,
        "throughput {mbps:.2} Mbps below Q10 floor {MIN_THROUGHPUT_MBPS} Mbps"
    );
}

async fn dial_with_retry(
    addr: SocketAddr,
    sni: &str,
    ca: &Path,
) -> mackes_nebula_https_tunnel::TunnelStream {
    // Loopback listen is already bound by the time we get here,
    // but on a busy CI the accept side might not be polling yet.
    // Two-attempt loop is enough — production paths have actual
    // retry-with-backoff in the connectivity worker.
    match dial(addr, sni, ca).await {
        Ok(s) => s,
        Err(_) => {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            dial(addr, sni, ca).await.expect("dial retry")
        }
    }
}
