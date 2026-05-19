//! In-RAM [`SecretStore`] test double.
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

use std::collections::HashMap;
use std::sync::Mutex;

use super::error::SecretStoreError;
use super::secret::SecretBytes;
use super::store::SecretStore;
use super::validate::{validated_label, WalletId};

/// A `HashMap`-backed [`SecretStore`] for tests. No persistence, no
/// encryption.
#[derive(Default)]
pub struct MemoryStore {
    map: Mutex<HashMap<(WalletId, String), Vec<u8>>>,
}

impl MemoryStore {
    /// A fresh empty store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretStore for MemoryStore {
    fn put(&self, wallet_id: WalletId, label: &str, bytes: &[u8]) -> Result<(), SecretStoreError> {
        let label = validated_label(label)?;
        let mut map = self.map.lock().expect("MemoryStore mutex poisoned");
        map.insert((wallet_id, label.to_string()), bytes.to_vec());
        Ok(())
    }

    fn get(
        &self,
        wallet_id: WalletId,
        label: &str,
    ) -> Result<Option<SecretBytes>, SecretStoreError> {
        let label = validated_label(label)?;
        let map = self.map.lock().expect("MemoryStore mutex poisoned");
        Ok(map
            .get(&(wallet_id, label.to_string()))
            .map(|v| SecretBytes::from_slice(v)))
    }

    fn delete(&self, wallet_id: WalletId, label: &str) -> Result<(), SecretStoreError> {
        let label = validated_label(label)?;
        let mut map = self.map.lock().expect("MemoryStore mutex poisoned");
        map.remove(&(wallet_id, label.to_string()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wid(b: u8) -> WalletId {
        WalletId::from([b; 32])
    }

    #[test]
    fn roundtrip_and_overwrite() {
        let s = MemoryStore::new();
        assert!(s.get(wid(1), "bip39_mnemonic").unwrap().is_none());
        s.put(wid(1), "bip39_mnemonic", &[1, 2, 3]).unwrap();
        assert_eq!(
            s.get(wid(1), "bip39_mnemonic")
                .unwrap()
                .unwrap()
                .expose_secret(),
            &[1, 2, 3]
        );
        s.put(wid(1), "bip39_mnemonic", &[4, 5]).unwrap();
        assert_eq!(
            s.get(wid(1), "bip39_mnemonic")
                .unwrap()
                .unwrap()
                .expose_secret(),
            &[4, 5]
        );
    }

    #[test]
    fn idempotent_delete_and_namespacing() {
        let s = MemoryStore::new();
        s.put(wid(1), "seed", &[7]).unwrap();
        s.delete(wid(1), "seed").unwrap();
        s.delete(wid(1), "seed").unwrap(); // idempotent
        assert!(s.get(wid(1), "seed").unwrap().is_none());

        s.put(wid(1), "seed", &[1]).unwrap();
        s.put(wid(2), "seed", &[2]).unwrap();
        assert_eq!(
            s.get(wid(1), "seed").unwrap().unwrap().expose_secret(),
            &[1]
        );
        assert_eq!(
            s.get(wid(2), "seed").unwrap().unwrap().expose_secret(),
            &[2]
        );
    }

    #[test]
    fn rejects_invalid_label() {
        let s = MemoryStore::new();
        assert!(matches!(
            s.put(wid(1), "../escape", &[0]),
            Err(SecretStoreError::InvalidLabel)
        ));
        assert!(matches!(
            s.get(wid(1), ""),
            Err(SecretStoreError::InvalidLabel)
        ));
    }
}
