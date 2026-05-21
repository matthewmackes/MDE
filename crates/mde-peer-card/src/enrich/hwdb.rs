//! PC-4 — Local hwdb / usb.ids resolver (offline, always-on).
//!
//! Real hwdb integration parses `/usr/share/hwdata/usb.ids` and
//! the systemd hwdb to resolve `vendor:product` → display names.
//! That production wiring is PC-4.a follow-up; this placeholder
//! ships the type surface so the card layout + tests can land.

use serde::{Deserialize, Serialize};

/// Local-resolved vendor/product display info.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HwdbInfo {
    /// Vendor display name (e.g. "Intel Corp.").
    pub vendor_name: String,
    /// Product display name (e.g. "UHD Graphics 620").
    pub product_name: String,
    /// Device class (e.g. "VGA compatible controller").
    pub device_class: String,
}

impl HwdbInfo {
    /// First-paint placeholder used while the production
    /// resolver isn't wired (PC-4.a).
    #[must_use]
    pub fn placeholder() -> Self {
        Self {
            vendor_name: "Unknown vendor".into(),
            product_name: "Unknown product".into(),
            device_class: "Generic device".into(),
        }
    }
}
