use crate::platform_wallet_info::PlatformWalletInfo;
use dashcore::{Address as DashAddress, Address, Network, Transaction};
use key_wallet::account::{ManagedAccountCollection, TransactionRecord};
use key_wallet::wallet::immature_transaction::{
    ImmatureTransaction, ImmatureTransactionCollection,
};
use key_wallet::wallet::managed_wallet_info::fee::FeeLevel;
use key_wallet::wallet::managed_wallet_info::transaction_building::{
    AccountTypePreference, TransactionError,
};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::ManagedWalletInfo;
use key_wallet::{Utxo, Wallet, WalletBalance};
use std::collections::BTreeSet;
use dpp::prelude::CoreBlockHeight;
use crate::IdentityManager;

/// Implement WalletInfoInterface for PlatformWalletInfo
impl WalletInfoInterface for PlatformWalletInfo {
    fn from_wallet(wallet: &Wallet) -> Self {
        Self {
            wallet_info: ManagedWalletInfo::from_wallet(wallet),
            identity_manager: IdentityManager::new(),
        }
    }

    fn from_wallet_with_name(wallet: &Wallet, name: String) -> Self {
        Self {
            wallet_info: ManagedWalletInfo::from_wallet_with_name(wallet, name),
            identity_manager: IdentityManager::new(),
        }
    }

    fn network(&self) -> Network {
        self.wallet_info.network()
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
        self.wallet_info.set_description(description)
    }

    fn birth_height(&self) -> CoreBlockHeight {
        self.wallet_info.birth_height()
    }

    fn set_birth_height(&mut self, height: CoreBlockHeight) {
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

    fn monitored_addresses(&self) -> Vec<DashAddress> {
        self.wallet_info.monitored_addresses()
    }

    fn utxos(&self) -> BTreeSet<&Utxo> {
        self.wallet_info.utxos()
    }

    fn get_spendable_utxos(&self) -> BTreeSet<&Utxo> {
        // Use the default trait implementation which filters utxos
        self.utxos()
            .into_iter()
            .filter(|utxo| !utxo.is_locked && (utxo.is_confirmed || utxo.is_instantlocked))
            .collect()
    }

    fn balance(&self) -> WalletBalance {
        self.wallet_info.balance()
    }

    fn update_balance(&mut self) {
        self.wallet_info.update_balance()
    }

    fn transaction_history(&self) -> Vec<&TransactionRecord> {
        self.wallet_info.transaction_history()
    }

    fn accounts_mut(&mut self) -> &mut ManagedAccountCollection {
        self.wallet_info.accounts_mut()
    }

    fn accounts(&self) -> &ManagedAccountCollection {
        self.wallet_info.accounts()
    }

    fn process_matured_transactions(
        &mut self,
        current_height: u32,
    ) -> Vec<ImmatureTransaction> {
        self.wallet_info
            .process_matured_transactions(current_height)
    }

    fn add_immature_transaction(&mut self, tx: ImmatureTransaction) {
        // Delegate to the underlying wallet_info
        self.wallet_info.add_immature_transaction(tx)
    }

    fn immature_transactions(&self) -> &ImmatureTransactionCollection {
        self.wallet_info.immature_transactions()
    }

    fn immature_balance(&self) -> u64 {
        self.wallet_info.immature_balance()
    }

    fn create_unsigned_payment_transaction(
        &mut self,
        wallet: &Wallet,
        account_index: u32,
        account_type_pref: Option<AccountTypePreference>,
        recipients: Vec<(Address, u64)>,
        fee_level: FeeLevel,
        current_block_height: u32,
    ) -> Result<Transaction, TransactionError> {
        self.wallet_info.create_unsigned_payment_transaction(
            wallet,
            account_index,
            account_type_pref,
            recipients,
            fee_level,
            current_block_height,
        )
    }

    fn update_chain_height(&mut self, current_height: u32) {
        // Delegate to the underlying wallet_info
        self.wallet_info
            .update_chain_height(current_height)
    }
}
