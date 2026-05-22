//! KDC2-2 crypto trait surface — RSA-2048 pairing handshake +
//! AES-256-GCM session.
//!
//! Implementations land in KDC2-2.4. This file ships the **trait
//! shapes only** so the wire / discovery / plugins modules can
//! depend on them without forcing an early crypto-lib choice
//! (`ring` vs. `rust-crypto`; the v2.1 KDC2 lock keeps that open
//! until KDC2-2.4 explicitly surveys it).
//!
//! ## KeyStore is the seam for future post-quantum
//!
//! v2.1 explicitly omits post-quantum crypto per the KDC2 lock,
//! but the `KeyStore` trait below is where a future PQ adapter
//! will plug in — implementations expose key material as opaque
//! handles so a PQ algorithm swap doesn't touch wire/discovery/
//! plugins.

use std::fmt;
use std::sync::Mutex;

use ring::rand::{SecureRandom, SystemRandom};
use zeroize::Zeroize;

/// Opaque identifier for a key — used by the wire layer to
/// reference an active session key without exposing bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyHandle(pub u64);

impl fmt::Display for KeyHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "key#{:016x}", self.0)
    }
}

/// Errors any crypto operation may surface to the wire layer.
/// Stable variants here let the audit chain log a `family` token
/// (e.g. `"signature_invalid"`) without owning a giant flat enum.
#[derive(Debug)]
pub enum CryptoError {
    /// Pairing handshake signature failed to verify.
    SignatureInvalid,
    /// Session key not in the `KeyStore` (peer is no longer paired,
    /// or daemon was restarted without persisting the store).
    UnknownKey(KeyHandle),
    /// Encrypted body failed AEAD authentication — tampered or
    /// wrong key.
    AeadAuthFailed,
    /// Caller passed a key of the wrong algorithm (e.g. an AES key
    /// where an RSA key was expected).
    WrongAlgorithm,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoError::SignatureInvalid => write!(f, "signature_invalid"),
            CryptoError::UnknownKey(k) => write!(f, "unknown_key({k})"),
            CryptoError::AeadAuthFailed => write!(f, "aead_auth_failed"),
            CryptoError::WrongAlgorithm => write!(f, "wrong_algorithm"),
        }
    }
}

impl std::error::Error for CryptoError {}

/// Store for active session + identity keys. Implementations live
/// in `mde-kdc` (host integration); this crate uses the trait at
/// the wire layer's encrypt/decrypt boundary.
///
/// Object-safe so `mde-kdc` can hand a `Box<dyn KeyStore>` to the
/// wire decoder.
pub trait KeyStore: Send + Sync {
    /// Look up the session key bytes for a given handle.
    /// Implementations should clear the returned bytes on drop —
    /// callers MUST treat the slice as ephemeral and avoid copying
    /// it.
    ///
    /// Returns `None` when the handle is unknown (peer is not
    /// currently paired, or the key was rotated since the handle
    /// was issued).
    fn session_key(&self, handle: KeyHandle) -> Option<Vec<u8>>;

    /// Register a new session key after a successful pairing
    /// handshake. Returns the handle the wire layer uses going
    /// forward.
    fn install_session_key(&self, raw_key: &[u8]) -> KeyHandle;

    /// Forget a session key (peer unpaired, key rotation, etc.).
    /// Idempotent — calling with an unknown handle is a no-op.
    fn forget(&self, handle: KeyHandle);
}

// ────────────────────────────────────────────────────────────────
// KDC2-2.4a — in-memory RingKeyStore (ring 0.17 + zeroize)
// ────────────────────────────────────────────────────────────────

/// Entry in [`RingKeyStore::keys`]. Wraps the raw key bytes in a
/// `Vec<u8>` that's zeroed on drop via `zeroize`.
#[derive(Debug)]
struct StoredKey {
    handle: KeyHandle,
    bytes: Vec<u8>,
}

impl Drop for StoredKey {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

/// In-memory [`KeyStore`] impl backed by ring 0.17's `SystemRandom`
/// for handle-id generation. Holds session keys in a `Mutex<Vec<
/// StoredKey>>` so it's `Sync` (the host integration crosses an
/// async task boundary).
///
/// Persistence is **deliberately not in this crate** — `mde-kdc`
/// (KDC2-3) wraps a `RingKeyStore` with a file-backed
/// `~/.config/mde/connect/devices.toml` layer for cross-restart
/// pairing. Keeping this in-memory means the protocol-level tests
/// stay self-contained.
pub struct RingKeyStore {
    rng: SystemRandom,
    keys: Mutex<Vec<StoredKey>>,
}

impl RingKeyStore {
    /// New empty key store. Allocates a single `SystemRandom`
    /// which is cheap (it's just a phantom-data zero-sized type
    /// in ring 0.17).
    #[must_use]
    pub fn new() -> Self {
        Self {
            rng: SystemRandom::new(),
            keys: Mutex::new(Vec::new()),
        }
    }

    /// How many session keys are currently held. Exposed for
    /// instrumentation + tests.
    #[must_use]
    pub fn key_count(&self) -> usize {
        self.keys.lock().expect("RingKeyStore mutex poisoned").len()
    }
}

impl Default for RingKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for RingKeyStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Don't leak key counts or handles into the debug output —
        // audit chain readers + panic logs shouldn't see crypto
        // material indirectly.
        f.debug_struct("RingKeyStore").finish_non_exhaustive()
    }
}

