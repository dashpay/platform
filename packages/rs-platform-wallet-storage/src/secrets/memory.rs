//! In-RAM [`CredentialStoreApi`] test double.
//!
//! Gated behind `__secrets-test-helpers` (Cargo's "MUST NOT enable from
//! downstream" convention) so it is unreachable from production builds
//! and can never be a silent fallback for a failed real backend
//! (SEC-REQ-2.3.1). Values sit in [`SecretBytes`] so even test memory
//! is wiped and the type contract is exercised uniformly
//! (SEC-REQ-2.3.2).
//!
//! ## Threat coverage
//!
//! Covers **nothing at rest** — process RAM only, by design. Never use
//! outside tests.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use keyring_core::api::{Credential, CredentialApi, CredentialPersistence, CredentialStoreApi};
use keyring_core::{Entry, Error as KeyringError, Result as KeyringResult};

use super::secret::SecretBytes;
use super::validate::validated_label;

const VENDOR: &str = "dash.platform-wallet-storage.memory";
const STORE_ID: &str = "memory-credential-store-v1";

type StoreMap = HashMap<(String, String), SecretBytes>;

/// A `HashMap`-backed credential store for tests. No persistence, no
/// encryption. Stored values sit in [`SecretBytes`] so even test
/// memory zeroizes on drop (SEC-REQ-2.3.2).
#[derive(Default)]
pub struct MemoryCredentialStore {
    map: Arc<Mutex<StoreMap>>,
}

impl MemoryCredentialStore {
    /// A fresh empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Convenience constructor returning the store as an
    /// `Arc<dyn CredentialStoreApi + Send + Sync>` for installation as
    /// the keyring default or for handing to adapters.
    pub fn new_arc() -> Arc<dyn CredentialStoreApi + Send + Sync> {
        Arc::new(Self::new())
    }
}

impl std::fmt::Debug for MemoryCredentialStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryCredentialStore").finish_non_exhaustive()
    }
}

impl CredentialStoreApi for MemoryCredentialStore {
    fn vendor(&self) -> String {
        VENDOR.to_string()
    }

    fn id(&self) -> String {
        STORE_ID.to_string()
    }

    fn build(
        &self,
        service: &str,
        user: &str,
        _modifiers: Option<&HashMap<&str, &str>>,
    ) -> KeyringResult<Entry> {
        let label = validated_label(user).map_err(|_| {
            KeyringError::Invalid("user".to_string(), "label allowlist violation".to_string())
        })?;
        let cred = MemoryCredential {
            map: self.map.clone(),
            service: service.to_string(),
            user: label.to_string(),
        };
        Ok(Entry::new_with_credential(Arc::new(cred)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn persistence(&self) -> CredentialPersistence {
        CredentialPersistence::ProcessOnly
    }
}

/// One row in a [`MemoryCredentialStore`].
pub struct MemoryCredential {
    map: Arc<Mutex<StoreMap>>,
    service: String,
    user: String,
}

impl std::fmt::Debug for MemoryCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryCredential")
            .field("service", &self.service)
            .field("user", &self.user)
            .finish_non_exhaustive()
    }
}

impl CredentialApi for MemoryCredential {
    fn set_secret(&self, secret: &[u8]) -> KeyringResult<()> {
        let mut m = self.map.lock().expect("MemoryCredentialStore mutex poisoned");
        m.insert(
            (self.service.clone(), self.user.clone()),
            SecretBytes::from_slice(secret),
        );
        Ok(())
    }

    fn get_secret(&self) -> KeyringResult<Vec<u8>> {
        let m = self.map.lock().expect("MemoryCredentialStore mutex poisoned");
        match m.get(&(self.service.clone(), self.user.clone())) {
            Some(v) => Ok(v.expose_secret().to_vec()),
            None => Err(KeyringError::NoEntry),
        }
    }

    fn delete_credential(&self) -> KeyringResult<()> {
        let mut m = self.map.lock().expect("MemoryCredentialStore mutex poisoned");
        match m.remove(&(self.service.clone(), self.user.clone())) {
            Some(_) => Ok(()),
            None => Err(KeyringError::NoEntry),
        }
    }

    fn get_credential(&self) -> KeyringResult<Option<Arc<Credential>>> {
        Ok(None)
    }

    fn get_specifiers(&self) -> Option<(String, String)> {
        Some((self.service.clone(), self.user.clone()))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(s: &MemoryCredentialStore, service: &str, user: &str) -> Entry {
        s.build(service, user, None).expect("build")
    }

    #[test]
    fn roundtrip_and_overwrite() {
        let s = MemoryCredentialStore::new();
        let e = build(&s, "svc", "bip39_mnemonic");
        assert!(matches!(e.get_secret(), Err(KeyringError::NoEntry)));
        e.set_secret(&[1, 2, 3]).unwrap();
        assert_eq!(e.get_secret().unwrap(), vec![1, 2, 3]);
        e.set_secret(&[4, 5]).unwrap();
        assert_eq!(e.get_secret().unwrap(), vec![4, 5]);
    }

    #[test]
    fn delete_returns_no_entry_when_absent_and_after_delete() {
        let s = MemoryCredentialStore::new();
        let e = build(&s, "svc", "seed");
        assert!(matches!(e.delete_credential(), Err(KeyringError::NoEntry)));
        e.set_secret(&[7]).unwrap();
        e.delete_credential().unwrap();
        assert!(matches!(e.delete_credential(), Err(KeyringError::NoEntry)));
        assert!(matches!(e.get_secret(), Err(KeyringError::NoEntry)));
    }

    #[test]
    fn namespacing_across_service() {
        let s = MemoryCredentialStore::new();
        let a = build(&s, "svc-a", "seed");
        let b = build(&s, "svc-b", "seed");
        a.set_secret(&[1]).unwrap();
        b.set_secret(&[2]).unwrap();
        assert_eq!(a.get_secret().unwrap(), vec![1]);
        assert_eq!(b.get_secret().unwrap(), vec![2]);
    }

    // The map's value type must be a zeroize-on-drop wrapper, never a
    // bare `Vec<u8>` (SEC-REQ-2.3.2). The compile-time witness:
    const _: () = {
        assert!(std::mem::needs_drop::<SecretBytes>());
    };

    #[test]
    fn stored_value_is_zeroizing_wrapper() {
        let s = MemoryCredentialStore::new();
        build(&s, "svc", "seed").set_secret(&[0xAB; 32]).unwrap();
        let map = s.map.lock().unwrap();
        // This binding only compiles if the value type is `SecretBytes`.
        let v: &SecretBytes = map.get(&("svc".to_string(), "seed".to_string())).unwrap();
        assert_eq!(v.expose_secret(), &[0xAB; 32]);
    }

    #[test]
    fn rejects_invalid_label() {
        let s = MemoryCredentialStore::new();
        for bad in ["../escape", "", "a b"] {
            let err = s.build("svc", bad, None).unwrap_err();
            match err {
                KeyringError::Invalid(attr, _) => assert_eq!(attr, "user"),
                other => panic!("expected Invalid, got {other:?}"),
            }
        }
    }
}
