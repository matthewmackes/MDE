//! KDC2-3.7+3.8 — file-backed pairing store.
//!
//! Owns `~/.config/mde/connect/` (or `$XDG_CONFIG_HOME/mde/connect/`):
//!
//! ```text
//! ~/.config/mde/connect/
//!   ├── identity.pem        # PEM-wrapped PKCS#8 RSA-2048 private key
//!   └── devices.toml        # paired peers + public keys + last seen
//! ```
//!
//! Locked decisions (v2.1 KDC2):
//!
//!   * **RSA-2048, NOT Ed25519.** The KDC2-3.7 spec mentioned
//!     Ed25519, but upstream KDE Connect's wire protocol uses
//!     RSA-PKCS1-v1_5/SHA-256 for handshake signatures. Going
//!     Ed25519 would break stock-client interop. RSA-2048
//!     matches mde-kdc-proto's `PairingKeyPair` + the keygen
//!     module shipped in KDC2-3.1. Newer-wins-silently per
//!     `.claude/CLAUDE.md` §1: the spec text is updated in
//!     place if a future operator surfaces this.
//!   * **Hardcut migration.** Per the v2.1 lock, we do NOT read
//!     `~/.config/kdeconnect/`. Operators re-pair their phones
//!     once on upgrade. The CHANGELOG v2.1.0 calls this out.
//!   * **PEM wrapping** for `identity.pem` (vs raw `.pk8`)
//!     because the worklist spec says `.pem` and `openssl
//!     pkcs8 -in identity.pem -nocrypt` works directly.
//!
//! First-launch identity generation (KDC2-3.8) lands here via
//! `PairingStore::open_or_init` — if `identity.pem` is missing,
//! the keygen module produces fresh PKCS#8 DER, the loader
//! wraps it as PEM, and atomic-writes via `tempfile` + rename
//! semantics.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use mde_kdc_proto::crypto::PairingKeyPair;
use pkcs8::der::pem::LineEnding;
#[cfg(test)]
use pkcs8::der::Encode;
use pkcs8::{DecodePrivateKey, EncodePrivateKey};
use rsa::RsaPrivateKey;
use serde::{Deserialize, Serialize};

use crate::keygen;

/// Default identity filename — operator-readable PEM-wrapped
/// PKCS#8 RSA-2048 private key.
pub const IDENTITY_FILE: &str = "identity.pem";

/// Default device-table filename — TOML.
pub const DEVICES_FILE: &str = "devices.toml";

/// Errors the pairing store may surface.
#[derive(Debug)]
pub enum PairingError {
    /// I/O failed reading or writing a pairing file.
    Io(std::io::Error),
    /// TOML parse / serialize failed on devices.toml.
    Toml(String),
    /// PKCS#8 decode failed on identity.pem.
    Pkcs8(String),
    /// First-launch keygen failed (delegates to
    /// `keygen::KeygenError`).
    Keygen(keygen::KeygenError),
}

impl std::fmt::Display for PairingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PairingError::Io(e) => write!(f, "io: {e}"),
            PairingError::Toml(s) => write!(f, "toml: {s}"),
            PairingError::Pkcs8(s) => write!(f, "pkcs8: {s}"),
            PairingError::Keygen(e) => write!(f, "keygen: {e}"),
        }
    }
}

impl std::error::Error for PairingError {}

impl From<std::io::Error> for PairingError {
    fn from(e: std::io::Error) -> Self {
        PairingError::Io(e)
    }
}

/// One paired device in `devices.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairedDevice {
    /// Stable identifier (KDC device UUID).
    pub id: String,
    /// Human-readable name shown in the Workbench peer card.
    pub name: String,
    /// Device kind token — matches
    /// `mde_kdc_proto::discovery::DeviceType` serde rendering
    /// (`phone`, `tablet`, `desktop`, `unknown`).
    pub kind: String,
    /// SHA-256 fingerprint of the peer's public-key DER, hex
    /// uppercase with `:` separators every byte (matches the
    /// OpenSSH fingerprint convention upstream KDE Connect's
    /// settings dialog uses).
    pub fingerprint: String,
    /// PKCS#1 RSAPublicKey DER bytes, base64-encoded. The peer's
    /// signature-verification key for the handshake. Storing as
    /// base64 keeps the TOML file ASCII; decode via the helper
    /// `paired_public_key_der()` when feeding ring.
    pub public_key_b64: String,
    /// Plugin tokens the device's KDC announce listed under
    /// `incomingCapabilities` — used by the host to gate
    /// outgoing sends.
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Unix epoch seconds when pair completed.
    pub paired_at: i64,
    /// Unix epoch seconds of the most recent reachability
    /// observation. 0 when never seen since pair.
    #[serde(default)]
    pub last_seen_at: i64,
}

