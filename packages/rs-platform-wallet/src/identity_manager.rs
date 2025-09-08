//! Identity management for platform wallets
//!
//! This module handles the storage and management of Dash Platform identities
//! associated with a wallet.

use crate::managed_identity::ManagedIdentity;
use crate::PlatformWalletError;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;
use indexmap::IndexMap;

/// Manages identities for a platform wallet
#[derive(Debug, Clone, Default)]
pub struct IdentityManager {
    /// All managed identities owned by this wallet, indexed by identity ID
    pub identities: IndexMap<Identifier, ManagedIdentity>,

    /// The primary identity ID (if set)
    pub primary_identity_id: Option<Identifier>,
}

impl IdentityManager {
    /// Create a new identity manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an identity to the manager
    pub fn add_identity(&mut self, identity: Identity) -> Result<(), PlatformWalletError> {
        let identity_id = identity.id();

        if self.identities.contains_key(&identity_id) {
            return Err(PlatformWalletError::IdentityAlreadyExists(identity_id));
        }

        // Create managed identity
        let managed_identity = ManagedIdentity::new(identity);

        // Add the managed identity
        self.identities.insert(identity_id, managed_identity);

        // If this is the first identity, make it primary
        if self.identities.len() == 1 {
            self.primary_identity_id = Some(identity_id);
        }

        Ok(())
    }

    /// Remove an identity from the manager
    pub fn remove_identity(
        &mut self,
        identity_id: &Identifier,
    ) -> Result<Identity, PlatformWalletError> {
        // Remove the managed identity
        let managed_identity = self
            .identities
            .shift_remove(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        // If this was the primary identity, clear it
        if self.primary_identity_id == Some(*identity_id) {
            self.primary_identity_id = None;

            // Optionally set the first remaining identity as primary
            if let Some(first_id) = self.identities.keys().next() {
                self.primary_identity_id = Some(*first_id);
            }
        }

        Ok(managed_identity.identity)
    }

    /// Get an identity by ID
    pub fn get_identity(&self, identity_id: &Identifier) -> Option<&Identity> {
        self.identities.get(identity_id).map(|m| &m.identity)
    }

    /// Get a mutable reference to an identity
    pub fn get_identity_mut(&mut self, identity_id: &Identifier) -> Option<&mut Identity> {
        self.identities
            .get_mut(identity_id)
            .map(|m| &mut m.identity)
    }

    /// Get all identities
    pub fn identities(&self) -> IndexMap<Identifier, Identity> {
        self.identities
            .iter()
            .map(|(id, managed)| (*id, managed.identity.clone()))
            .collect()
    }

    /// Get the primary identity
    pub fn primary_identity(&self) -> Option<&Identity> {
        self.primary_identity_id
            .as_ref()
            .and_then(|id| self.identities.get(id))
            .map(|m| &m.identity)
    }

    /// Set the primary identity
    pub fn set_primary_identity(
        &mut self,
        identity_id: Identifier,
    ) -> Result<(), PlatformWalletError> {
        if !self.identities.contains_key(&identity_id) {
            return Err(PlatformWalletError::IdentityNotFound(identity_id));
        }

        self.primary_identity_id = Some(identity_id);
        Ok(())
    }

    /// Get a managed identity by ID
    pub fn get_managed_identity(&self, identity_id: &Identifier) -> Option<&ManagedIdentity> {
        self.identities.get(identity_id)
    }

    /// Get a mutable managed identity by ID
    pub fn get_managed_identity_mut(
        &mut self,
        identity_id: &Identifier,
    ) -> Option<&mut ManagedIdentity> {
        self.identities.get_mut(identity_id)
    }

    /// Set a label for an identity
    pub fn set_label(
        &mut self,
        identity_id: &Identifier,
        label: String,
    ) -> Result<(), PlatformWalletError> {
        let managed = self
            .identities
            .get_mut(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        managed.set_label(label);
        Ok(())
    }

    /// Get all active identities
    pub fn active_identities(&self) -> Vec<&Identity> {
        self.identities
            .values()
            .filter(|managed| managed.is_active)
            .map(|managed| &managed.identity)
            .collect()
    }

    /// Get total credit balance across all identities
    pub fn total_credit_balance(&self) -> u64 {
        self.identities
            .values()
            .map(|managed| managed.identity.balance())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_identity(id: Identifier) -> Identity {
        use dpp::identity::v0::IdentityV0;
        use std::collections::BTreeMap;

        // Create a minimal test identity
        let identity_v0 = IdentityV0 {
            id,
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        };

        Identity::V0(identity_v0)
    }

    #[test]
    fn test_add_identity() {
        let mut manager = IdentityManager::new();
        let identity_id = Identifier::from([1u8; 32]);
        let identity = create_test_identity(identity_id);

        manager.add_identity(identity.clone()).unwrap();

        assert_eq!(manager.identities.len(), 1);
        assert!(manager.get_identity(&identity_id).is_some());
        assert_eq!(manager.primary_identity_id, Some(identity_id));
    }

    #[test]
    fn test_remove_identity() {
        let mut manager = IdentityManager::new();
        let identity_id = Identifier::from([1u8; 32]);
        let identity = create_test_identity(identity_id);

        manager.add_identity(identity).unwrap();
        let removed = manager.remove_identity(&identity_id).unwrap();

        assert_eq!(removed.id(), identity_id);
        assert_eq!(manager.identities.len(), 0);
        assert_eq!(manager.primary_identity_id, None);
    }

    #[test]
    fn test_primary_identity_switching() {
        let mut manager = IdentityManager::new();

        let id1 = Identifier::from([1u8; 32]);
        let id2 = Identifier::from([2u8; 32]);

        manager.add_identity(create_test_identity(id1)).unwrap();
        manager.add_identity(create_test_identity(id2)).unwrap();

        // First identity should be primary
        assert_eq!(manager.primary_identity_id, Some(id1));

        // Switch primary
        manager.set_primary_identity(id2).unwrap();
        assert_eq!(manager.primary_identity_id, Some(id2));
    }

    #[test]
    fn test_managed_identity() {
        let mut manager = IdentityManager::new();
        let identity_id = Identifier::from([1u8; 32]);

        manager
            .add_identity(create_test_identity(identity_id))
            .unwrap();

        // Update metadata
        manager
            .set_label(&identity_id, "My Identity".to_string())
            .unwrap();

        let managed = manager.get_managed_identity(&identity_id).unwrap();
        assert_eq!(managed.label, Some("My Identity".to_string()));
        assert_eq!(managed.is_active, true);
        assert_eq!(managed.last_sync_timestamp, None);
        assert_eq!(managed.last_sync_height, None);
        assert_eq!(managed.id(), identity_id);
    }
}