impl KeyStore for RingKeyStore {
    fn session_key(&self, handle: KeyHandle) -> Option<Vec<u8>> {
        let keys = self.keys.lock().expect("RingKeyStore mutex poisoned");
        keys.iter()
            .find(|k| k.handle == handle)
            .map(|k| k.bytes.clone())
    }

    fn install_session_key(&self, raw_key: &[u8]) -> KeyHandle {
        // ring's SystemRandom-backed handle id keeps the key
        // referenceable without exposing the key bytes anywhere
        // they could be logged. 64-bit handle is enough to avoid
        // collisions across a typical mesh lifetime.
        let mut id_bytes = [0_u8; 8];
        self.rng
            .fill(&mut id_bytes)
            .expect("ring SystemRandom can always fill");
        let handle = KeyHandle(u64::from_be_bytes(id_bytes));

        let mut keys = self.keys.lock().expect("RingKeyStore mutex poisoned");
        keys.push(StoredKey {
            handle,
            bytes: raw_key.to_vec(),
        });
        handle
    }

    fn forget(&self, handle: KeyHandle) {
        let mut keys = self.keys.lock().expect("RingKeyStore mutex poisoned");
        // `retain` walks every entry; the dropped `StoredKey`s
        // zeroize via their `Drop` impl.
        keys.retain(|k| k.handle != handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_handle_display_is_stable_hex() {
        // The audit chain logs `key#<hex>` references; if the
        // formatting drifts the audit reader's regex breaks.
        let h = KeyHandle(0x1234);
        let s = format!("{h}");
        assert_eq!(s, "key#0000000000001234");
    }

    #[test]
    fn crypto_error_display_is_machine_token() {
        // Audit-log entries grep on the Display output. Each
        // variant must produce a stable single-token string.
        assert_eq!(format!("{}", CryptoError::SignatureInvalid), "signature_invalid");
        assert_eq!(format!("{}", CryptoError::AeadAuthFailed), "aead_auth_failed");
        assert_eq!(format!("{}", CryptoError::WrongAlgorithm), "wrong_algorithm");
        let s = format!("{}", CryptoError::UnknownKey(KeyHandle(1)));
        assert!(s.starts_with("unknown_key("));
    }

    #[test]
    fn key_handle_is_copy_and_hash() {
        // Used as a HashMap key in the wire layer's session
        // dispatch table — Copy + Hash + Eq must all be present.
        use std::collections::HashSet;
        let h = KeyHandle(7);
        let _copied = h; // doesn't move
        let mut set = HashSet::new();
        set.insert(h);
        assert!(set.contains(&KeyHandle(7)));
    }

    // ─────────────────────────────────────────────────────────────
    // KDC2-2.4a RingKeyStore — install / lookup / forget round trip
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn ring_key_store_starts_empty() {
        let store = RingKeyStore::new();
        assert_eq!(store.key_count(), 0);
    }

    #[test]
    fn install_session_key_returns_unique_handles() {
        // ring SystemRandom-backed handle ids must avoid collisions
        // across separate installs.
        let store = RingKeyStore::new();
        let h1 = store.install_session_key(&[1, 2, 3]);
        let h2 = store.install_session_key(&[4, 5, 6]);
        assert_ne!(h1, h2, "two installs must produce distinct handles");
        assert_eq!(store.key_count(), 2);
    }

    #[test]
    fn session_key_round_trips_through_handle() {
        let store = RingKeyStore::new();
        let raw = vec![0xde, 0xad, 0xbe, 0xef];
        let h = store.install_session_key(&raw);
        let back = store.session_key(h).expect("just-installed key resolves");
        assert_eq!(back, raw);
    }

    #[test]
    fn session_key_returns_none_for_unknown_handle() {
        let store = RingKeyStore::new();
        assert!(store.session_key(KeyHandle(0xdeadbeef)).is_none());
    }

    #[test]
    fn forget_removes_a_known_key() {
        let store = RingKeyStore::new();
        let h = store.install_session_key(&[7, 8, 9]);
        assert!(store.session_key(h).is_some());
        store.forget(h);
        assert!(store.session_key(h).is_none());
        assert_eq!(store.key_count(), 0);
    }

    #[test]
    fn forget_is_idempotent_for_unknown_handle() {
        // Idempotent — calling forget on a handle that was never
        // installed (or already forgotten) must not panic or
        // mutate the store.
        let store = RingKeyStore::new();
        store.forget(KeyHandle(42)); // never installed
        assert_eq!(store.key_count(), 0);
        let h = store.install_session_key(&[1]);
        store.forget(h);
        store.forget(h); // already gone — still no panic
        assert_eq!(store.key_count(), 0);
    }

    #[test]
    fn debug_impl_omits_key_count_and_handles() {
        // Audit chain + panic logs must NOT learn anything about
        // crypto state from a Debug-formatted RingKeyStore.
        let store = RingKeyStore::new();
        store.install_session_key(&[1, 2, 3]);
        store.install_session_key(&[4, 5, 6]);
        let dbg = format!("{store:?}");
        // No key count, no handle ids, no key bytes.
        assert!(!dbg.contains("2"), "Debug leaks key count: {dbg}");
        assert!(!dbg.contains("0x"), "Debug leaks handle hex: {dbg}");
    }

    #[test]
    fn store_is_object_safe_via_keystore_trait() {
        // The trait is the seam the wire layer dispatches through;
        // it must accept a Box<dyn KeyStore>.
        let store: Box<dyn KeyStore> = Box::new(RingKeyStore::new());
        let h = store.install_session_key(&[42]);
        assert_eq!(store.session_key(h), Some(vec![42]));
    }
}
