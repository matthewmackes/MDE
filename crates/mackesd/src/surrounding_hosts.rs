//! MESH-A-4.a (v5.0.0) — surrounding-host taxonomy + classifier.
//!
//! A "surrounding host" is a LAN neighbour that is **not** a mesh peer
//! (R8-Q1..Q15, design doc §7.3). This module owns the two locked
//! enumerations — the 14 [`HostType`]s (R8-Q9) and the 3
//! [`TrustState`]s (R8-Q10) — plus the pure [`classify`] heuristic
//! that turns a discovery pass's [`HostSignals`] into a best-guess
//! type. `TrustState` serialises to the same lowercase strings the
//! `mde_card::probe::HostFacts.trust_state` field already carries (its
//! doc-comment names this module as the taxonomy owner).
//!
//! The discovery collectors that gather [`HostSignals`] from the wire
//! (mDNS / ARP / OUI / reverse-DNS / HTTP-banner / nmap fingerprint)
//! land in MESH-A-4.b; the worker that stores + mesh-syncs the
//! `SurroundingHost` records lands in MESH-A-4.c. This sub-task ships
//! the taxonomy + classifier + the `mackesd classify-host` CLI that
//! exercises it end-to-end.
//!
//! ## Classification heuristics (best-choice — no design lock)
//!
//! The design doc locks the 14 types but not the rules that infer
//! them, so [`classify`] uses a confidence-ordered cascade:
//!
//! 1. **mDNS service type** (strongest) — a printer announces
//!    `_ipp._tcp`, a Chromecast `_googlecast._tcp`, a NAS `_smb._tcp`.
//! 2. **MAC-OUI vendor** — disambiguates network gear, cameras,
//!    printers, NAS, consoles that don't announce mDNS.
//! 3. **Open ports** (weakest) — only the few unambiguous ones
//!    (9100 raw-print → Printer, 554 RTSP → Camera).
//!
//! Anything unmatched is [`HostType::Unknown`] — the classifier never
//! guesses past its confidence. Switch / Ap / Server need richer
//! signals (SNMP sysObjectID, LLDP) deferred to MESH-A-4.b; they are
//! valid taxonomy members reachable for manual assignment today.

use std::process::Command;

/// One of the 14 surrounding-host types (R8-Q9). Wire form is the
/// kebab-case [`HostType::wire_name`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostType {
    /// Home/office gateway router.
    Router,
    /// Managed or unmanaged network switch.
    Switch,
    /// Wireless access point.
    Ap,
    /// Network printer or scanner.
    Printer,
    /// Network-attached storage / file server.
    Nas,
    /// IP camera or NVR.
    Camera,
    /// Casting / streaming video target (Chromecast, AirPlay, Roku).
    TvCast,
    /// Smart speaker / audio receiver (Sonos, Echo, AirPlay audio).
    SmartSpeaker,
    /// Generic IoT / home-automation device.
    Iot,
    /// Phone or tablet handheld.
    Phone,
    /// Desktop or laptop computer.
    Computer,
    /// Headless server host.
    Server,
    /// Game console (PlayStation, Nintendo, Xbox).
    GameConsole,
    /// Unclassified — the signals matched no known type.
    Unknown,
}

impl HostType {
    /// Stable kebab-case wire name (matches the serde rename).
    #[must_use]
    pub fn wire_name(self) -> &'static str {
        match self {
            HostType::Router => "router",
            HostType::Switch => "switch",
            HostType::Ap => "ap",
            HostType::Printer => "printer",
            HostType::Nas => "nas",
            HostType::Camera => "camera",
            HostType::TvCast => "tv-cast",
            HostType::SmartSpeaker => "smart-speaker",
            HostType::Iot => "iot",
            HostType::Phone => "phone",
            HostType::Computer => "computer",
            HostType::Server => "server",
            HostType::GameConsole => "game-console",
            HostType::Unknown => "unknown",
        }
    }
}

