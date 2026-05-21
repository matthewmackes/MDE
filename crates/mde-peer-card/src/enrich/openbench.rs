//! PC-7 — OpenBenchmarking enrichment (online, optional).
//! Placeholder: production API call lands as part of PC-7.

use serde::{Deserialize, Serialize};

/// Resolved info from OpenBenchmarking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenbenchInfo {
    /// CPU percentile vs same model (0..=100).
    pub cpu_percentile: Option<u8>,
    /// GPU percentile vs same model.
    pub gpu_percentile: Option<u8>,
    /// SSD percentile vs same model.
    pub ssd_percentile: Option<u8>,
}
