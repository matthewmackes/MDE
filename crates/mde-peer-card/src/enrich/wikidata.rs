//! PC-6 — Wikidata + Wikipedia enrichment (online, optional).
//! Placeholder: production SPARQL query + Wikipedia REST fetch
//! lands as part of PC-6.

use serde::{Deserialize, Serialize};

/// Resolved info from Wikidata + Wikipedia.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikidataInfo {
    /// Manufacturer canonical name from Wikidata.
    pub manufacturer: String,
    /// Release year (4-digit Common Era).
    pub release_year: Option<u16>,
    /// Wikipedia REST summary — 2 sentences max per UX-21.
    pub summary: String,
    /// Hero image URL on Wikimedia Commons, or `None` if no
    /// image was found.
    pub hero_image_url: Option<String>,
}