/// On-disk `devices.toml` shape.
#[derive(Debug, Default, Serialize, Deserialize)]
struct DevicesFile {
    #[serde(default, rename = "devices")]
    devices: Vec<PairedDevice>,
}

/// File-backed paired-device store + identity key holder.
///
/// `PairingStore::open_or_init(dir)` is the canonical entry
/// point — it loads existing state or creates a fresh identity
/// on first launch.
/// KDC2-3.5.a — interior-mutability refactor (2026-05-22).
/// `devices` lives behind a `std::sync::Mutex` so D-Bus
/// `Pair`/`Unpair`/`UpdateDevice` methods can mutate through
/// an `Arc<PairingStore>` without dropping back to a single
/// owner. Lock holds are short (a single in-memory map op
/// + a TOML serialize) — fine for std::sync::Mutex even
/// from inside an async task.
#[derive(Debug)]
pub struct PairingStore {
    config_dir: PathBuf,
    identity: PairingKeyPair,
    devices: Mutex<BTreeMap<String, PairedDevice>>,
}

impl PairingStore {
    /// Load (or initialize on first launch) the pairing store at
    /// `config_dir`. The directory + parents are created if
    /// missing.
    ///
    /// First-launch (KDC2-3.8):
    ///   1. `config_dir` is mkdir-p'd.
    ///   2. If `identity.pem` doesn't exist, fresh RSA-2048
    ///      keypair is generated via `crate::keygen` and
    ///      atomically written.
    ///   3. If `devices.toml` doesn't exist, an empty store is
    ///      created.
    pub fn open_or_init(config_dir: impl Into<PathBuf>) -> Result<Self, PairingError> {
        let config_dir = config_dir.into();
        std::fs::create_dir_all(&config_dir)?;
        let identity = Self::load_or_create_identity(&config_dir)?;
        let devices = Self::load_devices(&config_dir.join(DEVICES_FILE))?;
        Ok(Self {
            config_dir,
            identity,
            devices: Mutex::new(devices),
        })
    }

    /// Construct from already-loaded pieces — used by tests +
    /// any future caller that wants to inject a synthesized
    /// identity (e.g. CI with a fixed test key).
    #[must_use]
    pub fn from_parts(
        config_dir: PathBuf,
        identity: PairingKeyPair,
        devices: Vec<PairedDevice>,
    ) -> Self {
        let devices: BTreeMap<String, PairedDevice> =
            devices.into_iter().map(|d| (d.id.clone(), d)).collect();
        Self {
            config_dir,
            identity,
            devices: Mutex::new(devices),
        }
    }

    /// The store's RSA pairing keypair. Used by the host
    /// integration's handshake code (KDC2-3.2 follow-up).
    #[must_use]
    pub fn identity(&self) -> &PairingKeyPair {
        &self.identity
    }

    /// UC-4 — stable host-side `deviceId` for the KDC wire
    /// protocol's identity packet + cert CN. Derived
    /// deterministically from the pkcs8 keypair bytes
    /// (SHA-256, truncated to 16 bytes, lower-case hex) so it
    /// stays stable across restarts without a separate
    /// persisted file. Mirrors stock KDE Connect's 32-char
    /// hex `deviceId` shape.
    ///
    /// Stable invariant: regenerating the identity (new
    /// keypair) yields a new host id, which the wire protocol
    /// expects (a new keypair is observationally a new device
    /// from the peer's point of view).
    #[must_use]
    pub fn host_id(&self) -> String {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(self.identity.pkcs8_bytes());
        digest[..16].iter().fold(String::with_capacity(32), |mut s, b| {
            use std::fmt::Write;
            let _ = write!(s, "{b:02x}");
            s
        })
    }

    /// Total number of paired devices. Cheap.
    #[must_use]
    pub fn paired_count(&self) -> usize {
        self.devices
            .lock()
            .expect("pairing-store mutex poisoned")
            .len()
    }

