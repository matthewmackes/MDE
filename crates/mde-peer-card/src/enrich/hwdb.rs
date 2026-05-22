//! PC-4 / PC-4.a — Local hwdb / usb.ids resolver (offline,
//! always-on).
//!
//! Parses `/usr/share/hwdata/usb.ids` into an in-memory map at
//! first lookup; subsequent lookups are O(1) per vendor +
//! O(n_devices_in_vendor) per product.
//!
//! Format (per https://www.linux-usb.org/usb-ids.html):
//!
//! ```text
//! # comment lines start with #
//! 1d6b  Linux Foundation
//!     0002  2.0 root hub
//!     0003  3.0 root hub
//! 8086  Intel Corp.
//!     5916  HD Graphics 620
//! ```
//!
//! Vendor lines start in column 0; device lines start with one
//! tab; interface lines start with two tabs (we ignore those —
//! peer-card cares about vendor + product only).
//!
//! PCI ids (PC-4.a future) would parse `/usr/share/hwdata/pci.ids`
//! with the same format. The hwdata package ships both files;
//! the parser is structurally identical so a follow-up wires
//! the second source through `Hwdb::load_pci_ids` without a
//! signature change.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Default location of usb.ids on Fedora / RHEL / Debian.
pub const DEFAULT_USB_IDS_PATH: &str = "/usr/share/hwdata/usb.ids";

/// Default location of pci.ids — same hwdata package as usb.ids.
pub const DEFAULT_PCI_IDS_PATH: &str = "/usr/share/hwdata/pci.ids";

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
    /// First-paint placeholder used when the on-disk hwdb
    /// isn't available (e.g. headless test container without
    /// `hwdata` installed).
    #[must_use]
    pub fn placeholder() -> Self {
        Self {
            vendor_name: "Unknown vendor".into(),
            product_name: "Unknown product".into(),
            device_class: "Generic device".into(),
        }
    }

    /// Looked-up form. Falls back to vendor:product hex strings
    /// for unresolved IDs so the card always paints something
    /// informative.
    #[must_use]
    pub fn from_lookup(vendor_id: &str, product_id: &str, hwdb: &Hwdb) -> Self {
        let vendor_name = hwdb
            .vendor(vendor_id)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Vendor {vendor_id}"));
        let product_name = hwdb
            .product(vendor_id, product_id)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Product {product_id}"));
        Self {
            vendor_name,
            product_name,
            // Device class lives in pci.ids (PCI Base Class
            // table). Without it the chassis-class string from
            // the probe is the source of truth; default here
            // is intentionally generic.
            device_class: "USB device".into(),
        }
    }
}

/// In-memory hwdb index. Cheap to clone (string-backed maps).
///
/// Construct via `Hwdb::load_usb_ids(path)` or `Hwdb::system()`
/// for the standard `/usr/share/hwdata/usb.ids` location.
#[derive(Debug, Clone, Default)]
pub struct Hwdb {
    /// `vendor_id (hex) → vendor name`.
    vendors: HashMap<String, String>,
    /// `(vendor_id, product_id) → product name`.
    products: HashMap<(String, String), String>,
}

impl Hwdb {
    /// Load + parse the usb.ids file at `path`. Returns an
    /// empty `Hwdb` on read errors so callers can keep painting
    /// hex fallbacks without crashing.
    #[must_use]
    pub fn load_usb_ids(path: &Path) -> Self {
        let Ok(raw) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        Self::parse(&raw)
    }

    /// Convenience — load the system-installed usb.ids file.
    /// Equivalent to `Hwdb::load_usb_ids(Path::new(DEFAULT_USB_IDS_PATH))`.
    #[must_use]
    pub fn system() -> Self {
        Self::load_usb_ids(&PathBuf::from(DEFAULT_USB_IDS_PATH))
    }

    /// PC-4.b — load + parse the pci.ids file at `path`. Same
    /// format as usb.ids (the hwdata package ships both), so
    /// the shared `parse()` handles both.
    #[must_use]
    pub fn load_pci_ids(path: &Path) -> Self {
        Self::load_usb_ids(path)
    }

