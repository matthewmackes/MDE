//! KDC2-4.3 — mesh-shunt fan-out.
//!
//! Pure helper the `mackesd::worker` reconcile tick calls on
//! each iteration. Reads every neighbor's `phones.json`
//! (KDC2-4.1) + injects every paired-phone record into the
//! host's `DiscoveryRegistry` (KDC2-2.11) as a
//! `SyntheticAnnounce`.
//!
//! Receivers can't distinguish a phone reachable via mesh-
//! shunt from one broadcast on the local LAN — same trust
//! model (cert fingerprint pinned at first pair) applies.

use std::path::Path;

use mde_kdc_proto::discovery::{Announce, DeviceType, DiscoveryRegistry, SyntheticAnnounce};

use crate::transport::phones_manifest::{self, ManifestError, PhoneRecord, PhonesManifest};

/// One injection-pass outcome. The worker logs this on each
/// tick + the audit chain records a summary so operators can
/// see "peer-A relayed 3 phones, peer-B was unreachable".
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ShuntStats {
    /// Neighbors whose `phones.json` was read successfully.
    pub neighbors_read: u32,
    /// Total phone records injected into the registry across
    /// every neighbor.
    pub phones_injected: u32,
    /// Neighbors whose manifest was absent (peer hasn't paired
    /// any phones yet). NOT a failure — just a no-op for that
    /// peer.
    pub neighbors_absent: u32,
    /// Neighbors whose manifest failed to parse (corrupt
    /// JSON, schema mismatch, I/O). Counted but not surfaced
    /// individually — caller checks the audit log for
    /// specifics.
    pub neighbors_errored: u32,
}

/// Walk each neighbor + read their `phones.json` + inject each
/// phone as a synthetic announce. Pure-fn — takes the registry
/// + paths + neighbor list, mutates the registry, returns
/// stats.
///
/// `now_ms` is the wall-clock timestamp the injection runs at;
/// each synthetic announce records this as `relayed_at_ms`.
/// Receivers' freshness check uses it later.
pub fn inject_neighbor_phones(
    qnm_root: &Path,
    neighbors: &[String],
    registry: &mut DiscoveryRegistry,
    now_ms: i64,
) -> ShuntStats {
    let mut stats = ShuntStats::default();
    for neighbor_id in neighbors {
        match phones_manifest::read_manifest(qnm_root, neighbor_id) {
            Ok(Some(manifest)) => {
                stats.neighbors_read += 1;
                for phone in &manifest.phones {
                    inject_one_phone(neighbor_id, phone, registry, now_ms);
                    stats.phones_injected += 1;
                }
            }
            Ok(None) => {
                // Neighbor hasn't paired any phones yet — no
                // manifest. Normal, not an error.
                stats.neighbors_absent += 1;
            }
            Err(ManifestError::Io(_))
            | Err(ManifestError::Json(_))
            | Err(ManifestError::UnsupportedSchema(_)) => {
                // Caller can grep the audit log for specifics;
                // this stats counter is the aggregate signal.
                stats.neighbors_errored += 1;
            }
        }
    }
    let _ = qnm_root; // kept for future "scan dir for all neighbors" mode
    let _ = neighbors;
    let _ = registry;
    let _ = now_ms;
    stats
}

fn inject_one_phone(
    relayer_id: &str,
    phone: &PhoneRecord,
    registry: &mut DiscoveryRegistry,
    now_ms: i64,
) {
    let announce = Announce {
        device_id: phone.id.clone(),
        device_name: phone.name.clone(),
        device_type: DeviceType::Phone,
        protocol_version: mde_kdc_proto::PROTOCOL_VERSION,
        incoming_capabilities: phone.capabilities.clone(),
        outgoing_capabilities: phone.capabilities.clone(),
    };
    let synthetic = SyntheticAnnounce {
        announce,
        relayed_by: relayer_id.to_string(),
        relayed_at_ms: now_ms,
    };
    registry.inject_synthetic(synthetic);
}

