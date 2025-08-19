//! Platform wallet with identity management
//!
//! This crate provides a wallet implementation that combines traditional
//! wallet functionality with Dash Platform identity management.

use dashcore::Address as DashAddress;
use dashcore::Transaction;
use dpp::identity::Identity;
use dpp::prelude::Identifier;
use indexmap::IndexMap;
use key_wallet::account::managed_account_collection::ManagedAccountCollection;
use key_wallet::transaction_checking::account_checker::TransactionCheckResult;
use key_wallet::transaction_checking::{TransactionContext, WalletTransactionChecker};
use key_wallet::wallet::managed_wallet_info::fee::FeeLevel;
use key_wallet::wallet::managed_wallet_info::transaction_building::{
    AccountTypePreference, TransactionError,
};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::{ManagedWalletInfo, TransactionRecord};
use key_wallet::{Address, Network, Utxo, Wallet, WalletBalance};
pub mod identity_manager;
pub mod managed_identity;

pub use identity_manager::IdentityManager;
pub use managed_identity::ManagedIdentity;

#[cfg(feature = "manager")]
pub use key_wallet_manager;

/// Platform wallet information that extends ManagedWalletInfo with identity support
#[derive(Debug, Clone)]
pub struct PlatformWalletInfo {
    /// The underlying managed wallet info
    pub wallet_info: ManagedWalletInfo,

    /// Identity manager for handling Platform identities
    pub identity_manager: IdentityManager,
}

impl PlatformWalletInfo {
    /// Create a new platform wallet info
    pub fn new(wallet_id: [u8; 32], name: String) -> Self {
        Self {
            wallet_info: ManagedWalletInfo::with_name(wallet_id, name),
            identity_manager: IdentityManager::new(),
        }
    }

    /// Get all identities associated with this wallet
    pub fn identities(&self) -> IndexMap<Identifier, Identity> {
        self.identity_manager.identities()
    }

    /// Get direct access to managed identities
    pub fn managed_identities(&self) -> &IndexMap<Identifier, ManagedIdentity> {
        &self.identity_manager.identities
    }

    /// Add an identity to this wallet
    pub fn add_identity(&mut self, identity: Identity) -> Result<(), PlatformWalletError> {
        self.identity_manager.add_identity(identity)
    }

    /// Get a specific identity by ID
    pub fn get_identity(&self, identity_id: &Identifier) -> Option<&Identity> {
        self.identity_manager.get_identity(identity_id)
    }

    /// Remove an identity from this wallet
    pub fn remove_identity(
        &mut self,
        identity_id: &Identifier,
    ) -> Result<Identity, PlatformWalletError> {
        self.identity_manager.remove_identity(identity_id)
    }

    /// Get the primary identity (if set)
    pub fn primary_identity(&self) -> Option<&Identity> {
        self.identity_manager.primary_identity()
    }

    /// Set the primary identity
    pub fn set_primary_identity(
        &mut self,
        identity_id: Identifier,
    ) -> Result<(), PlatformWalletError> {
        self.identity_manager.set_primary_identity(identity_id)
    }
}

/// Implement WalletTransactionChecker by delegating to ManagedWalletInfo
impl WalletTransactionChecker for PlatformWalletInfo {
    fn check_transaction(
        &mut self,
        tx: &Transaction,
        network: Network,
        context: TransactionContext,
        update_state_if_found: bool,
    ) -> TransactionCheckResult {
        // Delegate to the underlying wallet info
        self.wallet_info
            .check_transaction(tx, network, context, update_state_if_found)
    }
}

/// Implement WalletInfoInterface for PlatformWalletInfo
impl WalletInfoInterface for PlatformWalletInfo {
    fn with_name(wallet_id: [u8; 32], name: String) -> Self {
        PlatformWalletInfo::new(wallet_id, name)
    }

    fn wallet_id(&self) -> [u8; 32] {
        self.wallet_info.wallet_id()
    }

    fn name(&self) -> Option<&str> {
        self.wallet_info.name()
    }

    fn set_name(&mut self, name: String) {
        self.wallet_info.set_name(name)
    }

    fn description(&self) -> Option<&str> {
        self.wallet_info.description()
    }

    fn set_description(&mut self, description: Option<String>) {
        if let Some(desc) = description {
            self.wallet_info.set_description(desc);
        } else {
            // Clear description by setting empty string
            self.wallet_info.description = None;
        }
    }

    fn birth_height(&self) -> Option<u32> {
        self.wallet_info.birth_height()
    }

    fn set_birth_height(&mut self, height: Option<u32>) {
        self.wallet_info.set_birth_height(height)
    }

    fn first_loaded_at(&self) -> u64 {
        self.wallet_info.first_loaded_at()
    }

    fn set_first_loaded_at(&mut self, timestamp: u64) {
        self.wallet_info.set_first_loaded_at(timestamp)
    }

    fn update_last_synced(&mut self, timestamp: u64) {
        self.wallet_info.update_last_synced(timestamp)
    }

    fn monitored_addresses(&self, network: Network) -> Vec<DashAddress> {
        self.wallet_info.monitored_addresses(network)
    }

    fn get_utxos(&self) -> Vec<Utxo> {
        self.wallet_info.get_utxos().into_iter().cloned().collect()
    }

    fn get_balance(&self) -> WalletBalance {
        self.wallet_info.get_balance()
    }

    fn update_balance(&mut self) {
        self.wallet_info.update_balance()
    }

    fn get_transaction_history(&self) -> Vec<&TransactionRecord> {
        self.wallet_info.get_transaction_history()
    }

    fn accounts_mut(&mut self, network: Network) -> Option<&mut ManagedAccountCollection> {
        self.wallet_info.accounts_mut(network)
    }

    fn accounts(&self, network: Network) -> Option<&ManagedAccountCollection> {
        self.wallet_info.accounts(network)
    }

    fn create_unsigned_payment_transaction(
        &mut self,
        wallet: &Wallet,
        network: Network,
        account_index: u32,
        account_type_pref: Option<AccountTypePreference>,
        recipients: Vec<(Address, u64)>,
        fee_level: FeeLevel,
        current_block_height: u32,
    ) -> Result<Transaction, TransactionError> {
        self.wallet_info.create_unsigned_payment_transaction(
            wallet,
            network,
            account_index,
            account_type_pref,
            recipients,
            fee_level,
            current_block_height,
        )
    }
}

/// Errors that can occur in platform wallet operations
#[derive(Debug, thiserror::Error)]
pub enum PlatformWalletError {
    #[error("Identity already exists: {0}")]
    IdentityAlreadyExists(Identifier),

    #[error("Identity not found: {0}")]
    IdentityNotFound(Identifier),

    #[error("No primary identity set")]
    NoPrimaryIdentity,

    #[error("Invalid identity data: {0}")]
    InvalidIdentityData(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_wallet_creation() {
        let wallet_id = [1u8; 32];
        let wallet = PlatformWalletInfo::new(wallet_id, "Test Platform Wallet".to_string());

        assert_eq!(wallet.wallet_id(), wallet_id);
        assert_eq!(wallet.name(), Some("Test Platform Wallet"));
        assert_eq!(wallet.identities().len(), 0);
    }
}
