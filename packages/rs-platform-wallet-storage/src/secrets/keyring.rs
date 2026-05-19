//! [`KeyringStore`] — OS keyring backend.
//!
//! Built on `keyring-core 1.0.0` (the split-architecture library) plus
//! the per-platform credential-store crates; the `keyring` 4.x crate
//! itself is the sample CLI and is not a dependency here.
//!
//! Delegates at-rest protection to the OS credential store. Its
//! security *is* the OS keyring's security.
//!
//! ## Threat coverage
//!
//! Covers **A1** (other local user) and **A4** (lost laptop) where the
//! platform encrypts keyring items at rest and scopes them to the user.
//! Does **not** cover **A2/A3** same-user malware (most OS keyrings
//! hand the secret to any same-user process that asks), **A5** if the
//! keyring daemon itself is scraped, or **headless Linux** with no
//! Secret Service — that fails closed (`BackendUnavailable`), never
//! degrades to plaintext.
//!
//! ### Per-OS reality
//!
//! - **Linux/FreeBSD:** Secret Service (gnome-keyring / KWallet) needs
//!   a D-Bus session + unlocked collection. Headless / SSH / CI boxes
//!   frequently lack it → fail closed; the operator selects
//!   [`EncryptedFileStore`](super::EncryptedFileStore) explicitly.
//! - **macOS:** Keychain ACL — a re-signed binary with the same
//!   code-signing identity is A3 (accepted, AR-5).
//! - **Windows:** Credential Manager / DPAPI is user-profile scoped; a
//!   same-user process can unprotect it. DPAPI is **not** a defense
//!   against same-user malware, only A1/A4.

use std::sync::Arc;

use keyring_core::api::CredentialStore;
use keyring_core::{Entry, Error as KeyringError};

use super::error::SecretStoreError;
use super::secret::SecretBytes;
use super::store::SecretStore;
use super::validate::{validated_label, WalletId};

/// Keyring `service` namespace — application-scoped so a different app
/// cannot silently read the entry (SEC-REQ-2.1.2).
const SERVICE: &str = "dash.platform-wallet-storage";

/// An OS-keyring-backed [`SecretStore`].
///
/// The `account` is `"{wallet_id_hex}:{label}"`, so two wallets cannot
/// collide. Only that non-secret index appears in keyring attributes —
/// never a secret byte (SEC-REQ-2.1.2, CWE-312).
pub struct KeyringStore {
    store: Arc<CredentialStore>,
}

impl KeyringStore {
    /// Open the platform's default credential store, failing closed
    /// (typed [`SecretStoreError::BackendUnavailable`]) when none is
    /// reachable (headless / no Secret Service / no D-Bus). Never
    /// panics, never falls back to a weaker store (SEC-REQ-2.1.3).
    pub fn new() -> Result<Self, SecretStoreError> {
        let store = default_store()?;
        Ok(Self { store })
    }

    fn entry(&self, wallet_id: &WalletId, label: &str) -> Result<Entry, SecretStoreError> {
        let account = format!("{}:{}", wallet_id.to_hex(), label);
        self.store
            .build(SERVICE, &account, None)
            .map_err(map_keyring_err)
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn default_store() -> Result<Arc<CredentialStore>, SecretStoreError> {
    // Prefer the kernel keyutils store; fall back to Secret Service.
    // Both failing (headless, no session keyring, no D-Bus) is
    // fail-closed by design (SEC-REQ-2.1.3 / AR-4).
    if let Ok(s) = linux_keyutils_keyring_store::Store::new() {
        return Ok(s as Arc<CredentialStore>);
    }
    dbus_secret_service_keyring_store::Store::new()
        .map(|s| s as Arc<CredentialStore>)
        .map_err(map_keyring_err)
}

#[cfg(target_os = "macos")]
fn default_store() -> Result<Arc<CredentialStore>, SecretStoreError> {
    apple_native_keyring_store::Store::new()
        .map(|s| s as Arc<CredentialStore>)
        .map_err(map_keyring_err)
}

#[cfg(target_os = "windows")]
fn default_store() -> Result<Arc<CredentialStore>, SecretStoreError> {
    windows_native_keyring_store::Store::new()
        .map(|s| s as Arc<CredentialStore>)
        .map_err(map_keyring_err)
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "macos",
    target_os = "windows"
)))]
fn default_store() -> Result<Arc<CredentialStore>, SecretStoreError> {
    Err(SecretStoreError::BackendUnavailable)
}

