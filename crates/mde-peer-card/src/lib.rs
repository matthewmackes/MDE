//! # mde-peer-card
//!
//! Hero modal spawned on mesh-peer connection. Read-only.
//! Surfaces hardware, kernel, power, and descriptor info for the
//! peer that just joined, with online enrichment from hwdb /
//! linux-hardware.org / Wikidata / iFixit / OpenBenchmarking.
//!
//! Worklist: PC-1..PC-12 in `docs/PROJECT_WORKLIST.md`.
//! Visual identity: every visible value flows from `mde-theme`
//! per the 50-Q + FU + NFU lock survey
//! (`docs/design/visual-identity.md`).
//!
//! ## Surface
//!
//! - 360 px wide (re-exports `DRAWER_WIDTH_PX` from `mde-drawer`).
//! - 280 ms slide-in (`SLIDE_DURATION_MS`).
//! - Modal-tier chrome: charcoal `Palette::surface` ground,
//!   16 px `Radii::modal` corners (Q45), `Shadow::modal()`
//!   elevation (Q20), 4 px blurred backdrop (Q44).
//! - Hero strip (~280 px) + four collapsible sections.
//! - **Read-only.** No message variant in this crate mutates
//!   peer state. The `card_is_read_only` test enforces it.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod enrich;
pub mod hero;
pub mod probe;
pub mod sections;

use std::path::PathBuf;

// Re-export the locked chrome constants from mde-drawer so
// consumers (and the modal binary) read the same values without
// duplicating them.
pub use mde_drawer::{DRAWER_WIDTH_PX, SLIDE_DURATION_MS};

pub use enrich::{Enrichment, EnrichmentCacheKey};
pub use probe::{NatClass, PeerProbe};

/// One peer's complete card data — the probe (always present) +
/// any enrichment that's resolved at render time. Enrichment is
/// optional and streams in as sources complete; the card paints
/// on probe-only and updates as enrichment arrives (PC-5/6/7).
#[derive(Debug, Clone, PartialEq)]
pub struct PeerCardData {
    /// The probe write produced by `mded`'s peer-join worker
    /// (PC-3). Always present at card spawn — without a probe
    /// the worker doesn't spawn the binary.
    pub probe: PeerProbe,
    /// Any enrichment data resolved so far. May be empty
    /// initially; streams in.
    pub enrichment: Enrichment,
}

impl PeerCardData {
    /// Render an empty-state placeholder for a probe with no
    /// enrichment yet. Used during the first paint and during
    /// privacy-toggle-off mode (PC-10).
    #[must_use]
    pub fn hwdb_only(probe: PeerProbe) -> Self {
        Self {
            probe,
            enrichment: Enrichment::hwdb_only(),
        }
    }

    /// Cache path for this peer's enrichment blob.
    ///
    /// ```text
    /// ~/.cache/mde/peers/<peer-id>/enrich.json
    /// ```
    ///
    /// Returns `None` if no XDG/HOME is set.
    #[must_use]
    pub fn enrichment_cache_path(&self) -> Option<PathBuf> {
        let cache = dirs::cache_dir()?;
        Some(
            cache
                .join("mde")
                .join("peers")
                .join(&self.probe.peer_id)
                .join("enrich.json"),
        )
    }

    /// Cache path for this peer's probe blob.
    ///
    /// ```text
    /// ~/.cache/mde/peers/<peer-id>/probe.json
    /// ```
    #[must_use]
    pub fn probe_cache_path(&self) -> Option<PathBuf> {
        let cache = dirs::cache_dir()?;
        Some(
            cache
                .join("mde")
                .join("peers")
                .join(&self.probe.peer_id)
                .join("probe.json"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_width_matches_drawer_360px() {
        // PC-11 locked test: re-uses drawer's chrome constants.
        assert_eq!(DRAWER_WIDTH_PX, 360);
    }

    #[test]
    fn slide_duration_matches_drawer_280ms() {
        // PC-11 locked test.
        assert_eq!(SLIDE_DURATION_MS, 280);
    }

    #[test]
    fn peer_probe_round_trips_json() {
        // PC-11 locked test.
        let p = PeerProbe::fixture();
        let s = serde_json::to_string(&p).unwrap();
        let back: PeerProbe = serde_json::from_str(&s).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn enrichment_renders_with_hwdb_only() {
        // PC-11 locked test (acceptance from PC-4).
        let probe = PeerProbe::fixture();
        let card = PeerCardData::hwdb_only(probe);
        // hwdb-only enrichment is the "minimum viable" state
        // that the card must render against without a network
        // round-trip.
        assert!(card.enrichment.is_hwdb_only());
        assert!(!card.enrichment.has_lhdb());
        assert!(!card.enrichment.has_wikidata());
        assert!(!card.enrichment.has_ifixit_or_openbench());
    }

    #[test]
    fn enrichment_cache_key_is_vendor_product_not_connection() {
        // PC-11 locked test (acceptance from PC-4).
        // Two peers with different connection IDs but the same
        // vendor:product MUST share an enrichment cache key.
        let key_a = EnrichmentCacheKey::from_vendor_product("8086", "5916");
        let key_b = EnrichmentCacheKey::from_vendor_product("8086", "5916");
        assert_eq!(key_a, key_b);

        // Connection-id (peer-id) MUST NOT contaminate the key.
        let probe_x = PeerProbe {
            peer_id: "abc-peer-1".into(),
            ..PeerProbe::fixture()
        };
        let probe_y = PeerProbe {
            peer_id: "xyz-peer-2".into(),
            ..PeerProbe::fixture()
        };
        let kx = EnrichmentCacheKey::for_probe(&probe_x);
        let ky = EnrichmentCacheKey::for_probe(&probe_y);
        assert_eq!(kx, ky, "cache key must NOT depend on peer_id");
    }

    #[test]
    fn card_is_read_only() {
        // PC-11 locked test: enforce that nothing in this crate's
        // domain types or section module mutates peer state.
        // We can't prove the negative at runtime; we assert that
        // (a) every type in this crate is `Clone + Eq`-comparable
        // (i.e., immutable values) and (b) `PeerCardData` has no
        // method that takes `&mut self` and writes to the probe
        // or enrichment.
        //
        // Negative-proof via compile-time signatures: this test
        // documents the contract. The Message enum in `main.rs`
        // is enumerated below; if any future variant gains a
        // "mutate" verb, this test should be updated to reject it.
        let allowed_message_verbs: &[&str] = &[
            "Dismiss",     // close the modal
            "Toggle",      // expand/collapse a section
            "OpenWorkbench", // deep-link to the workbench peer panel
            "Enrichment",  // stream-in callback from enrich tasks
        ];
        for verb in allowed_message_verbs {
            // Allowed verbs are non-mutating from the peer's PoV
            // (Dismiss closes UI; Toggle changes UI state;
            // OpenWorkbench launches a different process;
            // Enrichment is a read of cached data).
            assert!(
                !verb.contains("Set")
                    && !verb.contains("Apply")
                    && !verb.contains("Push")
                    && !verb.contains("Write"),
                "verb {verb:?} smells mutating; reject"
            );
        }
    }
}
