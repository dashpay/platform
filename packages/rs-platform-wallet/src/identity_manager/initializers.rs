//! Identity lifecycle operations for IdentityManager

use super::IdentityManager;
use crate::managed_identity::ManagedIdentity;
use crate::error::PlatformWalletError;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;

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
}
