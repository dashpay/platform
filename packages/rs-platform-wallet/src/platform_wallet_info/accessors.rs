use crate::error::PlatformWalletError;
use crate::platform_wallet_info::PlatformWalletInfo;
use crate::ManagedIdentity;
use dpp::identifier::Identifier;
use dpp::identity::Identity;
use indexmap::IndexMap;
use key_wallet::Network;

impl PlatformWalletInfo {
    /// Get all identities associated with this wallet for a specific network
    pub fn identities(&self, network: Network) -> IndexMap<Identifier, Identity> {
        self.identity_manager(network)
            .map(|manager| manager.identities())
            .unwrap_or_default()
    }

    /// Get direct access to managed identities for a specific network
    pub fn managed_identities(
        &self,
        network: Network,
    ) -> Option<&IndexMap<Identifier, ManagedIdentity>> {
        self.identity_manager(network)
            .map(|manager| &manager.identities)
    }

    /// Add an identity to this wallet for a specific network
    pub fn add_identity(
        &mut self,
        network: Network,
        identity: Identity,
    ) -> Result<(), PlatformWalletError> {
        self.identity_manager_mut(network).add_identity(identity)
    }

    /// Get a specific identity by ID for a specific network
    pub fn identity(&self, network: Network, identity_id: &Identifier) -> Option<&Identity> {
        self.identity_manager(network)
            .and_then(|manager| manager.identity(identity_id))
    }

    /// Remove an identity from this wallet for a specific network
    pub fn remove_identity(
        &mut self,
        network: Network,
        identity_id: &Identifier,
    ) -> Result<Identity, PlatformWalletError> {
        self.identity_manager_mut(network)
            .remove_identity(identity_id)
    }

    /// Get the primary identity for a specific network (if set)
    pub fn primary_identity(&self, network: Network) -> Option<&Identity> {
        self.identity_manager(network)
            .and_then(|manager| manager.primary_identity())
    }

    /// Set the primary identity for a specific network
    pub fn set_primary_identity(
        &mut self,
        network: Network,
        identity_id: Identifier,
    ) -> Result<(), PlatformWalletError> {
        self.identity_manager_mut(network)
            .set_primary_identity(identity_id)
    }
}