/// Read a single manifest's phones into the registry. Lower-
/// level than [`inject_neighbor_phones`] — used by tests +
/// any caller that already has a parsed manifest in hand.
pub fn inject_from_manifest(
    manifest: &PhonesManifest,
    registry: &mut DiscoveryRegistry,
    now_ms: i64,
) -> u32 {
    let mut count = 0;
    for phone in &manifest.phones {
        inject_one_phone(&manifest.peer_id, phone, registry, now_ms);
        count += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::phones_manifest::PhoneRecord;
    use tempfile::tempdir;

    fn sample_phone(id: &str) -> PhoneRecord {
        PhoneRecord {
            id: id.into(),
            name: id.into(),
            fingerprint: "AB:CD".into(),
            capabilities: vec!["kdeconnect.clipboard".into()],
            last_seen: 1_700_000_000,
        }
    }

    fn write_manifest(qnm_root: &Path, peer_id: &str, phones: Vec<PhoneRecord>) {
        let m = PhonesManifest::new(peer_id, phones);
        phones_manifest::write_manifest(qnm_root, &m).unwrap();
    }

    #[test]
    fn inject_from_empty_manifest_returns_zero() {
        let mut reg = DiscoveryRegistry::new();
        let m = PhonesManifest::new("peer-A", vec![]);
        let n = inject_from_manifest(&m, &mut reg, 1000);
        assert_eq!(n, 0);
        assert!(reg.is_empty());
    }

    #[test]
    fn inject_from_manifest_with_phones_populates_registry() {
        let mut reg = DiscoveryRegistry::new();
        let m = PhonesManifest::new(
            "peer-A",
            vec![sample_phone("phone-1"), sample_phone("phone-2")],
        );
        let n = inject_from_manifest(&m, &mut reg, 1000);
        assert_eq!(n, 2);
        assert_eq!(reg.len(), 2);
        // Both phones are attributed to peer-A as relayer.
        assert_eq!(reg.relayer_for("phone-1"), Some("peer-A"));
        assert_eq!(reg.relayer_for("phone-2"), Some("peer-A"));
    }

    #[test]
    fn inject_neighbor_phones_walks_every_neighbor() {
        let tmp = tempdir().unwrap();
        write_manifest(tmp.path(), "peer-A", vec![sample_phone("phone-A1")]);
        write_manifest(
            tmp.path(),
            "peer-B",
            vec![sample_phone("phone-B1"), sample_phone("phone-B2")],
        );
        let mut reg = DiscoveryRegistry::new();
        let stats = inject_neighbor_phones(
            tmp.path(),
            &["peer-A".to_string(), "peer-B".to_string()],
            &mut reg,
            1000,
        );
        assert_eq!(stats.neighbors_read, 2);
        assert_eq!(stats.phones_injected, 3);
        assert_eq!(stats.neighbors_absent, 0);
        assert_eq!(stats.neighbors_errored, 0);
        assert_eq!(reg.len(), 3);
        assert_eq!(reg.relayer_for("phone-A1"), Some("peer-A"));
        assert_eq!(reg.relayer_for("phone-B2"), Some("peer-B"));
    }

    #[test]
    fn inject_neighbor_phones_counts_absent_manifests() {
        let tmp = tempdir().unwrap();
        // No phones.json written for peer-Z.
        let mut reg = DiscoveryRegistry::new();
        let stats = inject_neighbor_phones(tmp.path(), &["peer-Z".to_string()], &mut reg, 1000);
        assert_eq!(stats.neighbors_read, 0);
        assert_eq!(stats.neighbors_absent, 1);
        assert_eq!(stats.phones_injected, 0);
        assert!(reg.is_empty());
    }

    #[test]
    fn inject_neighbor_phones_counts_corrupt_manifests() {
        let tmp = tempdir().unwrap();
        // Write a corrupt phones.json for peer-X.
        let path = phones_manifest::manifest_path(tmp.path(), "peer-X");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "not [ valid toml or json").unwrap();
        let mut reg = DiscoveryRegistry::new();
        let stats = inject_neighbor_phones(tmp.path(), &["peer-X".to_string()], &mut reg, 1000);
        assert_eq!(stats.neighbors_errored, 1);
        assert!(reg.is_empty());
    }

    #[test]
    fn inject_records_phone_type_as_phone() {
        // KDC's device_type token for a phone is "phone" (the
        // serde rendering of DeviceType::Phone). Receivers
        // gate phone-only UI on this — must match.
        let mut reg = DiscoveryRegistry::new();
        let m = PhonesManifest::new("peer-A", vec![sample_phone("phone-X")]);
        inject_from_manifest(&m, &mut reg, 1000);
        let fresh = reg.take_fresh(1000);
        assert_eq!(fresh.len(), 1);
        assert_eq!(fresh[0].device_type, DeviceType::Phone);
    }

    #[test]
    fn re_injection_via_inject_neighbor_phones_replaces_relayer() {
        // Phone moves between neighbors: starts under peer-A,
        // then peer-B picks it up. The registry must reflect
        // the new relayer.
        let tmp = tempdir().unwrap();
        write_manifest(tmp.path(), "peer-A", vec![sample_phone("phone-X")]);
        write_manifest(tmp.path(), "peer-B", vec![sample_phone("phone-X")]);
        let mut reg = DiscoveryRegistry::new();
        // First pass: peer-A as relayer.
        inject_neighbor_phones(tmp.path(), &["peer-A".to_string()], &mut reg, 1000);
        assert_eq!(reg.relayer_for("phone-X"), Some("peer-A"));
        // Second pass: peer-B picks it up. Same device_id →
        // upsert replaces.
        inject_neighbor_phones(tmp.path(), &["peer-B".to_string()], &mut reg, 2000);
        assert_eq!(reg.relayer_for("phone-X"), Some("peer-B"));
        assert_eq!(reg.len(), 1);
    }
}
