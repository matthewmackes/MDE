//! NF-3.6 (v2.5) — `dev.mackes.MDE.Nebula` D-Bus surface.
//!
//! Exposes three methods on the session bus that the panel +
//! the wizard call instead of shelling out to `mackesd ca …`:
//!
//!   * `Enroll(passcode: s, name: s) -> s` — wraps
//!     `mackesd_core::enrollment::build_request`. Returns the
//!     JSON-encoded request that the leader's pending-inbox
//!     ingests.
//!   * `Status() -> s` — JSON object with shape
//!     `{state, mesh_id, active_epoch, lighthouse_count,
//!     peer_count, active_transport}`. The panel's mesh-status
//!     applet (NF-10.1) consumes this without spawning a CLI.
//!   * `RegenCerts() -> s` — triggers an NF-2.5 CA-epoch
//!     rotation. Returns a JSON outcome
//!     `{rotated, new_epoch, resigned_count}`.
//!
//! Polkit gating tracked separately as NF-3.6.a — the
//! existing `dev.mackes.mded.admin` action ID referenced in
//! the worklist doesn't yet ship a `.policy` file. Until
//! NF-3.6.a lands, the surface is open on the session bus
//! (same as every other `ipc::*` surface today). The bus
//! itself is per-user, so reach is bounded by the operator's
//! local desktop session.

#![cfg(feature = "async-services")]

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;
use zbus::interface;

/// Stable D-Bus interface name.
pub const NEBULA_INTERFACE: &str = "dev.mackes.MDE.Nebula";

/// Object path the service registers at.
pub const NEBULA_OBJECT_PATH: &str = "/dev/mackes/MDE/Nebula";

/// Well-known bus name. Same root as the existing
/// `dev.mackes.MDE.Fleet.Files` service so the panel's
/// existing connection can `serve_at` both.
pub const NEBULA_BUS_NAME: &str = "dev.mackes.MDE.Nebula";

/// Service state passed into the registered object.
#[derive(Clone)]
pub struct NebulaService {
    store: Arc<Mutex<rusqlite::Connection>>,
    node_id: String,
    mesh_id: String,
    /// Bundle path used to derive lighthouse_count without a
    /// second DB hop. Defaults to
    /// `~/QNM-Shared/<self>/mackesd/nebula-bundle.json`.
    bundle_path: PathBuf,
    /// Peer-cert directory passed through to bump_epoch.
    peer_cert_dir: PathBuf,
    /// Role-host marker path — read to derive `state`.
    role_marker_path: PathBuf,
}

impl NebulaService {
    /// Construct a service handle with the daemon-default
    /// paths (`/var/lib/mackesd/nebula/role.host`,
    /// `/etc/nebula/peers/`).
    #[must_use]
    pub fn new(
        store: Arc<Mutex<rusqlite::Connection>>,
        node_id: String,
        mesh_id: String,
        bundle_path: PathBuf,
    ) -> Self {
        Self {
            store,
            node_id,
            mesh_id,
            bundle_path,
            peer_cert_dir: PathBuf::from("/etc/nebula/peers"),
            role_marker_path: PathBuf::from(
                crate::workers::nebula_supervisor::DEFAULT_ROLE_HOST_MARKER,
            ),
        }
    }

    /// Override the peer-cert dir (tests).
    #[must_use]
    pub fn with_peer_cert_dir(mut self, dir: PathBuf) -> Self {
        self.peer_cert_dir = dir;
        self
    }

    /// Override the role marker path (tests).
    #[must_use]
    pub fn with_role_marker(mut self, path: PathBuf) -> Self {
        self.role_marker_path = path;
        self
    }
}

#[interface(name = "dev.mackes.MDE.Nebula")]
impl NebulaService {
    /// Build an enrollment request signed under a fresh
    /// Ed25519 identity. The returned string is the JSON shape
    /// `mackesd_core::enrollment::EnrollmentRequest`
    /// serializes to; the leader's pending-inbox (NF-7.x
    /// wizard) drops it in.
    ///
    /// Errors:
    ///   * `Failed("passcode failed validation")` when the
    ///     passcode isn't 16 URL-safe characters.
    ///   * `Failed("serialize: …")` if serde-json refuses the
    ///     request (effectively never — surfaced as the same
    ///     human-readable error for symmetry with the CLI).
    async fn enroll(&self, passcode: &str, name: &str) -> zbus::fdo::Result<String> {
        let identity = crate::enrollment::build_identity();
        let display = if name.is_empty() {
            self.node_id
                .strip_prefix("peer:")
                .unwrap_or(self.node_id.as_str())
                .to_string()
        } else {
            name.to_string()
        };
        let req = crate::enrollment::build_request(&identity, passcode, &display)
            .ok_or_else(|| {
                zbus::fdo::Error::Failed(
                    "passcode failed validation (must be 16 URL-safe characters)".into(),
                )
            })?;
        serde_json::to_string(&req)
            .map_err(|e| zbus::fdo::Error::Failed(format!("serialize: {e}")))
    }