/// Trust classification (R8-Q10). Serialises to the lowercase strings
/// `mde_card::probe::HostFacts.trust_state` carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustState {
    /// Operator-trusted neighbour.
    Trusted,
    /// Seen but not yet trusted — the default for a freshly-discovered
    /// host.
    Unknown,
    /// Operator-blocked; MESH-A-5 enforces the mesh-wide firewall DROP.
    Blocked,
}

impl Default for TrustState {
    fn default() -> Self {
        // A freshly-seen neighbour is untrusted-but-not-blocked.
        TrustState::Unknown
    }
}

/// The signals a discovery pass (MESH-A-4.b) gathers about a host,
/// fed to [`classify`]. All fields optional/empty — a host seen only
/// in the ARP table has just an `oui_vendor`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HostSignals {
    /// mDNS service types advertised (`_ipp._tcp`, `_airplay._tcp`, …).
    #[serde(default)]
    pub mdns_services: Vec<String>,
    /// Open TCP ports observed.
    #[serde(default)]
    pub open_ports: Vec<u16>,
    /// MAC-OUI vendor string (`Hewlett Packard`, `Ubiquiti Inc`, …).
    #[serde(default)]
    pub oui_vendor: String,
    /// Hostname (mDNS / reverse-DNS), used for the console hostname
    /// hint (MESH-A-4.b.2). Empty when unknown.
    #[serde(default)]
    pub hostname: String,
}

/// A discovered surrounding host (a LAN neighbour that is not a mesh
/// peer). Built by the MESH-A-4.b collectors; the A-4.c worker stores
/// + mesh-syncs these records. (The A-4.a note pencilled this struct
/// in for A-4.c; it lands here in A-4.b.1, where the mDNS sweep first
/// constructs it.)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SurroundingHost {
    /// IPv4/IPv6 address.
    pub ip: String,
    /// MAC address (empty until an ARP/OUI pass fills it — A-4.b.2).
    #[serde(default)]
    pub mac: String,
    /// MAC-OUI vendor (empty until A-4.b.2).
    #[serde(default)]
    pub vendor: String,
    /// Hostname (mDNS / reverse-DNS; may be empty).
    #[serde(default)]
    pub hostname: String,
    /// Advertised service identifiers (mDNS service types today).
    #[serde(default)]
    pub services: Vec<String>,
    /// Classified host type.
    pub host_type: HostType,
    /// Trust state (defaults to Unknown for a freshly-seen host).
    #[serde(default)]
    pub trust: TrustState,
    /// Unix-epoch ms first seen.
    pub first_seen_ms: i64,
    /// Unix-epoch ms last seen.
    pub last_seen_ms: i64,
}

/// One resolved mDNS service record (an `avahi-browse` `=` line).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MdnsService {
    /// Resolved address.
    pub ip: String,
    /// Resolved hostname.
    pub hostname: String,
    /// Service type (`_ipp._tcp`, `_googlecast._tcp`, …).
    pub service_type: String,
}

/// Classify a host from its discovery signals into one of the 14
/// [`HostType`]s. See the module docs for the confidence cascade;
/// returns [`HostType::Unknown`] when nothing matches.
#[must_use]
pub fn classify(sig: &HostSignals) -> HostType {
    // 1. Console hostname hint — highest confidence for the specific
    //    `PS4-`/`Xbox-`/`Nintendo-` patterns, and it must outrank a
    //    media service the console also advertises (a PS4 announces
    //    `_spotify-connect._tcp`, which would otherwise read as a
    //    smart speaker).
    if let Some(t) = host_type_from_hostname(&sig.hostname) {
        return t;
    }
    // 2. mDNS service type — strongest generic signal.
    for svc in &sig.mdns_services {
        if let Some(t) = host_type_from_mdns(svc) {
            return t;
        }
    }
    // 3. MAC-OUI vendor.
    if let Some(t) = host_type_from_vendor(&sig.oui_vendor) {
        return t;
    }
    // 4. Open ports — weakest, only the unambiguous ones.
    for &port in &sig.open_ports {
        if let Some(t) = host_type_from_port(port) {
            return t;
        }
    }
    HostType::Unknown
}

