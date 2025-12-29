use crate::error::PlatformWalletError;
use crate::platform_wallet_info::PlatformWalletInfo;
use crate::ManagedIdentity;
use dpp::identifier::Identifier;
use dpp::identity::Identity;
use indexmap::IndexMap;

impl PlatformWalletInfo {
    /// Get all identities associated with this wallet
    pub fn identities(&self) -> IndexMap<Identifier, Identity> {
        self.identity_manager().identities()
    }

    /// Get direct access to managed identities
    pub fn managed_identities(
        &self,
    ) -> &IndexMap<Identifier, ManagedIdentity> {
        &self.identity_manager().identities
    }

    /// Add an identity to this wallet
    pub fn add_identity(
        &mut self,
        identity: Identity,
    ) -> Result<(), PlatformWalletError> {
        self.identity_manager_mut().add_identity(identity)
    }

    /// Get a specific identity by ID
    pub fn identity(&self, identity_id: &Identifier) -> Option<&Identity> {
        self.identity_manager().identity(identity_id)
    }

    /// Remove an identity from this wallet
    pub fn remove_identity(
        &mut self,
        identity_id: &Identifier,
    ) -> Result<Identity, PlatformWalletError> {
        self.identity_manager_mut()
            .remove_identity(identity_id)
    }

    /// Get the primary identity (if set)
    pub fn primary_identity(&self) -> Option<&Identity> {
        self.identity_manager().primary_identity()
    }

    /// Set the primary identity
    pub fn set_primary_identity(
        &mut self,
        identity_id: Identifier,
    ) -> Result<(), PlatformWalletError> {
        self.identity_manager_mut()
            .set_primary_identity(identity_id)
    }
}
