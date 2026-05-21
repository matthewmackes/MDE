//! PC-7 — iFixit enrichment (online, optional).
//! Placeholder: production API call lands as part of PC-7.

use serde::{Deserialize, Serialize};

/// Resolved info from iFixit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IfixitInfo {
    /// Repairability score 1..=10 (iFixit's scale).
    pub repairability_score: u8,
    /// Teardown thumbnail URL.
    pub teardown_thumb_url: String,
}