/// Map an mDNS service type to a host type. Substring match so a full
/// `_ipp._tcp.local.` instance name still resolves.
fn host_type_from_mdns(service: &str) -> Option<HostType> {
    let s = service.to_ascii_lowercase();
    if s.contains("_ipp")
        || s.contains("_printer")
        || s.contains("_pdl-datastream")
        || s.contains("_scanner")
        || s.contains("_uscan")
    {
        return Some(HostType::Printer);
    }
    if s.contains("_googlecast")
        || s.contains("_airplay")
        || s.contains("_amzn-wplay")
        || s.contains("_roku")
        || s.contains("_androidtvremote")
    {
        return Some(HostType::TvCast);
    }
    if s.contains("_raop") || s.contains("_spotify-connect") || s.contains("_sonos") {
        return Some(HostType::SmartSpeaker);
    }
    if s.contains("_smb")
        || s.contains("_afpovertcp")
        || s.contains("_nfs")
        || s.contains("_adisk")
        || s.contains("_webdav")
    {
        return Some(HostType::Nas);
    }
    if s.contains("_axis-video") || s.contains("_rtsp") || s.contains("_onvif") {
        return Some(HostType::Camera);
    }
    if s.contains("_hap")
        || s.contains("_homekit")
        || s.contains("_matter")
        || s.contains("_hue")
        || s.contains("_coap")
    {
        return Some(HostType::Iot);
    }
    if s.contains("_apple-mobdev") || s.contains("_companion-link") {
        return Some(HostType::Phone);
    }
    if s.contains("_workstation") {
        return Some(HostType::Computer);
    }
    None
}

/// Map a MAC-OUI vendor string to a host type. Case-insensitive
/// substring match against well-known vendor tokens.
fn host_type_from_vendor(vendor: &str) -> Option<HostType> {
    let v = vendor.to_ascii_lowercase();
    if v.is_empty() {
        return None;
    }
    // Network infrastructure — router/AP/switch are hard to split by
    // vendor alone, so map to Router (the common LAN gateway device).
    for needle in [
        "ubiquiti", "cisco", "netgear", "tp-link", "tplink", "mikrotik", "asustek",
        "d-link", "dlink", "aruba", "ruckus", "juniper", "zyxel", "fortinet",
    ] {
        if v.contains(needle) {
            return Some(HostType::Router);
        }
    }
    for needle in ["hewlett", "hp inc", "canon", "epson", "brother", "lexmark", "xerox", "kyocera"]
    {
        if v.contains(needle) {
            return Some(HostType::Printer);
        }
    }
    for needle in ["hikvision", "dahua", "axis communications", "reolink", "wyze", "amcrest"] {
        if v.contains(needle) {
            return Some(HostType::Camera);
        }
    }
    for needle in ["synology", "qnap", "western digital", "drobo"] {
        if v.contains(needle) {
            return Some(HostType::Nas);
        }
    }
    for needle in ["sonos", "bose", "harman"] {
        if v.contains(needle) {
            return Some(HostType::SmartSpeaker);
        }
    }
    for needle in ["nintendo", "sony interactive"] {
        if v.contains(needle) {
            return Some(HostType::GameConsole);
        }
    }
    if v.contains("raspberry") {
        return Some(HostType::Computer);
    }
    None
}

/// Map an open port to a host type — only the few unambiguous ports.
fn host_type_from_port(port: u16) -> Option<HostType> {
    match port {
        9100 => Some(HostType::Printer), // raw print / JetDirect
        554 => Some(HostType::Camera),   // RTSP
        _ => None,
    }
}

/// Map a hostname to a host type for the few high-confidence patterns
/// (MESH-A-4.b.2). Today only game consoles, whose hostnames
/// (`PS4-…`, `Xbox-…`, `Nintendo-…`) are far more reliable than the
/// media services they also advertise. Case-insensitive substring
/// match; `None` for generic hostnames.
fn host_type_from_hostname(hostname: &str) -> Option<HostType> {
    let h = hostname.to_ascii_lowercase();
    if h.is_empty() {
        return None;
    }
    for needle in ["ps4", "ps5", "playstation", "xbox", "nintendo"] {
        if h.contains(needle) {
            return Some(HostType::GameConsole);
        }
    }
    None
}