    /// Return a JSON-encoded snapshot of the local Nebula
    /// state. Shape:
    ///
    /// ```json
    /// {
    ///   "state":             "host" | "peer" | "uninitialised",
    ///   "mesh_id":           "mesh-anvil",
    ///   "active_epoch":      0,
    ///   "lighthouse_count":  2,
    ///   "peer_count":        5,
    ///   "active_transport":  "udp"
    /// }
    /// ```
    ///
    /// `state = "uninitialised"` when no `nebula_ca` row
    /// exists for `mesh_id` — the panel renders the "first
    /// boot wizard required" empty state.
    async fn status(&self) -> zbus::fdo::Result<String> {
        let conn = self.store.lock().await;
        let active = crate::ca::mint::current_ca(&conn, &self.mesh_id)
            .map_err(|e| zbus::fdo::Error::Failed(format!("ca read: {e}")))?;
        let (active_epoch, state) = match active {
            Some((epoch, _pem)) => {
                let role = if self.role_marker_path.exists() {
                    "host"
                } else {
                    "peer"
                };
                (Some(epoch), role)
            }
            None => (None, "uninitialised"),
        };
        let peer_count = active_epoch
            .map(|e| count_active_peer_certs(&conn, e).unwrap_or(0))
            .unwrap_or(0);
        let lighthouse_count = read_lighthouse_count(&self.bundle_path);
        let active_transport = active_transport_kind(&self.bundle_path);
        let payload = serde_json::json!({
            "state":            state,
            "mesh_id":          self.mesh_id,
            "active_epoch":     active_epoch,
            "lighthouse_count": lighthouse_count,
            "peer_count":       peer_count,
            "active_transport": active_transport,
        });
        Ok(payload.to_string())
    }

    /// Trigger an NF-2.5 CA-epoch rotation. Returns the
    /// outcome as JSON:
    ///
    /// ```json
    /// { "rotated": true,  "new_epoch": 3, "resigned_count": 4 }
    /// { "rotated": false, "reason": "no active CA" }
    /// ```
    ///
    /// On `nebula-cert` failures the call propagates the
    /// underlying error string back to the caller — the panel
    /// surfaces it in the "Mesh ops" toast.
    async fn regen_certs(&self) -> zbus::fdo::Result<String> {
        let mut conn = self.store.lock().await;
        let outcome = crate::ca::epoch::bump_epoch(
            &crate::ca::SubprocessBackend,
            &mut conn,
            &self.mesh_id,
            None,
            None,
            &self.peer_cert_dir,
            crate::ca::epoch::DEFAULT_FALLBACK_LIFETIME_DAYS,
        )
        .map_err(|e| zbus::fdo::Error::Failed(format!("bump_epoch: {e}")))?;
        let payload = match outcome {
            crate::ca::epoch::BumpOutcome::Rotated {
                new_epoch,
                resigned_count,
                ..
            } => serde_json::json!({
                "rotated":        true,
                "new_epoch":      new_epoch,
                "resigned_count": resigned_count,
            }),
            crate::ca::epoch::BumpOutcome::NoActiveCa => serde_json::json!({
                "rotated": false,
                "reason":  "no active CA",
            }),
        };
        Ok(payload.to_string())
    }
}

/// Count active (non-revoked) peer cert rows at the given epoch.
fn count_active_peer_certs(conn: &rusqlite::Connection, epoch: i64) -> Option<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM nebula_peer_certs \
         WHERE epoch = ?1 AND revoked_at IS NULL",
        [epoch],
        |r| r.get(0),
    )
    .ok()
}

/// Read the lighthouse count from the bundle file. Returns 0
/// when the bundle is missing or malformed — the panel
/// renders that as "no lighthouses yet" rather than an error.
fn read_lighthouse_count(bundle_path: &std::path::Path) -> i64 {
    crate::ca::bundle::read_bundle(bundle_path)
        .map(|b| b.lighthouses.len() as i64)
        .unwrap_or(0)
}

/// Derive the active transport from the bundle's lighthouse
/// roster. Returns `"udp"` when the bundle exists (Nebula's
/// default), `"tcp443"` if every lighthouse advertises a
/// `:443` external_addr (covert-path-only deployment), and
/// `"none"` when no bundle exists.
fn active_transport_kind(bundle_path: &std::path::Path) -> &'static str {
    match crate::ca::bundle::read_bundle(bundle_path) {
        Ok(b) => {
            if !b.lighthouses.is_empty()
                && b.lighthouses
                    .iter()
                    .all(|lh| lh.external_addr.ends_with(":443"))
            {
                "tcp443"
            } else {
                "udp"
            }
        }
        Err(_) => "none",
    }
}

