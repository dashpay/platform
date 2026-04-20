//! Construction & lifecycle for [`IdentityManager`].
//!
//! Adds, removes, and looks up the HD index for managed identities.
//! Every mutation here persists a corresponding
//! [`IdentityChangeSet`](crate::changeset::IdentityChangeSet) via
//! the supplied [`WalletPersister`].

use super::IdentityManager;
use crate::changeset::{IdentityChangeSet, IdentityEntry};
use crate::error::PlatformWalletError;
use crate::wallet::identity::managed_identity::ManagedIdentity;
use crate::wallet::persister::WalletPersister;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;

impl IdentityManager {
    /// Create a new identity manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an identity to the manager with its BIP-9 HD identity index.
    ///
    /// Every identity in this wallet must have its HD index so that signing
    /// and ECDH derivation can locate the correct keys.
    ///
    /// Persists the resulting changeset via `persister` and returns `()`.
    pub fn add_identity(
        &mut self,
        identity: Identity,
        identity_index: u32,
        persister: &WalletPersister,
    ) -> Result<(), PlatformWalletError> {
        let identity_id = identity.id();

        if self.identities.contains_key(&identity_id) {
            return Err(PlatformWalletError::IdentityAlreadyExists(identity_id));
        }

        let managed_identity = ManagedIdentity::new(identity, identity_index);
        let entry = IdentityEntry::from_managed(&managed_identity);
        self.identities.insert(identity_id, managed_identity);

        let mut cs = IdentityChangeSet::default();
        cs.identities.insert(identity_id, entry);

        // If this is the first identity, make it primary
        if self.identities.len() == 1 {
            self.primary_identity_id = Some(identity_id);
            cs.primary_identity = Some(identity_id);
        }

        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }

        Ok(())
    }

    /// Get the BIP-9 HD identity index for a given identity ID.
    ///
    /// Returns `None` if the identity is not managed or its index was not recorded.
    pub fn identity_index(&self, identity_id: &Identifier) -> Option<u32> {
        self.identities.get(identity_id).map(|m| m.identity_index)
    }

    /// Remove an identity from the manager.
    ///
    /// Persists the tombstone changeset via `persister` and returns the
    /// removed [`Identity`].
    ///
    /// Note: if the removed identity was the only one, the new primary
    /// selection in the persisted changeset remains `None`; the apply path
    /// re-derives the cleared state from the `removed` set alone.
    pub fn remove_identity(
        &mut self,
        identity_id: &Identifier,
        persister: &WalletPersister,
    ) -> Result<Identity, PlatformWalletError> {
        let managed_identity = self
            .identities
            .shift_remove(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        let mut cs = IdentityChangeSet::default();
        cs.removed.insert(*identity_id);

        if self.primary_identity_id == Some(*identity_id) {
            self.primary_identity_id = self.identities.keys().next().copied();
            if let Some(new_primary) = self.primary_identity_id {
                cs.primary_identity = Some(new_primary);
            }
        }

        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }

        Ok(managed_identity.identity)
    }
}