/// Parse `avahi-browse -aprt` output into resolved mDNS service
/// records. Only `=` (resolved) lines carry an address; `+` (browse)
/// lines are skipped. Fields are `;`-separated:
/// `=;iface;proto;name;type;domain;hostname;address;port;txt…`.
#[must_use]
pub fn parse_avahi_browse(stdout: &str) -> Vec<MdnsService> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        if !line.starts_with('=') {
            continue;
        }
        let f: Vec<&str> = line.split(';').collect();
        if f.len() < 8 {
            continue;
        }
        let service_type = f[4].trim().to_string();
        let hostname = f[6].trim().to_string();
        let ip = f[7].trim().to_string();
        if ip.is_empty() || service_type.is_empty() {
            continue;
        }
        out.push(MdnsService {
            ip,
            hostname,
            service_type,
        });
    }
    out
}

/// Group resolved mDNS records by IP into [`SurroundingHost`]s,
/// classifying each from its advertised service types. `now_ms`
/// stamps first/last-seen. Pure over the already-collected records.
#[must_use]
pub fn hosts_from_mdns(records: &[MdnsService], now_ms: i64) -> Vec<SurroundingHost> {
    use std::collections::BTreeMap;
    // ip -> (hostname, service-types in first-seen order)
    let mut by_ip: BTreeMap<String, (String, Vec<String>)> = BTreeMap::new();
    for r in records {
        let entry = by_ip
            .entry(r.ip.clone())
            .or_insert_with(|| (r.hostname.clone(), Vec::new()));
        if entry.0.is_empty() && !r.hostname.is_empty() {
            entry.0 = r.hostname.clone();
        }
        if !entry.1.contains(&r.service_type) {
            entry.1.push(r.service_type.clone());
        }
    }
    by_ip
        .into_iter()
        .map(|(ip, (hostname, services))| {
            let sig = HostSignals {
                mdns_services: services.clone(),
                hostname: hostname.clone(),
                ..Default::default()
            };
            SurroundingHost {
                ip,
                mac: String::new(),
                vendor: String::new(),
                hostname,
                services,
                host_type: classify(&sig),
                trust: TrustState::default(),
                first_seen_ms: now_ms,
                last_seen_ms: now_ms,
            }
        })
        .collect()
}

/// Browse the LAN for mDNS services via `avahi-browse -aprt` and parse
/// the resolved records. Returns empty when `binary` is absent
/// (headless / air-gapped peer) or exits non-zero. The shell-out is
/// HW-bench-gated like the netassess collectors; [`parse_avahi_browse`]
/// is the unit-tested pure half.
#[must_use]
pub fn collect_mdns(binary: &str) -> Vec<MdnsService> {
    let Ok(out) = Command::new(binary).args(["-a", "-p", "-r", "-t"]).output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    parse_avahi_browse(&String::from_utf8_lossy(&out.stdout))
}