/// Register the NebulaService at the canonical bus name +
/// object path on the session bus. The returned connection
/// must stay alive for the daemon's lifetime (drop = surface
/// goes away). Mirrors `ipc::files::register_fleet_files`.
///
/// # Errors
///
/// Whatever zbus reports.
pub async fn register_nebula(state: NebulaService) -> zbus::Result<zbus::Connection> {
    zbus::connection::Builder::session()?
        .name(NEBULA_BUS_NAME)?
        .serve_at(NEBULA_OBJECT_PATH, state)?
        .build()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_store() -> Arc<Mutex<rusqlite::Connection>> {
        let conn = crate::store::open_in_memory().expect("open in-mem");
        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn interface_lock() {
        assert_eq!(NEBULA_INTERFACE, "dev.mackes.MDE.Nebula");
        assert_eq!(NEBULA_OBJECT_PATH, "/dev/mackes/MDE/Nebula");
        assert_eq!(NEBULA_BUS_NAME, "dev.mackes.MDE.Nebula");
    }

    #[tokio::test]
    async fn status_reports_uninitialised_when_no_ca() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = NebulaService::new(
            fresh_store(),
            "peer:test".into(),
            "m1".into(),
            tmp.path().join("nebula-bundle.json"),
        )
        .with_role_marker(tmp.path().join("role.host"));
        let body = svc.status().await.expect("status");
        assert!(body.contains("\"state\":\"uninitialised\""));
        assert!(body.contains("\"active_epoch\":null"));
        assert!(body.contains("\"peer_count\":0"));
    }

    #[tokio::test]
    async fn status_reports_host_when_marker_present_and_ca_exists() {
        use crate::ca::{mint, MockBackend};
        let tmp = tempfile::tempdir().unwrap();
        let store = fresh_store();
        {
            let conn = store.lock().await;
            mint::mint_ca(
                &MockBackend,
                &conn,
                "m1",
                Some(&tmp.path().join("ca.crt")),
                Some(&tmp.path().join("ca.key")),
            )
            .expect("mint");
        }
        let marker = tmp.path().join("role.host");
        std::fs::write(&marker, "role:host\n").unwrap();
        let svc = NebulaService::new(
            store,
            "peer:test".into(),
            "m1".into(),
            tmp.path().join("nebula-bundle.json"),
        )
        .with_role_marker(marker);
        let body = svc.status().await.expect("status");
        assert!(body.contains("\"state\":\"host\""));
        assert!(body.contains("\"active_epoch\":0"));
    }

    #[tokio::test]
    async fn status_reports_peer_when_marker_absent() {
        use crate::ca::{mint, MockBackend};
        let tmp = tempfile::tempdir().unwrap();
        let store = fresh_store();
        {
            let conn = store.lock().await;
            mint::mint_ca(
                &MockBackend,
                &conn,
                "m1",
                Some(&tmp.path().join("ca.crt")),
                Some(&tmp.path().join("ca.key")),
            )
            .expect("mint");
        }
        let svc = NebulaService::new(
            store,
            "peer:test".into(),
            "m1".into(),
            tmp.path().join("nebula-bundle.json"),
        )
        .with_role_marker(tmp.path().join("role.host")); // missing
        let body = svc.status().await.expect("status");
        assert!(body.contains("\"state\":\"peer\""));
    }

    #[tokio::test]
    async fn enroll_rejects_invalid_passcode() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = NebulaService::new(
            fresh_store(),
            "peer:test".into(),
            "m1".into(),
            tmp.path().join("nebula-bundle.json"),
        );
        let err = svc.enroll("too-short", "anvil").await.unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("passcode failed validation"),
            "expected validation error, got: {msg}"
        );
    }

    #[tokio::test]
    async fn enroll_emits_json_for_valid_passcode() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = NebulaService::new(
            fresh_store(),
            "peer:test".into(),
            "m1".into(),
            tmp.path().join("nebula-bundle.json"),
        );
        let passcode = crate::passcode::generate();
        let body = svc.enroll(&passcode, "anvil").await.expect("enroll");
        // Must round-trip as JSON.
        let _: serde_json::Value = serde_json::from_str(&body).expect("json");
        assert!(body.contains("anvil"));
    }

    #[tokio::test]
    async fn enroll_falls_back_to_node_id_when_name_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = NebulaService::new(
            fresh_store(),
            "peer:fallback-host".into(),
            "m1".into(),
            tmp.path().join("nebula-bundle.json"),
        );
        let passcode = crate::passcode::generate();
        let body = svc.enroll(&passcode, "").await.expect("enroll");
        // The fallback strips the `peer:` prefix.
        assert!(body.contains("fallback-host"));
        assert!(!body.contains("peer:fallback-host"));
    }

    #[tokio::test]
    async fn regen_certs_reports_no_active_ca() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = NebulaService::new(
            fresh_store(),
            "peer:test".into(),
            "m1".into(),
            tmp.path().join("nebula-bundle.json"),
        )
        .with_peer_cert_dir(tmp.path().join("peers"));
        let body = svc.regen_certs().await.expect("regen");
        assert!(body.contains("\"rotated\":false"));
        assert!(body.contains("no active CA"));
    }

    #[test]
    fn read_lighthouse_count_returns_zero_for_missing_bundle() {
        let tmp = tempfile::tempdir().unwrap();
        let n = read_lighthouse_count(&tmp.path().join("missing.json"));
        assert_eq!(n, 0);
    }

    #[test]
    fn active_transport_returns_none_when_bundle_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let kind = active_transport_kind(&tmp.path().join("missing.json"));
        assert_eq!(kind, "none");
    }

    #[test]
    fn active_transport_returns_tcp443_when_every_lh_uses_443() {
        use crate::ca::bundle::{write_bundle, LighthouseEntry, NebulaBundle};
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nebula-bundle.json");
        let b = NebulaBundle {
            mesh_id: "m1".into(),
            epoch: 0,
            ca_cert_pem: "ca".into(),
            peer_cert_pem: "p".into(),
            peer_key_pem: "k".into(),
            overlay_ip: "10.42.0.5".into(),
            mesh_cidr: "10.42.0.0/16".into(),
            lighthouses: vec![
                LighthouseEntry {
                    node_id: "peer:lh1".into(),
                    overlay_ip: "10.42.0.1".into(),
                    external_addr: "lh1.example.com:443".into(),
                },
                LighthouseEntry {
                    node_id: "peer:lh2".into(),
                    overlay_ip: "10.42.0.2".into(),
                    external_addr: "lh2.example.com:443".into(),
                },
            ],
            created_at: 1,
        };
        write_bundle(&path, &b).unwrap();
        assert_eq!(active_transport_kind(&path), "tcp443");
    }

    #[test]
    fn active_transport_returns_udp_when_any_lh_uses_4242() {
        use crate::ca::bundle::{write_bundle, LighthouseEntry, NebulaBundle};
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nebula-bundle.json");
        let b = NebulaBundle {
            mesh_id: "m1".into(),
            epoch: 0,
            ca_cert_pem: "ca".into(),
            peer_cert_pem: "p".into(),
            peer_key_pem: "k".into(),
            overlay_ip: "10.42.0.5".into(),
            mesh_cidr: "10.42.0.0/16".into(),
            lighthouses: vec![LighthouseEntry {
                node_id: "peer:lh1".into(),
                overlay_ip: "10.42.0.1".into(),
                external_addr: "lh1.example.com:4242".into(),
            }],
            created_at: 1,
        };
        write_bundle(&path, &b).unwrap();
        assert_eq!(active_transport_kind(&path), "udp");
    }

    #[tokio::test]
    async fn status_includes_lighthouse_count_from_bundle() {
        use crate::ca::bundle::{write_bundle, LighthouseEntry, NebulaBundle};
        use crate::ca::{mint, MockBackend};
        let tmp = tempfile::tempdir().unwrap();
        let bundle_path = tmp.path().join("nebula-bundle.json");
        write_bundle(
            &bundle_path,
            &NebulaBundle {
                mesh_id: "m1".into(),
                epoch: 0,
                ca_cert_pem: "ca".into(),
                peer_cert_pem: "p".into(),
                peer_key_pem: "k".into(),
                overlay_ip: "10.42.0.5".into(),
                mesh_cidr: "10.42.0.0/16".into(),
                lighthouses: vec![
                    LighthouseEntry {
                        node_id: "peer:lh1".into(),
                        overlay_ip: "10.42.0.1".into(),
                        external_addr: "lh1.example.com:4242".into(),
                    },
                    LighthouseEntry {
                        node_id: "peer:lh2".into(),
                        overlay_ip: "10.42.0.2".into(),
                        external_addr: "lh2.example.com:4242".into(),
                    },
                ],
                created_at: 1,
            },
        )
        .unwrap();
        let store = fresh_store();
        {
            let conn = store.lock().await;
            mint::mint_ca(
                &MockBackend,
                &conn,
                "m1",
                Some(&tmp.path().join("ca.crt")),
                Some(&tmp.path().join("ca.key")),
            )
            .expect("mint");
        }
        let svc = NebulaService::new(store, "peer:test".into(), "m1".into(), bundle_path);
        let body = svc.status().await.expect("status");
        assert!(body.contains("\"lighthouse_count\":2"));
        assert!(body.contains("\"active_transport\":\"udp\""));
    }
}
