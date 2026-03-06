//! Accessor methods for IdentityManager

use super::IdentityManager;
use crate::error::PlatformWalletError;
use crate::managed_identity::ManagedIdentity;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;
use indexmap::IndexMap;

impl IdentityManager {
    /// Get an identity by ID
    pub fn identity(&self, identity_id: &Identifier) -> Option<&Identity> {
        self.identities.get(identity_id).map(|m| &m.identity)
    }

    /// Get a mutable reference to an identity
    pub fn identity_mut(&mut self, identity_id: &Identifier) -> Option<&mut Identity> {
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

    /// Get all identities as a vector
    pub fn all_identities(&self) -> Vec<&Identity> {
        self.identities
            .values()
            .map(|managed| &managed.identity)
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
    pub fn managed_identity(&self, identity_id: &Identifier) -> Option<&ManagedIdentity> {
        self.identities.get(identity_id)
    }

    /// Get a mutable managed identity by ID
    pub fn managed_identity_mut(
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

    /// Get total credit balance across all identities
    pub fn total_credit_balance(&self) -> u64 {
        self.identities
            .values()
            .map(|managed| managed.identity.balance())
            .sum()
    }
}