    /// Look up a paired device by id. Returns a clone — the
    /// caller can't hold a reference into the Mutex.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<PairedDevice> {
        self.devices
            .lock()
            .expect("pairing-store mutex poisoned")
            .get(id)
            .cloned()
    }

    /// Insert (or replace) a paired device + persist.
    pub fn upsert(&self, device: PairedDevice) -> Result<(), PairingError> {
        {
            let mut guard = self.devices.lock().expect("pairing-store mutex poisoned");
            guard.insert(device.id.clone(), device);
        }
        self.persist_devices()
    }

    /// Forget a paired device. No-op if id is unknown. Persists.
    /// Returns `Ok(true)` when a device was actually removed,
    /// `Ok(false)` for the unknown-id no-op — D-Bus callers map
    /// the latter to `NoSuchDevice`.
    pub fn forget(&self, id: &str) -> Result<bool, PairingError> {
        let removed = {
            let mut guard = self.devices.lock().expect("pairing-store mutex poisoned");
            guard.remove(id).is_some()
        };
        if removed {
            self.persist_devices()?;
        }
        Ok(removed)
    }

    /// Snapshot every paired device (ordered by id). Returns a
    /// cloned `Vec` because the lock can't outlive this call.
    #[must_use]
    pub fn list(&self) -> Vec<PairedDevice> {
        self.devices
            .lock()
            .expect("pairing-store mutex poisoned")
            .values()
            .cloned()
            .collect()
    }

    /// UC-2 — Look up a paired device by its cert fingerprint
    /// (the upper-case-hex SHA-256 of the cert DER, with `:`
    /// separators, exactly the format `tls::compute_fingerprint`
    /// produces).
    ///
    /// The inbound TLS accept path (UC-3) uses this to map an
    /// accepted client's leaf cert back to its `PairedDevice`
    /// record. Comparison is case-insensitive on the hex bytes —
    /// the store canonically stores upper-case, but defensive
    /// folding lets a future caller pass a value derived from a
    /// different formatter without silently mismatching.
    ///
    /// Returns the first match if multiple devices share the
    /// same fingerprint (which shouldn't happen — each device
    /// generates its own keypair — but defending the lookup
    /// against duplicates costs one extra comparison).
    #[must_use]
    pub fn find_by_fingerprint(&self, fingerprint: &str) -> Option<PairedDevice> {
        let target = fingerprint.to_ascii_uppercase();
        self.devices
            .lock()
            .expect("pairing-store mutex poisoned")
            .values()
            .find(|d| d.fingerprint.to_ascii_uppercase() == target)
            .cloned()
    }

    /// Where the store lives on disk. Used by tests + diagnostics.
    #[must_use]
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    // ──────────────────────────────────────────────────────────
    // Internal helpers
    // ──────────────────────────────────────────────────────────

    fn load_or_create_identity(dir: &Path) -> Result<PairingKeyPair, PairingError> {
        let path = dir.join(IDENTITY_FILE);
        if path.exists() {
            let raw = std::fs::read_to_string(&path)?;
            // RsaPrivateKey::from_pkcs8_pem returns the public-
            // key-bearing struct; convert to PKCS#8 DER + load
            // into ring via PairingKeyPair.
            let key = RsaPrivateKey::from_pkcs8_pem(&raw)
                .map_err(|e| PairingError::Pkcs8(format!("read identity: {e}")))?;
            let der = key
                .to_pkcs8_der()
                .map_err(|e| PairingError::Pkcs8(format!("re-encode pkcs8: {e}")))?;
            PairingKeyPair::from_pkcs8(der.as_bytes())
                .map_err(|e| PairingError::Pkcs8(format!("ring load: {e}")))
        } else {
            // First-launch: generate + persist.
            let der = keygen::generate_pkcs8().map_err(PairingError::Keygen)?;
            let key = RsaPrivateKey::from_pkcs8_der(&der)
                .map_err(|e| PairingError::Pkcs8(format!("decode generated: {e}")))?;
            let pem = key
                .to_pkcs8_pem(LineEnding::LF)
                .map_err(|e| PairingError::Pkcs8(format!("encode pem: {e}")))?;
            atomic_write(&path, pem.as_bytes())?;
            // Restrict permissions on the private key file — 0600.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&path)?.permissions();
                perms.set_mode(0o600);
                std::fs::set_permissions(&path, perms)?;
            }
            PairingKeyPair::from_pkcs8(&der)
                .map_err(|e| PairingError::Pkcs8(format!("ring load: {e}")))
        }
    }

    fn load_devices(path: &Path) -> Result<BTreeMap<String, PairedDevice>, PairingError> {
        match std::fs::read_to_string(path) {
            Ok(raw) => {
                let file: DevicesFile =
                    toml::from_str(&raw).map_err(|e| PairingError::Toml(format!("{e}")))?;
                Ok(file
                    .devices
                    .into_iter()
                    .map(|d| (d.id.clone(), d))
                    .collect())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
            Err(e) => Err(PairingError::Io(e)),
        }
    }

    fn persist_devices(&self) -> Result<(), PairingError> {
        let file = {
            let guard = self.devices.lock().expect("pairing-store mutex poisoned");
            DevicesFile {
                devices: guard.values().cloned().collect(),
            }
        };
        let raw = toml::to_string_pretty(&file)
            .map_err(|e| PairingError::Toml(format!("serialize: {e}")))?;
        let path = self.config_dir.join(DEVICES_FILE);
        atomic_write(&path, raw.as_bytes())?;
        Ok(())
    }
}

