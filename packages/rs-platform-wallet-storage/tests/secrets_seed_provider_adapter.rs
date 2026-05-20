//! Item S — `CredentialStoreSeedProvider` adapter behaviour + the
//! skip-classification mapping (RT-S building block).
//!
//! Requires the `secrets` + `__secrets-test-helpers` features.

#![cfg(all(feature = "secrets", feature = "__secrets-test-helpers"))]

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use keyring_core::api::{Credential, CredentialApi, CredentialPersistence, CredentialStoreApi};
use keyring_core::{Entry, Error as KeyringError, Result as KeyringResult};
use platform_wallet::seed_provider::{
    SecretStoreErrorKind, SeedProvider, SeedUnavailable, WalletSecret,
};
use platform_wallet_storage::secrets::{
    CredentialStoreSeedProvider, FileStoreFailure, MemoryCredentialStore, WalletId, SERVICE_PREFIX,
};

/// Service string an adapter call would target for `wid` — used to
/// seed the in-RAM store under the same key the adapter resolves to.
fn service_for(wid: &WalletId) -> String {
    format!("{SERVICE_PREFIX}{}", wid.to_hex())
}

/// Put `bytes` under `(service_for(wid), label)` in `store`.
fn seed(
    store: &Arc<dyn CredentialStoreApi + Send + Sync>,
    wid: WalletId,
    label: &str,
    bytes: &[u8],
) {
    let entry = store.build(&service_for(&wid), label, None).unwrap();
    entry.set_secret(bytes).unwrap();
}

/// A `CredentialStoreApi` whose `build`-returned entries always fail
/// `get_secret` with a configured error — for the "store locked /
/// unavailable" skip sub-cases. Errors are cloned via a factory fn
/// because `KeyringError` is not `Clone`.
struct FailingCredentialStore {
    err_factory: fn() -> KeyringError,
}

impl FailingCredentialStore {
    fn new_arc(err_factory: fn() -> KeyringError) -> Arc<dyn CredentialStoreApi + Send + Sync> {
        Arc::new(Self { err_factory })
    }
}

impl std::fmt::Debug for FailingCredentialStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FailingCredentialStore")
            .finish_non_exhaustive()
    }
}

impl CredentialStoreApi for FailingCredentialStore {
    fn vendor(&self) -> String {
        "dash.platform-wallet-storage.test-failing".to_string()
    }
    fn id(&self) -> String {
        "failing-credential-store-v1".to_string()
    }
    fn build(
        &self,
        _service: &str,
        _user: &str,
        _modifiers: Option<&HashMap<&str, &str>>,
    ) -> KeyringResult<Entry> {
        Ok(Entry::new_with_credential(Arc::new(FailingCredential {
            err_factory: self.err_factory,
        })))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn persistence(&self) -> CredentialPersistence {
        CredentialPersistence::ProcessOnly
    }
}

struct FailingCredential {
    err_factory: fn() -> KeyringError,
}

impl std::fmt::Debug for FailingCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FailingCredential").finish_non_exhaustive()
    }
}

