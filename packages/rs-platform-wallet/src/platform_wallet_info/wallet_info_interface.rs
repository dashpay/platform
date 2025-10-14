use std::collections::{BTreeSet, BTreeMap};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::{Utxo, Wallet, WalletBalance};
use key_wallet::wallet::ManagedWalletInfo;
use dashcore::{Address as DashAddress, Address, Network, Transaction};
use key_wallet::account::{ManagedAccountCollection, TransactionRecord};
use key_wallet::wallet::immature_transaction::{ImmatureTransaction, ImmatureTransactionCollection};
use key_wallet::wallet::managed_wallet_info::transaction_building::{AccountTypePreference, TransactionError};
use key_wallet::wallet::managed_wallet_info::fee::FeeLevel;
use crate::platform_wallet_info::PlatformWalletInfo;

/// Implement WalletInfoInterface for PlatformWalletInfo
impl WalletInfoInterface for PlatformWalletInfo {
    fn from_wallet(wallet: &Wallet) -> Self {
        Self {
            wallet_info: ManagedWalletInfo::from_wallet(wallet),
            identity_managers: BTreeMap::new(),
        }
    }

    fn from_wallet_with_name(wallet: &Wallet, name: String) -> Self {
        Self {
            wallet_info: ManagedWalletInfo::from_wallet_with_name(wallet, name),
            identity_managers: BTreeMap::new(),
        }
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

    fn accounts_mut(&mut self, network: Network) -> Option<&mut ManagedAccountCollection> {
        self.wallet_info.accounts_mut(network)
    }

    fn accounts(&self, network: Network) -> Option<&ManagedAccountCollection> {
        self.wallet_info.accounts(network)
    }

    fn process_matured_transactions(
        &mut self,
        network: Network,
        current_height: u32,
    ) -> Vec<ImmatureTransaction> {
        self.wallet_info
            .process_matured_transactions(network, current_height)
    }

    fn add_immature_transaction(&mut self, network: Network, tx: ImmatureTransaction) {
        self.wallet_info.add_immature_transaction(network, tx)
    }

    fn immature_transactions(&self, network: Network) -> Option<&ImmatureTransactionCollection> {
        self.wallet_info.immature_transactions(network)
    }

    fn network_immature_balance(&self, network: Network) -> u64 {
        self.wallet_info.network_immature_balance(network)
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