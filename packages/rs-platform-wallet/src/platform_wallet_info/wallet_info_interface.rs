use crate::platform_wallet_info::PlatformWalletInfo;
use crate::IdentityManager;
use dashcore::{Address as DashAddress, Network, Transaction, Txid};
use dpp::prelude::CoreBlockHeight;
use key_wallet::account::{ManagedAccountCollection, TransactionRecord};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::ManagedWalletInfo;
use key_wallet::{Utxo, Wallet, WalletCoreBalance};
use std::collections::BTreeSet;

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

    fn synced_height(&self) -> CoreBlockHeight {
        self.wallet_info.synced_height()
    }

    fn monitored_addresses(&self) -> Vec<DashAddress> {
        self.wallet_info.monitored_addresses()
    }

    fn utxos(&self) -> BTreeSet<&Utxo> {
        self.wallet_info.utxos()
    }

    fn get_spendable_utxos(&self) -> BTreeSet<&Utxo> {
        self.wallet_info.get_spendable_utxos()
    }

    fn balance(&self) -> WalletCoreBalance {
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

    fn immature_transactions(&self) -> Vec<Transaction> {
        self.wallet_info.immature_transactions()
    }

    fn update_synced_height(&mut self, current_height: u32) {
        self.wallet_info.update_synced_height(current_height)
    }

    fn mark_instant_send_utxos(&mut self, txid: &Txid) -> bool {
        self.wallet_info.mark_instant_send_utxos(txid)
    }

    fn monitor_revision(&self) -> u64 {
        self.wallet_info.monitor_revision()
    }
}