/// Parse `getent hosts <ip>` output into the resolved hostname. The
/// line is `<address>   <canonical-name> [aliases…]`; returns the
/// canonical name, or `None` when there is no name field.
#[must_use]
pub fn parse_getent_hosts(output: &str) -> Option<String> {
    output
        .split_whitespace()
        .nth(1)
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

/// Reverse-resolve `ip` to a hostname via `getent hosts` (the system
/// resolver — DNS PTR + `/etc/hosts` + mDNS). `None` when unresolved
/// or `getent` is absent. HW-bench-gated shell-out; the pure half is
/// [`parse_getent_hosts`].
#[must_use]
pub fn reverse_dns(ip: &str) -> Option<String> {
    let out = Command::new("getent").args(["hosts", ip]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    parse_getent_hosts(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig_mdns(svc: &str) -> HostSignals {
        HostSignals {
            mdns_services: vec![svc.to_string()],
            ..Default::default()
        }
    }

    fn sig_vendor(vendor: &str) -> HostSignals {
        HostSignals {
            oui_vendor: vendor.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn mdns_printer_cast_nas_speaker_camera() {
        assert_eq!(classify(&sig_mdns("_ipp._tcp.local.")), HostType::Printer);
        assert_eq!(classify(&sig_mdns("_googlecast._tcp")), HostType::TvCast);
        assert_eq!(classify(&sig_mdns("_smb._tcp")), HostType::Nas);
        assert_eq!(classify(&sig_mdns("_raop._tcp")), HostType::SmartSpeaker);
        assert_eq!(classify(&sig_mdns("_rtsp._tcp")), HostType::Camera);
    }

    #[test]
    fn vendor_router_camera_console_nas() {
        assert_eq!(classify(&sig_vendor("Ubiquiti Inc")), HostType::Router);
        assert_eq!(classify(&sig_vendor("Hikvision Digital")), HostType::Camera);
        assert_eq!(classify(&sig_vendor("Nintendo Co., Ltd.")), HostType::GameConsole);
        assert_eq!(classify(&sig_vendor("Synology Incorporated")), HostType::Nas);
        assert_eq!(classify(&sig_vendor("Hewlett Packard")), HostType::Printer);
    }

    #[test]
    fn port_fallback_only_for_unambiguous_ports() {
        let printer = HostSignals { open_ports: vec![9100], ..Default::default() };
        assert_eq!(classify(&printer), HostType::Printer);
        let camera = HostSignals { open_ports: vec![554], ..Default::default() };
        assert_eq!(classify(&camera), HostType::Camera);
        // 443 alone is too generic — stays Unknown.
        let web = HostSignals { open_ports: vec![443], ..Default::default() };
        assert_eq!(classify(&web), HostType::Unknown);
    }

    #[test]
    fn mdns_outranks_vendor_and_port() {
        // A printer behind a Ubiquiti-OUI NIC on port 443 still reads
        // as a printer from its mDNS announce.
        let sig = HostSignals {
            mdns_services: vec!["_ipp._tcp".to_string()],
            open_ports: vec![443],
            oui_vendor: "Ubiquiti Inc".to_string(),
            hostname: String::new(),
        };
        assert_eq!(classify(&sig), HostType::Printer);
    }

    #[test]
    fn empty_signals_are_unknown() {
        assert_eq!(classify(&HostSignals::default()), HostType::Unknown);
        assert_eq!(classify(&sig_vendor("Totally Unknown Vendor")), HostType::Unknown);
    }

    #[test]
    fn all_14_host_types_have_distinct_wire_names() {
        let all = [
            HostType::Router,
            HostType::Switch,
            HostType::Ap,
            HostType::Printer,
            HostType::Nas,
            HostType::Camera,
            HostType::TvCast,
            HostType::SmartSpeaker,
            HostType::Iot,
            HostType::Phone,
            HostType::Computer,
            HostType::Server,
            HostType::GameConsole,
            HostType::Unknown,
        ];
        let names: std::collections::HashSet<&str> = all.iter().map(|t| t.wire_name()).collect();
        assert_eq!(names.len(), 14, "all 14 wire names distinct");
    }

    #[test]
    fn host_type_serde_matches_wire_name() {
        assert_eq!(serde_json::to_string(&HostType::TvCast).unwrap(), "\"tv-cast\"");
        assert_eq!(serde_json::to_string(&HostType::GameConsole).unwrap(), "\"game-console\"");
        assert_eq!(serde_json::to_string(&HostType::Ap).unwrap(), "\"ap\"");
    }

    #[test]
    fn trust_state_serializes_to_hostfacts_lowercase_strings() {
        assert_eq!(serde_json::to_string(&TrustState::Trusted).unwrap(), "\"trusted\"");
        assert_eq!(serde_json::to_string(&TrustState::Unknown).unwrap(), "\"unknown\"");
        assert_eq!(serde_json::to_string(&TrustState::Blocked).unwrap(), "\"blocked\"");
        assert_eq!(TrustState::default(), TrustState::Unknown);
    }

    // ── MESH-A-4.b.1: mDNS collector ──

    #[test]
    fn parse_avahi_browse_keeps_resolved_skips_browse_lines() {
        let raw = "+;eth0;IPv4;HP\\032LaserJet;_ipp._tcp;local\n\
                   =;eth0;IPv4;HP\\032LaserJet;_ipp._tcp;local;printer.local;192.168.1.50;631;\"txtvers=1\"\n\
                   =;eth0;IPv4;Chromecast;_googlecast._tcp;local;cast.local;192.168.1.60;8009;\"\"\n";
        let recs = parse_avahi_browse(raw);
        assert_eq!(recs.len(), 2, "the + browse line is skipped");
        assert_eq!(recs[0].ip, "192.168.1.50");
        assert_eq!(recs[0].service_type, "_ipp._tcp");
        assert_eq!(recs[0].hostname, "printer.local");
        assert_eq!(recs[1].ip, "192.168.1.60");
        assert_eq!(recs[1].service_type, "_googlecast._tcp");
    }

    #[test]
    fn hosts_from_mdns_groups_by_ip_and_classifies() {
        let recs = vec![
            MdnsService {
                ip: "192.168.1.50".into(),
                hostname: "printer.local".into(),
                service_type: "_ipp._tcp".into(),
            },
            MdnsService {
                ip: "192.168.1.50".into(),
                hostname: "printer.local".into(),
                service_type: "_pdl-datastream._tcp".into(),
            },
            MdnsService {
                ip: "192.168.1.60".into(),
                hostname: "cast.local".into(),
                service_type: "_googlecast._tcp".into(),
            },
        ];
        let hosts = hosts_from_mdns(&recs, 1234);
        assert_eq!(hosts.len(), 2, "two distinct IPs → two hosts");
        let printer = hosts.iter().find(|h| h.ip == "192.168.1.50").unwrap();
        assert_eq!(printer.host_type, HostType::Printer);
        assert_eq!(printer.services.len(), 2, "both service types retained");
        assert_eq!(printer.hostname, "printer.local");
        assert_eq!(printer.first_seen_ms, 1234);
        assert_eq!(printer.last_seen_ms, 1234);
        assert_eq!(printer.trust, TrustState::Unknown);
        assert!(printer.mac.is_empty(), "MAC fills in A-4.b.2");
        let cast = hosts.iter().find(|h| h.ip == "192.168.1.60").unwrap();
        assert_eq!(cast.host_type, HostType::TvCast);
    }

    // ── MESH-A-4.b.2: hostname hint + reverse-DNS ──

    #[test]
    fn console_hostname_hint_outranks_media_service() {
        // A PS4 advertises _spotify-connect (→ smart-speaker by service
        // type) but its hostname pins it to a game console.
        let sig = HostSignals {
            mdns_services: vec!["_spotify-connect._tcp".to_string()],
            hostname: "PS4-64F7B2.local".to_string(),
            ..Default::default()
        };
        assert_eq!(classify(&sig), HostType::GameConsole);
    }

    #[test]
    fn host_type_from_hostname_matches_consoles_only() {
        assert_eq!(host_type_from_hostname("PS5-1234"), Some(HostType::GameConsole));
        assert_eq!(host_type_from_hostname("Xbox-Living-Room"), Some(HostType::GameConsole));
        assert_eq!(host_type_from_hostname("nintendo-switch"), Some(HostType::GameConsole));
        assert_eq!(host_type_from_hostname("fileserver.local"), None);
        assert_eq!(host_type_from_hostname(""), None);
    }

    #[test]
    fn empty_hostname_preserves_prior_classification() {
        // No hostname → mDNS still wins (A-4.a behaviour unchanged).
        let sig = HostSignals {
            mdns_services: vec!["_ipp._tcp".to_string()],
            ..Default::default()
        };
        assert_eq!(classify(&sig), HostType::Printer);
    }

    #[test]
    fn parse_getent_hosts_extracts_canonical_name() {
        assert_eq!(
            parse_getent_hosts("192.168.1.50   printer.local").as_deref(),
            Some("printer.local")
        );
        assert_eq!(
            parse_getent_hosts("192.168.1.60 cast.local alias1 alias2").as_deref(),
            Some("cast.local")
        );
        assert_eq!(parse_getent_hosts(""), None);
        assert_eq!(parse_getent_hosts("192.168.1.99"), None); // no name field
    }
}