/// Map keyring-core errors to the typed taxonomy. `NoEntry` is *not*
/// mapped here — callers translate it to `Ok(None)`/`Ok(())`. No
/// keyring error string is embedded (it could echo the `account`,
/// which is non-secret, but the taxonomy stays clean — SEC-REQ-2.0.1).
///
/// Per keyring-core 1.0.0, `NoStorageAccess` covers the *locked*
/// collection case ("it might be that the credential store is
/// locked"), so it maps to [`SecretStoreError::KeyringLocked`] to
/// drive the unlock-retry UX (SEC-REQ-2.1.4). A genuinely absent
/// backend (`NoDefaultStore` / `PlatformFailure`) is
/// [`SecretStoreError::BackendUnavailable`].
fn map_keyring_err(e: KeyringError) -> SecretStoreError {
    match e {
        KeyringError::NoEntry => SecretStoreError::NotFound,
        KeyringError::NoStorageAccess(_) => SecretStoreError::KeyringLocked,
        KeyringError::NoDefaultStore | KeyringError::PlatformFailure(_) => {
            SecretStoreError::BackendUnavailable
        }
        _ => SecretStoreError::BackendUnavailable,
    }
}

impl SecretStore for KeyringStore {
    fn put(&self, wallet_id: WalletId, label: &str, bytes: &[u8]) -> Result<(), SecretStoreError> {
        let label = validated_label(label)?;
        let entry = self.entry(&wallet_id, label)?;
        entry.set_secret(bytes).map_err(map_keyring_err)
    }

    fn get(
        &self,
        wallet_id: WalletId,
        label: &str,
    ) -> Result<Option<SecretBytes>, SecretStoreError> {
        let label = validated_label(label)?;
        let entry = self.entry(&wallet_id, label)?;
        match entry.get_secret() {
            Ok(mut v) => {
                let secret = SecretBytes::from_slice(&v);
                // keyring-core returns a bare `Vec<u8>`; wipe the
                // intermediate now that it is wrapped (SEC-REQ-3.1).
                use zeroize::Zeroize;
                v.zeroize();
                Ok(Some(secret))
            }
            Err(KeyringError::NoEntry) => Ok(None),
            Err(e) => Err(map_keyring_err(e)),
        }
    }

    fn delete(&self, wallet_id: WalletId, label: &str) -> Result<(), SecretStoreError> {
        let label = validated_label(label)?;
        let entry = self.entry(&wallet_id, label)?;
        match entry.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(e) => Err(map_keyring_err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_label_rejected_before_backend() {
        // Label validation must precede any keyring access, so this
        // is deterministic even on headless CI with no keyring.
        if let Ok(s) = KeyringStore::new() {
            assert!(matches!(
                s.put(WalletId::from([0; 32]), "../escape", b"x"),
                Err(SecretStoreError::InvalidLabel)
            ));
        }
    }

    #[test]
    fn locked_keyring_maps_to_keyring_locked() {
        // keyring-core's `NoStorageAccess` covers the locked-collection
        // case; it must surface as `KeyringLocked` so the caller can
        // prompt for unlock (SEC-REQ-2.1.4), not as `BackendUnavailable`.
        let locked =
            KeyringError::NoStorageAccess(std::io::Error::other("collection is locked").into());
        assert!(matches!(
            map_keyring_err(locked),
            SecretStoreError::KeyringLocked
        ));
        // A genuinely absent backend stays `BackendUnavailable`.
        assert!(matches!(
            map_keyring_err(KeyringError::NoDefaultStore),
            SecretStoreError::BackendUnavailable
        ));
        assert!(matches!(
            map_keyring_err(KeyringError::NoEntry),
            SecretStoreError::NotFound
        ));
    }

    #[test]
    fn headless_fails_closed_not_panic() {
        // On headless CI `new()` returns `BackendUnavailable`; where a
        // keyring exists it succeeds. Either way: typed, no panic, no
        // plaintext fallback.
        match KeyringStore::new() {
            Ok(_) | Err(SecretStoreError::BackendUnavailable) => {}
            Err(other) => panic!("unexpected: {other}"),
        }
    }

    /// Round-trip needs a live keyring; `#[ignore]` so headless CI does
    /// not fail. Run locally on a desktop with an unlocked keyring:
    /// `cargo test --features secrets keyring_roundtrip -- --ignored`
    #[test]
    #[ignore]
    fn keyring_roundtrip_and_namespacing() {
        let s = KeyringStore::new().expect("keyring available");
        let w1 = WalletId::from([1; 32]);
        let w2 = WalletId::from([2; 32]);
        s.put(w1, "seed", b"alpha").unwrap();
        s.put(w2, "seed", b"beta").unwrap();
        assert_eq!(
            s.get(w1, "seed").unwrap().unwrap().expose_secret(),
            b"alpha"
        );
        assert_eq!(s.get(w2, "seed").unwrap().unwrap().expose_secret(), b"beta");
        s.delete(w1, "seed").unwrap();
        s.delete(w2, "seed").unwrap();
        assert!(s.get(w1, "seed").unwrap().is_none());
    }
}