impl CredentialApi for FailingCredential {
    fn set_secret(&self, _: &[u8]) -> KeyringResult<()> {
        Err((self.err_factory)())
    }
    fn get_secret(&self) -> KeyringResult<Vec<u8>> {
        Err((self.err_factory)())
    }
    fn delete_credential(&self) -> KeyringResult<()> {
        Err((self.err_factory)())
    }
    fn get_credential(&self) -> KeyringResult<Option<Arc<Credential>>> {
        Ok(None)
    }
    fn get_specifiers(&self) -> Option<(String, String)> {
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[test]
fn mnemonic_preferred_over_seed() {
    let store: Arc<dyn CredentialStoreApi + Send + Sync> = MemoryCredentialStore::new_arc();
    let wid = WalletId::from([0xAA; 32]);
    seed(&store, wid, "mnemonic", b"abandon abandon abandon");
    seed(&store, wid, "seed", &[7u8; 64]);
    let provider = CredentialStoreSeedProvider::new(store);
    match provider.seed_for([0xAA; 32]).unwrap() {
        WalletSecret::Mnemonic(p) => assert_eq!(p.expose(), "abandon abandon abandon"),
        WalletSecret::Seed(_) => panic!("mnemonic must win when both exist"),
    }
}

#[test]
fn seed_used_when_no_mnemonic() {
    let store: Arc<dyn CredentialStoreApi + Send + Sync> = MemoryCredentialStore::new_arc();
    let wid = WalletId::from([0xBB; 32]);
    seed(&store, wid, "seed", &[3u8; 64]);
    let provider = CredentialStoreSeedProvider::new(store);
    match provider.seed_for([0xBB; 32]).unwrap() {
        WalletSecret::Seed(s) => assert_eq!(s.expose(), &[3u8; 64]),
        WalletSecret::Mnemonic(_) => panic!("expected seed"),
    }
}

#[test]
fn absent_maps_to_seed_absent() {
    let store: Arc<dyn CredentialStoreApi + Send + Sync> = MemoryCredentialStore::new_arc();
    let provider = CredentialStoreSeedProvider::new(store);
    let err = provider.seed_for([0xCC; 32]).unwrap_err();
    assert_eq!(err, SeedUnavailable::Absent);
}

#[test]
fn no_default_store_maps_to_backend_unavailable() {
    let provider = CredentialStoreSeedProvider::new(FailingCredentialStore::new_arc(|| {
        KeyringError::NoDefaultStore
    }));
    let err = provider.seed_for([0xDD; 32]).unwrap_err();
    assert_eq!(
        err,
        SeedUnavailable::StoreUnavailable(SecretStoreErrorKind::BackendUnavailable)
    );
}

#[test]
fn keyring_locked_maps_to_store_unavailable() {
    // A bare `NoStorageAccess` with no file-backend marker is the
    // "OS keyring locked" shape: maps to StoreUnavailable(KeyringLocked).
    let provider = CredentialStoreSeedProvider::new(FailingCredentialStore::new_arc(|| {
        KeyringError::NoStorageAccess(Box::new(std::io::Error::other("locked")))
    }));
    let err = provider.seed_for([0xDE; 32]).unwrap_err();
    assert_eq!(
        err,
        SeedUnavailable::StoreUnavailable(SecretStoreErrorKind::KeyringLocked)
    );
}

#[test]
fn wrong_passphrase_round_trips_to_store_unavailable() {
    let provider = CredentialStoreSeedProvider::new(FailingCredentialStore::new_arc(|| {
        KeyringError::NoStorageAccess(Box::new(FileStoreFailure::WrongPassphrase))
    }));
    let err = provider.seed_for([0xDF; 32]).unwrap_err();
    assert_eq!(
        err,
        SeedUnavailable::StoreUnavailable(SecretStoreErrorKind::WrongPassphrase)
    );
}

#[test]
fn decrypt_failure_maps_to_integrity_check() {
    let provider = CredentialStoreSeedProvider::new(FailingCredentialStore::new_arc(|| {
        KeyringError::BadStoreFormat(FileStoreFailure::Decrypt.to_string())
    }));
    let err = provider.seed_for([0xE0; 32]).unwrap_err();
    assert_eq!(
        err,
        SeedUnavailable::StoreError(SecretStoreErrorKind::IntegrityCheckFailed)
    );
}

#[test]
fn malformed_vault_maps_to_store_error() {
    let provider = CredentialStoreSeedProvider::new(FailingCredentialStore::new_arc(|| {
        KeyringError::BadStoreFormat(FileStoreFailure::MalformedVault.to_string())
    }));
    let err = provider.seed_for([0xE1; 32]).unwrap_err();
    assert_eq!(
        err,
        SeedUnavailable::StoreError(SecretStoreErrorKind::MalformedVault)
    );
}

#[test]
fn invalid_label_maps_to_invalid_label() {
    let provider = CredentialStoreSeedProvider::new(FailingCredentialStore::new_arc(|| {
        KeyringError::Invalid("user".to_string(), "label allowlist violation".to_string())
    }));
    let err = provider.seed_for([0xE2; 32]).unwrap_err();
    assert_eq!(
        err,
        SeedUnavailable::StoreError(SecretStoreErrorKind::InvalidLabel)
    );
}

/// No secret byte, label value, or stringified store source appears in
/// `SeedUnavailable`'s `Display`/`Debug` (RT-Z building block).
#[test]
fn skip_reason_renders_no_secret() {
    let store: Arc<dyn CredentialStoreApi + Send + Sync> = MemoryCredentialStore::new_arc();
    let wid = WalletId::from([0xEE; 32]);
    seed(&store, wid, "seed", b"SUPERSECRETSEEDBYTES");
    // Absent for a different id → SeedAbsent, no secret rendered.
    let provider = CredentialStoreSeedProvider::new(store);
    let err = provider.seed_for([0x00; 32]).unwrap_err();
    let rendered = format!("{err} {err:?}");
    assert!(
        !rendered.contains("SUPERSECRET"),
        "secret leaked: {rendered}"
    );
    assert_eq!(err, SeedUnavailable::Absent);
}
