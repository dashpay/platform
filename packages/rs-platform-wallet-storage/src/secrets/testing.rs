//! In-memory test fixtures for the secrets backends.
//!
//! [`InMemoryCredentialStore`] is a writable
//! [`CredentialStoreApi`](keyring_core::api::CredentialStoreApi) double for
//! the [`SecretStore::Os`](crate::secrets::SecretStore::Os) arm — the only
//! shipped credential stores need a live OS keychain / D-Bus session, so
//! the Os arm (where the L-1 strip residual "bites hardest", per the threat
//! model §8.3) is otherwise un-coverable in CI (GAP-005).
//!
//! Beyond the SPI, it exposes [`raw_overwrite`](InMemoryCredentialStore::raw_overwrite)
//! / [`raw_get`](InMemoryCredentialStore::raw_get) so a test can play the
//! **backend-write attacker**: replace a slot's stored bytes with an
//! arbitrary blob (e.g. a stripped scheme-0 envelope) the way a breached
//! keychain would, then assert the strict read still fails closed.
//!
//! Compiled under `cfg(test)` or the `__test-helpers` feature, never in a
//! production build.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use keyring_core::api::{Credential, CredentialApi, CredentialPersistence, CredentialStoreApi};
use keyring_core::{Entry, Error as KeyringError, Result as KeyringResult};

use super::validate::WalletId;
use super::SERVICE_PREFIX;

/// Shared `(service, user) -> bytes` map backing every credential a store
/// hands out, so writes through one entry are visible to another.
type Items = Arc<Mutex<HashMap<(String, String), Vec<u8>>>>;

/// An in-memory, writable [`CredentialStoreApi`] for Os-arm tests.
#[derive(Default)]
pub struct InMemoryCredentialStore {
    items: Items,
}

impl InMemoryCredentialStore {
    /// A fresh, empty store, ready to install as
    /// `SecretStore::Os(store.into_arc())`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Hand back an `Arc<dyn CredentialStoreApi + Send + Sync>` for the
    /// `SecretStore::Os` arm. Clones share the same backing map, so the
    /// returned trait object and `self` see each other's writes.
    pub fn as_dyn(&self) -> Arc<dyn CredentialStoreApi + Send + Sync> {
        Arc::new(Self {
            items: Arc::clone(&self.items),
        })
    }

    /// The service string `SecretStore` derives for `wallet_id`, so a test
    /// can address the exact slot the Os arm reads/writes.
    pub fn service_for(wallet_id: &WalletId) -> String {
        format!("{SERVICE_PREFIX}{}", wallet_id.to_hex())
    }

    /// **Attacker primitive:** overwrite the raw bytes at `(wallet_id,
    /// label)` with `bytes`, bypassing any envelope layer — exactly what a
    /// breached keychain write does.
    pub fn raw_overwrite(&self, wallet_id: &WalletId, label: &str, bytes: &[u8]) {
        self.lock().insert(
            (Self::service_for(wallet_id), label.to_string()),
            bytes.to_vec(),
        );
    }

    /// Read the raw stored bytes at `(wallet_id, label)`, if any.
    pub fn raw_get(&self, wallet_id: &WalletId, label: &str) -> Option<Vec<u8>> {
        self.lock()
            .get(&(Self::service_for(wallet_id), label.to_string()))
            .cloned()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<(String, String), Vec<u8>>> {
        self.items.lock().unwrap_or_else(|p| p.into_inner())
    }
}

impl CredentialStoreApi for InMemoryCredentialStore {
    fn vendor(&self) -> String {
        "dash.platform-wallet-storage/in-memory-test".to_string()
    }

    fn id(&self) -> String {
        "in-memory-credential-store".to_string()
    }

    fn build(
        &self,
        service: &str,
        user: &str,
        _modifiers: Option<&HashMap<&str, &str>>,
    ) -> KeyringResult<Entry> {
        let cred = InMemoryCredential {
            items: Arc::clone(&self.items),
            service: service.to_string(),
            user: user.to_string(),
        };
        Ok(Entry::new_with_credential(Arc::new(cred)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn persistence(&self) -> CredentialPersistence {
        CredentialPersistence::UntilDelete
    }
}

impl std::fmt::Debug for InMemoryCredentialStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never render stored bytes (they may be secret material).
        f.debug_struct("InMemoryCredentialStore")
            .field("entries", &self.lock().len())
            .finish()
    }
}

/// One `(service, user)` slot in an [`InMemoryCredentialStore`].
struct InMemoryCredential {
    items: Items,
    service: String,
    user: String,
}

impl InMemoryCredential {
    fn key(&self) -> (String, String) {
        (self.service.clone(), self.user.clone())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<(String, String), Vec<u8>>> {
        self.items.lock().unwrap_or_else(|p| p.into_inner())
    }
}

impl CredentialApi for InMemoryCredential {
    fn set_secret(&self, secret: &[u8]) -> KeyringResult<()> {
        self.lock().insert(self.key(), secret.to_vec());
        Ok(())
    }

    fn get_secret(&self) -> KeyringResult<Vec<u8>> {
        self.lock()
            .get(&self.key())
            .cloned()
            .ok_or(KeyringError::NoEntry)
    }

    fn delete_credential(&self) -> KeyringResult<()> {
        if self.lock().remove(&self.key()).is_some() {
            Ok(())
        } else {
            Err(KeyringError::NoEntry)
        }
    }

    fn get_credential(&self) -> KeyringResult<Option<Arc<Credential>>> {
        Ok(None)
    }

    fn get_specifiers(&self) -> Option<(String, String)> {
        Some(self.key())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for InMemoryCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryCredential")
            .field("service", &self.service)
            .field("user", &self.user)
            .finish_non_exhaustive()
    }
}
