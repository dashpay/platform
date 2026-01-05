use crate::IdentityManager;
use key_wallet::wallet::ManagedWalletInfo;
use key_wallet::Network;
use std::fmt;

mod accessors;
mod contact_requests;
mod managed_account_operations;
mod matured_transactions;
mod wallet_info_interface;
mod wallet_transaction_checker;

/// Platform wallet information that extends ManagedWalletInfo with identity support
#[derive(Clone)]
pub struct PlatformWalletInfo {
    /// The underlying managed wallet info
    pub wallet_info: ManagedWalletInfo,

    /// Identity manager
    pub identity_manager: IdentityManager,
}

impl PlatformWalletInfo {
    /// Create a new platform wallet info for a specific network
    pub fn new(network: Network, wallet_id: [u8; 32], name: String) -> Self {
        Self {
            wallet_info: ManagedWalletInfo::with_name(network, wallet_id, name),
            identity_manager: IdentityManager::new(),
        }
    }

    /// Get or create an identity manager
    fn identity_manager_mut(&mut self) -> &mut IdentityManager {
        &mut self.identity_manager
    }

    /// Get an identity manager (if it exists)
    fn identity_manager(&self) -> &IdentityManager {
        &self.identity_manager
    }
}

impl fmt::Debug for PlatformWalletInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlatformWalletInfo")
            .field("wallet_info", &self.wallet_info)
            .field("identity_manager", &self.identity_manager)
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
        let wallet = PlatformWalletInfo::new(Network::Testnet, wallet_id, "Test Platform Wallet".to_string());

        assert_eq!(wallet.wallet_id(), wallet_id);
        assert_eq!(wallet.name(), Some("Test Platform Wallet"));
        assert_eq!(wallet.identities().len(), 0);
    }
}
