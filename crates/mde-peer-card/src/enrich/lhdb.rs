//! PC-5 — linux-hardware.org enrichment (online, optional).
//! Placeholder: production HTTP query lands as part of PC-5.

use serde::{Deserialize, Serialize};

/// Resolved info from linux-hardware.org.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LhdbInfo {
    /// Whether the bound driver is upstream-reported working.
    pub driver_supported: bool,
    /// Earliest kernel version that reports support
    /// (e.g. `"5.10"`).
    pub min_kernel: Option<String>,
    /// Number of similar-machine probe reports on file.
    pub similar_probes: u32,
}