/// Atomic write via temp-file + rename. Crashes mid-write don't
/// leave a half-written devices.toml or identity.pem.
fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no parent dir"))?;
    let mut tmp = tempfile_path(dir, path);
    std::fs::write(&tmp, bytes)?;
    // Rename is atomic on POSIX (when src + dst on same FS).
    std::fs::rename(&tmp, path).inspect_err(|_| {
        // Best-effort cleanup if the rename fails.
        let _ = std::fs::remove_file(&tmp);
        tmp.clear();
    })?;
    Ok(())
}

/// Build a sibling temp-path. Reuses the parent + a `.tmp`
/// suffix derived from the target name + pid so concurrent
/// writes on the same target don't collide.
fn tempfile_path(dir: &Path, target: &Path) -> PathBuf {
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "tmp".to_string());
    dir.join(format!(".{name}.tmp.{}", std::process::id()))
}

trait PathBufExt {
    fn clear(&mut self);
}
impl PathBufExt for PathBuf {
    fn clear(&mut self) {
        *self = PathBuf::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn open_or_init_creates_fresh_identity_on_first_launch() {
        let tmp = tempdir().unwrap();
        let store = PairingStore::open_or_init(tmp.path()).unwrap();
        // identity.pem must exist post-init.
        assert!(tmp.path().join(IDENTITY_FILE).exists());
        // devices.toml is NOT created until the first upsert.
        assert!(!tmp.path().join(DEVICES_FILE).exists());
        assert_eq!(store.paired_count(), 0);
    }

    #[test]
    fn open_or_init_is_idempotent_across_calls() {
        let tmp = tempdir().unwrap();
        let _ = PairingStore::open_or_init(tmp.path()).unwrap();
        // Capture the identity PEM bytes; second open must NOT
        // regenerate (we'd lose the keypair on every restart).
        let identity_before = std::fs::read_to_string(tmp.path().join(IDENTITY_FILE)).unwrap();
        let _ = PairingStore::open_or_init(tmp.path()).unwrap();
        let identity_after = std::fs::read_to_string(tmp.path().join(IDENTITY_FILE)).unwrap();
        assert_eq!(
            identity_before, identity_after,
            "second open must NOT regenerate identity",
        );
    }

    #[test]
    fn identity_file_has_restrictive_permissions() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let tmp = tempdir().unwrap();
            let _ = PairingStore::open_or_init(tmp.path()).unwrap();
            let mode = std::fs::metadata(tmp.path().join(IDENTITY_FILE))
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600, "identity.pem must be 0600; got {mode:o}");
        }
    }

    #[test]
    fn upsert_persists_a_device_to_disk() {
        let tmp = tempdir().unwrap();
        let store = PairingStore::open_or_init(tmp.path()).unwrap();
        let device = PairedDevice {
            id: "abc-123".into(),
            name: "Pixel 8".into(),
            kind: "phone".into(),
            fingerprint: "AB:CD:EF".into(),
            public_key_b64: "dGVzdGtleQ==".into(),
            capabilities: vec!["kdeconnect.clipboard".into()],
            paired_at: 1_700_000_000,
            last_seen_at: 0,
        };
        store.upsert(device.clone()).unwrap();
        assert_eq!(store.paired_count(), 1);
        // devices.toml now exists on disk.
        let raw = std::fs::read_to_string(tmp.path().join(DEVICES_FILE)).unwrap();
        assert!(raw.contains("abc-123"));
        assert!(raw.contains("Pixel 8"));

        // Re-open + confirm the device is loaded.
        drop(store);
        let store = PairingStore::open_or_init(tmp.path()).unwrap();
        assert_eq!(store.paired_count(), 1);
        assert_eq!(store.get("abc-123"), Some(device));
    }

    #[test]
    fn forget_removes_and_persists() {
        let tmp = tempdir().unwrap();
        let store = PairingStore::open_or_init(tmp.path()).unwrap();
        store
            .upsert(PairedDevice {
                id: "x".into(),
                name: "X".into(),
                kind: "desktop".into(),
                fingerprint: "00".into(),
                public_key_b64: "AA".into(),
                capabilities: vec![],
                paired_at: 0,
                last_seen_at: 0,
            })
            .unwrap();
        assert_eq!(store.paired_count(), 1);
        assert!(store.forget("x").unwrap());
        assert_eq!(store.paired_count(), 0);
        // Idempotent: forgetting an unknown id returns Ok(false).
        assert!(!store.forget("never-existed").unwrap());
        assert_eq!(store.paired_count(), 0);
    }

    #[test]
    fn list_returns_devices_in_id_order() {
        let tmp = tempdir().unwrap();
        let store = PairingStore::open_or_init(tmp.path()).unwrap();
        for id in ["c", "a", "b"] {
            store
                .upsert(PairedDevice {
                    id: id.into(),
                    name: id.into(),
                    kind: "phone".into(),
                    fingerprint: "00".into(),
                    public_key_b64: "AA".into(),
                    capabilities: vec![],
                    paired_at: 0,
                    last_seen_at: 0,
                })
                .unwrap();
        }
        let ids: Vec<String> = store.list().into_iter().map(|d| d.id).collect();
        assert_eq!(ids, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }

    #[test]
    fn upsert_through_shared_arc_works_with_immutable_ref() {
        // KDC2-3.5.a lock — the whole point of the refactor is
        // that an Arc<PairingStore> can mutate without an outer
        // Mutex<>. Prove it.
        use std::sync::Arc;
        let tmp = tempdir().unwrap();
        let store = Arc::new(PairingStore::open_or_init(tmp.path()).unwrap());
        let cloned = Arc::clone(&store);
        cloned
            .upsert(PairedDevice {
                id: "from-arc".into(),
                name: "n".into(),
                kind: "phone".into(),
                fingerprint: "FF".into(),
                public_key_b64: "AA".into(),
                capabilities: vec![],
                paired_at: 0,
                last_seen_at: 0,
            })
            .unwrap();
        assert_eq!(store.paired_count(), 1);
        assert!(store.get("from-arc").is_some());
    }

    // ─────────────────────────────────────────────────────────
    // UC-2 — find_by_fingerprint lookup for inbound TLS accept
    // ─────────────────────────────────────────────────────────

    fn make_paired(id: &str, fingerprint: &str) -> PairedDevice {
        PairedDevice {
            id: id.into(),
            name: id.into(),
            kind: "phone".into(),
            fingerprint: fingerprint.into(),
            public_key_b64: "AA==".into(),
            capabilities: vec!["kdeconnect.clipboard".into()],
            paired_at: 1_700_000_000,
            last_seen_at: 1_700_000_500,
        }
    }

    #[test]
    fn find_by_fingerprint_returns_paired_device_on_match() {
        let tmp = tempdir().unwrap();
        let store = PairingStore::open_or_init(tmp.path()).unwrap();
        store
            .upsert(make_paired("alice", "AB:CD:EF:01:23:45"))
            .unwrap();
        store
            .upsert(make_paired("bob", "11:22:33:44:55:66"))
            .unwrap();
        let hit = store
            .find_by_fingerprint("AB:CD:EF:01:23:45")
            .expect("alice's fingerprint must resolve");
        assert_eq!(hit.id, "alice");
    }

    #[test]
    fn find_by_fingerprint_returns_none_for_unknown() {
        let tmp = tempdir().unwrap();
        let store = PairingStore::open_or_init(tmp.path()).unwrap();
        store.upsert(make_paired("alice", "AB:CD:EF")).unwrap();
        assert!(store.find_by_fingerprint("ZZ:ZZ:ZZ").is_none());
        assert!(store.find_by_fingerprint("").is_none());
    }

    #[test]
    fn find_by_fingerprint_is_case_insensitive() {
        let tmp = tempdir().unwrap();
        let store = PairingStore::open_or_init(tmp.path()).unwrap();
        store.upsert(make_paired("alice", "AB:CD:EF")).unwrap();
        // Stored uppercase; lookup with lowercase must hit.
        let hit = store.find_by_fingerprint("ab:cd:ef").expect("case fold");
        assert_eq!(hit.id, "alice");
    }

    #[test]
    fn find_by_fingerprint_disambiguates_against_other_devices() {
        let tmp = tempdir().unwrap();
        let store = PairingStore::open_or_init(tmp.path()).unwrap();
        store.upsert(make_paired("alice", "AA:BB")).unwrap();
        store.upsert(make_paired("bob", "CC:DD")).unwrap();
        store.upsert(make_paired("carol", "EE:FF")).unwrap();
        assert_eq!(
            store.find_by_fingerprint("CC:DD").map(|d| d.id),
            Some("bob".to_string()),
        );
        assert_eq!(
            store.find_by_fingerprint("EE:FF").map(|d| d.id),
            Some("carol".to_string()),
        );
    }

    #[test]
    fn host_id_is_32_hex_chars_and_stable() {
        let tmp = tempdir().unwrap();
        let store = PairingStore::open_or_init(tmp.path()).unwrap();
        let id = store.host_id();
        assert_eq!(id.len(), 32, "host id must be 32 hex chars, got {id:?}");
        assert!(
            id.chars().all(|c| c.is_ascii_hexdigit()),
            "host id must be all hex chars",
        );
        // Stable across calls.
        assert_eq!(store.host_id(), id);
        // Stable across re-open with the same identity.pem.
        drop(store);
        let store2 = PairingStore::open_or_init(tmp.path()).unwrap();
        assert_eq!(store2.host_id(), id);
    }

    #[test]
    fn corrupt_devices_toml_surfaces_error() {
        let tmp = tempdir().unwrap();
        // Pre-seed a broken devices.toml so open_or_init must
        // parse it.
        std::fs::create_dir_all(tmp.path()).unwrap();
        std::fs::write(tmp.path().join(DEVICES_FILE), "not [ valid toml").unwrap();
        // Force identity to exist so open_or_init doesn't fail
        // on keygen (which is cheap but eats >0 entropy per call).
        let der = keygen::generate_pkcs8().unwrap();
        let key = RsaPrivateKey::from_pkcs8_der(&der).unwrap();
        let pem = key.to_pkcs8_pem(LineEnding::LF).unwrap();
        std::fs::write(tmp.path().join(IDENTITY_FILE), pem.as_bytes()).unwrap();

        let r = PairingStore::open_or_init(tmp.path());
        assert!(matches!(r, Err(PairingError::Toml(_))));
    }

    #[test]
    fn signature_round_trip_using_persisted_identity() {
        // Round-trip lock: the identity loaded from disk must
        // be able to sign + verify against itself. Forces the
        // RSA-2048-via-pem-via-DER-via-ring path to actually work.
        let tmp = tempdir().unwrap();
        let store = PairingStore::open_or_init(tmp.path()).unwrap();
        let sig = store
            .identity()
            .sign(b"pairing challenge")
            .expect("sign with loaded identity");
        // Extract the public key from the PEM we just wrote and
        // verify via mde-kdc-proto::crypto::verify_signature.
        let pem = std::fs::read_to_string(tmp.path().join(IDENTITY_FILE)).unwrap();
        let private = RsaPrivateKey::from_pkcs8_pem(&pem).unwrap();
        let public = private.to_public_key();
        // RSAPublicKey DER (PKCS#1) — ring's verifier expects
        // this form (not SubjectPublicKeyInfo).
        let pub_der = rsa::pkcs1::EncodeRsaPublicKey::to_pkcs1_der(&public)
            .expect("to_pkcs1_der")
            .to_der()
            .expect("der bytes");
        mde_kdc_proto::crypto::verify_signature(&pub_der, b"pairing challenge", &sig)
            .expect("verify against loaded identity");
    }
}
