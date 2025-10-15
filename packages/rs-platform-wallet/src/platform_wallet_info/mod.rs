use crate::IdentityManager;
use key_wallet::wallet::ManagedWalletInfo;
use key_wallet::Network;
use std::collections::BTreeMap;
use std::fmt;

mod accessors;
mod contact_requests;
mod managed_account_operations;
mod wallet_info_interface;
mod wallet_transaction_checker;

/// Platform wallet information that extends ManagedWalletInfo with identity support
#[derive(Clone)]
pub struct PlatformWalletInfo {
    /// The underlying managed wallet info
    pub wallet_info: ManagedWalletInfo,

    /// Identity managers for each network
    pub identity_managers: BTreeMap<Network, IdentityManager>,
}

impl PlatformWalletInfo {
    /// Create a new platform wallet info
    pub fn new(wallet_id: [u8; 32], name: String) -> Self {
        Self {
            wallet_info: ManagedWalletInfo::with_name(wallet_id, name),
            identity_managers: BTreeMap::new(),
        }
    }

    /// Get or create an identity manager for a specific network
    fn identity_manager_mut(&mut self, network: Network) -> &mut IdentityManager {
        self.identity_managers
            .entry(network)
            .or_insert_with(IdentityManager::new)
    }

    /// Get an identity manager for a specific network (if it exists)
    fn identity_manager(&self, network: Network) -> Option<&IdentityManager> {
        self.identity_managers.get(&network)
    }
}

impl fmt::Debug for PlatformWalletInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlatformWalletInfo")
            .field("wallet_info", &self.wallet_info)
            .field("identity_managers", &self.identity_managers)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::platform_wallet_info::PlatformWalletInfo;
    use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
    use key_wallet::Network;

    #[test]
    fn test_platform_wallet_creation() {
        let wallet_id = [1u8; 32];
        let wallet = PlatformWalletInfo::new(wallet_id, "Test Platform Wallet".to_string());

        assert_eq!(wallet.wallet_id(), wallet_id);
        assert_eq!(wallet.name(), Some("Test Platform Wallet"));
        assert_eq!(wallet.identities(Network::Testnet).len(), 0);
    }
}
