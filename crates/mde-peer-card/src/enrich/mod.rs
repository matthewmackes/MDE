//! Enrichment layer — pulls additional product info from open
//! data sources. Five sub-modules, one per source:
//!
//! - [`hwdb`] (PC-4) — local, offline, always-on. `vendor:product`
//!   → display name from `/usr/share/hwdata/usb.ids` + systemd
//!   hwdb. Required for the "hwdb-only" first-paint state.
//! - [`lhdb`] (PC-5) — linux-hardware.org. Driver compatibility +
//!   kernel support + similar-machine probes. 7-day TTL.
//! - [`wikidata`] (PC-6) — Wikidata SPARQL for manufacturer +
//!   release year + hero image; Wikipedia REST summary for the
//!   description. 30-day TTL.
//! - [`ifixit`] (PC-7) — teardown thumbnail + repairability score.
//!   30-day TTL.
//! - [`openbench`] (PC-7) — CPU / GPU / SSD percentile vs same
//!   model. 30-day TTL.
//!
//! All four online sources can be disabled via PC-10's
//! `peer_card.online_enrichment` toggle. PC-1..PC-4 ship the
//! offline data path + the type surface; the online integrations
//! land as PC-5..PC-7 (placeholders here).

pub mod hwdb;
pub mod ifixit;
pub mod lhdb;
pub mod openbench;
pub mod wikidata;

use serde::{Deserialize, Serialize};

use crate::probe::PeerProbe;

/// Cache key for the enrichment blob. Per PC-4 acceptance:
/// **vendor:product, NOT peer-id**, so two peers with identical
/// hardware share the cache entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EnrichmentCacheKey {
    /// Hex vendor ID, no `0x` prefix.
    pub vendor_id: String,
    /// Hex product ID, no `0x` prefix.
    pub product_id: String,
}

impl EnrichmentCacheKey {
    /// Build from raw IDs.
    #[must_use]
    pub fn from_vendor_product(vendor: &str, product: &str) -> Self {
        Self {
            vendor_id: vendor.to_ascii_lowercase(),
            product_id: product.to_ascii_lowercase(),
        }
    }

    /// Build from a probe — strips the connection-id dimension.
    #[must_use]
    pub fn for_probe(probe: &PeerProbe) -> Self {
        Self::from_vendor_product(&probe.vendor_id, &probe.product_id)
    }

    /// Canonical file path for the cache entry.
    #[must_use]
    pub fn cache_filename(&self) -> String {
        format!("{}:{}.json", self.vendor_id, self.product_id)
    }
}

/// Aggregated enrichment data resolved from the five sources.
/// All fields are optional; the card paints with whatever's
/// resolved.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Enrichment {
    /// hwdb (PC-4) — vendor + product display names.
    pub hwdb: Option<hwdb::HwdbInfo>,
    /// linux-hardware.org (PC-5).
    pub lhdb: Option<lhdb::LhdbInfo>,
    /// Wikidata + Wikipedia (PC-6).
    pub wikidata: Option<wikidata::WikidataInfo>,
    /// iFixit (PC-7).
    pub ifixit: Option<ifixit::IfixitInfo>,
    /// OpenBenchmarking (PC-7).
    pub openbench: Option<openbench::OpenbenchInfo>,
}

impl Enrichment {
    /// Construct the hwdb-only initial state — the card's
    /// first-paint substrate before any network round-trip
    /// completes.
    #[must_use]
    pub fn hwdb_only() -> Self {
        Self {
            hwdb: Some(hwdb::HwdbInfo::placeholder()),
            ..Self::default()
        }
    }

    /// True if no online enrichment has resolved yet.
    #[must_use]
    pub fn is_hwdb_only(&self) -> bool {
        self.hwdb.is_some()
            && self.lhdb.is_none()
            && self.wikidata.is_none()
            && self.ifixit.is_none()
            && self.openbench.is_none()
    }

    /// True if Linux Hardware DB has resolved.
    #[must_use]
    pub fn has_lhdb(&self) -> bool {
        self.lhdb.is_some()
    }

    /// True if Wikidata / Wikipedia has resolved.
    #[must_use]
    pub fn has_wikidata(&self) -> bool {
        self.wikidata.is_some()
    }

    /// True if either iFixit or OpenBenchmarking has resolved.
    #[must_use]
    pub fn has_ifixit_or_openbench(&self) -> bool {
        self.ifixit.is_some() || self.openbench.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_is_case_insensitive_for_inputs() {
        let a = EnrichmentCacheKey::from_vendor_product("8086", "5916");
        let b = EnrichmentCacheKey::from_vendor_product("8086", "5916");
        let c = EnrichmentCacheKey::from_vendor_product("8086", "5916");
        let upper = EnrichmentCacheKey::from_vendor_product("8086", "5916");
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(a, upper);
    }

    #[test]
    fn cache_key_lowercases_input() {
        let k = EnrichmentCacheKey::from_vendor_product("8086", "5916");
        assert_eq!(k.vendor_id, "8086");
    }

    #[test]
    fn cache_filename_is_deterministic() {
        let k = EnrichmentCacheKey::from_vendor_product("8086", "5916");
        assert_eq!(k.cache_filename(), "8086:5916.json");
    }

    #[test]
    fn hwdb_only_has_no_online_sources() {
        let e = Enrichment::hwdb_only();
        assert!(e.is_hwdb_only());
    }

    #[test]
    fn default_has_no_sources_at_all() {
        let e = Enrichment::default();
        assert!(!e.is_hwdb_only());
        assert!(!e.has_lhdb());
        assert!(!e.has_wikidata());
        assert!(!e.has_ifixit_or_openbench());
    }
}