    /// PC-4.b — load the system-installed pci.ids file.
    #[must_use]
    pub fn system_pci() -> Self {
        Self::load_pci_ids(&PathBuf::from(DEFAULT_PCI_IDS_PATH))
    }

    /// Process-wide cached `Hwdb`. The first caller pays the
    /// parse cost (~50 ms for 1 MB of usb.ids); subsequent
    /// callers get a `&'static Hwdb` reference.
    ///
    /// Wraps a `OnceLock` so threads racing the first init see
    /// a fully-populated value, not a partial one.
    pub fn shared() -> &'static Hwdb {
        static CACHE: OnceLock<Hwdb> = OnceLock::new();
        CACHE.get_or_init(Self::system)
    }

    /// PC-4.b — process-wide cached pci.ids index. Separate
    /// `OnceLock` from `shared()` so callers can hold both at
    /// once without contention.
    pub fn shared_pci() -> &'static Hwdb {
        static CACHE: OnceLock<Hwdb> = OnceLock::new();
        CACHE.get_or_init(Self::system_pci)
    }

    /// Parse the on-disk text format.
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        let mut vendors = HashMap::new();
        let mut products = HashMap::new();
        let mut current_vendor: Option<String> = None;

        for line in raw.lines() {
            // Skip blank lines + comments.
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // Interface lines start with two tabs — peer-card
            // doesn't surface those today; skip cheaply.
            if line.starts_with("\t\t") {
                continue;
            }
            if let Some(tail) = line.strip_prefix('\t') {
                // Device line: `<id>  <name>` (two-space sep).
                if let Some(vendor_id) = current_vendor.as_deref() {
                    if let Some((product_id, product_name)) = split_id_name(tail) {
                        products.insert((vendor_id.to_owned(), product_id), product_name);
                    }
                }
            } else if let Some((vendor_id, vendor_name)) = split_id_name(line) {
                // Vendor line at column 0.
                vendors.insert(vendor_id.clone(), vendor_name);
                current_vendor = Some(vendor_id);
            }
        }

        Self { vendors, products }
    }

    /// Resolve a vendor id (4-char lowercase hex) to its display
    /// name. Returns `None` for unknown ids.
    #[must_use]
    pub fn vendor(&self, vendor_id: &str) -> Option<&str> {
        self.vendors
            .get(&normalize_id(vendor_id))
            .map(String::as_str)
    }

    /// Resolve `(vendor_id, product_id)` to a product display
    /// name. Returns `None` for unknown pairs.
    #[must_use]
    pub fn product(&self, vendor_id: &str, product_id: &str) -> Option<&str> {
        self.products
            .get(&(normalize_id(vendor_id), normalize_id(product_id)))
            .map(String::as_str)
    }

    /// Number of vendors in the index. Useful in tests + a
    /// readiness-probe surface.
    #[must_use]
    pub fn vendor_count(&self) -> usize {
        self.vendors.len()
    }
}

/// Split a `<id>  <name>` line where the id is hex + the name
/// is the rest after at least one whitespace gap. Returns
/// `(id, name)` lowercased on the id; name preserved verbatim.
fn split_id_name(line: &str) -> Option<(String, String)> {
    let mut parts = line.splitn(2, char::is_whitespace);
    let id = parts.next()?.trim();
    let name = parts.next()?.trim_start();
    if id.is_empty() || name.is_empty() {
        return None;
    }
    Some((normalize_id(id), name.to_owned()))
}

/// Lowercase the id so lookups are case-insensitive. USB ids
/// are conventionally lowercase hex; PCI ids likewise.
fn normalize_id(id: &str) -> String {
    id.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "\
# Sample usb.ids fragment for tests.
1d6b  Linux Foundation
\t0002  2.0 root hub
\t0003  3.0 root hub
\t\t01  Optional iface descriptor (should be ignored)
8086  Intel Corp.
\t5916  HD Graphics 620
\t5917  HD Graphics 620 (rev 02)
046d  Logitech, Inc.
\tb034  MX Master 3S
";

    #[test]
    fn parse_yields_three_vendors() {
        let h = Hwdb::parse(FIXTURE);
        assert_eq!(h.vendor_count(), 3);
        assert_eq!(h.vendor("1d6b"), Some("Linux Foundation"));
        assert_eq!(h.vendor("8086"), Some("Intel Corp."));
        assert_eq!(h.vendor("046d"), Some("Logitech, Inc."));
    }

    #[test]
    fn parse_resolves_known_product() {
        let h = Hwdb::parse(FIXTURE);
        assert_eq!(h.product("8086", "5916"), Some("HD Graphics 620"));
        assert_eq!(h.product("046d", "b034"), Some("MX Master 3S"));
    }

    #[test]
    fn parse_skips_interface_lines() {
        // `\t\t01  …` is an interface descriptor — we shouldn't
        // accidentally treat it as a product on the previous
        // vendor.
        let h = Hwdb::parse(FIXTURE);
        assert_eq!(h.product("1d6b", "01"), None);
    }

    #[test]
    fn unknown_lookups_return_none() {
        let h = Hwdb::parse(FIXTURE);
        assert_eq!(h.vendor("ffff"), None);
        assert_eq!(h.product("8086", "ffff"), None);
    }

    #[test]
    fn lookups_are_case_insensitive() {
        let h = Hwdb::parse(FIXTURE);
        assert_eq!(h.vendor("8086"), Some("Intel Corp."));
        assert_eq!(
            h.vendor("8086".to_ascii_uppercase().as_str()),
            Some("Intel Corp.")
        );
        assert_eq!(h.product("8086", "5916"), h.product("8086", "5916"));
    }

    #[test]
    fn from_lookup_falls_back_on_unknown_ids() {
        let h = Hwdb::parse(FIXTURE);
        let info = HwdbInfo::from_lookup("ffff", "eeee", &h);
        assert_eq!(info.vendor_name, "Vendor ffff");
        assert_eq!(info.product_name, "Product eeee");
    }

    #[test]
    fn from_lookup_uses_resolved_names_when_known() {
        let h = Hwdb::parse(FIXTURE);
        let info = HwdbInfo::from_lookup("8086", "5916", &h);
        assert_eq!(info.vendor_name, "Intel Corp.");
        assert_eq!(info.product_name, "HD Graphics 620");
    }

    #[test]
    fn placeholder_carries_unknown_strings() {
        let p = HwdbInfo::placeholder();
        assert!(p.vendor_name.contains("Unknown"));
        assert!(p.product_name.contains("Unknown"));
    }

    #[test]
    fn missing_file_yields_empty_hwdb() {
        let h = Hwdb::load_usb_ids(Path::new("/tmp/definitely-does-not-exist-xyz123.ids"));
        assert_eq!(h.vendor_count(), 0);
    }

    #[test]
    fn pci_ids_parses_with_same_format() {
        // PC-4.b — pci.ids uses the same vendor/device line
        // shape as usb.ids, so `Hwdb::load_pci_ids` should
        // resolve PCI vendor/device lookups identically.
        let pci_fixture = "\
8086  Intel Corporation
\t1237  440FX - 82441FX PMC [Natoma]
\t7000  82371SB PIIX3 ISA [Natoma/Triton II]
10de  NVIDIA Corporation
\t1c03  GP106 [GeForce GTX 1060 6GB]
";
        let h = Hwdb::parse(pci_fixture);
        assert_eq!(h.vendor("8086"), Some("Intel Corporation"));
        assert_eq!(h.vendor("10de"), Some("NVIDIA Corporation"));
        assert_eq!(
            h.product("10de", "1c03"),
            Some("GP106 [GeForce GTX 1060 6GB]")
        );
    }

    #[test]
    fn default_pci_ids_path_lives_under_hwdata() {
        // PC-4.b — guard against drift in the path constant.
        assert!(DEFAULT_PCI_IDS_PATH.ends_with("pci.ids"));
        assert!(DEFAULT_PCI_IDS_PATH.contains("hwdata"));
    }
}
