//! Trait implementations for `PlatformWalletInfo`.
//!
//! Implements [`WalletInfoInterface`], [`WalletTransactionChecker`], and
//! [`ManagedAccountOperations`] by delegating to the inner
//! `ManagedWalletState<PlatformWalletPersisterBridge>`.

use std::collections::BTreeSet;

use async_trait::async_trait;
use dashcore::ephemerealdata::instant_lock::InstantLock;
use dashcore::prelude::CoreBlockHeight;
use dashcore::{Address as DashAddress, Transaction, Txid};

use key_wallet::account::AccountType;
use key_wallet::bip32::ExtendedPubKey;
use key_wallet::changeset::UtxoChangeSet;
use key_wallet::managed_account::managed_account_collection::ManagedAccountCollection;
use key_wallet::transaction_checking::account_checker::TransactionCheckResult;
use key_wallet::transaction_checking::TransactionContext;
use key_wallet::transaction_checking::WalletTransactionChecker;
use key_wallet::wallet::managed_wallet_info::managed_account_operations::ManagedAccountOperations;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::TransactionRecord;
use key_wallet::{Network, Utxo, Wallet, WalletCoreBalance};

use super::platform_wallet::PlatformWalletInfo;
// TODO: Move to state module
// ---------------------------------------------------------------------------
// WalletInfoInterface — delegate to `self.managed_state`
// ---------------------------------------------------------------------------

impl WalletInfoInterface for PlatformWalletInfo {
    fn from_wallet(wallet: &Wallet) -> Self {
        use super::persister::PlatformWalletPersisterBridge;
        use key_wallet_manager::ManagedWalletState;

        let inner = ManagedWalletState::<PlatformWalletPersisterBridge>::from_wallet(wallet);
        Self {
            managed_state: inner,
            balance: std::sync::Arc::new(super::core::WalletBalance::new()),
            identity_manager: super::identity::IdentityManager::new(),
            tracked_asset_locks: std::collections::BTreeMap::new(),
            platform_address_balances: std::collections::BTreeMap::new(),
            token_watched: std::collections::BTreeMap::new(),
            token_balances: std::collections::BTreeMap::new(),
        }
    }

    fn from_wallet_with_name(wallet: &Wallet, name: String) -> Self {
        use super::persister::PlatformWalletPersisterBridge;
        use key_wallet_manager::ManagedWalletState;

        let inner = ManagedWalletState::<PlatformWalletPersisterBridge>::from_wallet_with_name(
            wallet, name,
        );
        Self {
            managed_state: inner,
            balance: std::sync::Arc::new(super::core::WalletBalance::new()),
            identity_manager: super::identity::IdentityManager::new(),
            tracked_asset_locks: std::collections::BTreeMap::new(),
            platform_address_balances: std::collections::BTreeMap::new(),
            token_watched: std::collections::BTreeMap::new(),
            token_balances: std::collections::BTreeMap::new(),
        }
    }

    fn wallet(&self) -> &Wallet {
        self.managed_state.wallet()
    }

    fn wallet_mut(&mut self) -> &mut Wallet {
        self.managed_state.wallet_mut()
    }

    fn network(&self) -> Network {
        self.managed_state.network()
    }

    fn wallet_id(&self) -> [u8; 32] {
        self.managed_state.wallet_id()
    }

    fn name(&self) -> Option<&str> {
        self.managed_state.name()
    }

    fn set_name(&mut self, name: String) {
        self.managed_state.set_name(name);
    }

    fn description(&self) -> Option<&str> {
        self.managed_state.description()
    }

    fn set_description(&mut self, description: Option<String>) {
        self.managed_state.set_description(description);
    }

    fn birth_height(&self) -> CoreBlockHeight {
        self.managed_state.birth_height()
    }

    fn set_birth_height(&mut self, height: CoreBlockHeight) {
        self.managed_state.set_birth_height(height);
    }

    fn first_loaded_at(&self) -> u64 {
        self.managed_state.first_loaded_at()
    }

    fn set_first_loaded_at(&mut self, timestamp: u64) {
        self.managed_state.set_first_loaded_at(timestamp);
    }

    fn update_last_synced(&mut self, timestamp: u64) {
        self.managed_state.update_last_synced(timestamp);
    }

    fn monitored_addresses(&self) -> Vec<DashAddress> {
        self.managed_state.monitored_addresses()
    }

    fn utxos(&self) -> BTreeSet<&Utxo> {
        self.managed_state.utxos()
    }

    fn get_spendable_utxos(&self) -> BTreeSet<&Utxo> {
        self.managed_state.get_spendable_utxos()
    }

    fn balance(&self) -> WalletCoreBalance {
        self.managed_state.balance()
    }

    fn update_balance(&mut self) {
        self.managed_state.update_balance();
        // Also update the lock-free atomic balance.
        let bal = self.managed_state.balance();
        self.balance.update(&bal);
    }

    fn transaction_history(&self) -> Vec<&TransactionRecord> {
        self.managed_state.transaction_history()
    }

    fn accounts_mut(&mut self) -> &mut ManagedAccountCollection {
        self.managed_state.accounts_mut()
    }

    fn accounts(&self) -> &ManagedAccountCollection {
        self.managed_state.accounts()
    }

    fn immature_transactions(&self) -> Vec<Transaction> {
        self.managed_state.immature_transactions()
    }

    fn synced_height(&self) -> CoreBlockHeight {
        self.managed_state.synced_height()
    }
    // TODO: Why we have manual balance update here? These thibgs are not eit event what we use to update balance?
    fn update_synced_height(&mut self, current_height: u32) {
        self.managed_state.update_synced_height(current_height);
        let bal = self.managed_state.balance();
        self.balance.update(&bal);
    }

    fn mark_instant_send_utxos(&mut self, txid: &Txid, lock: &InstantLock) -> (bool, UtxoChangeSet) {
        let result = self.managed_state.mark_instant_send_utxos(txid, lock);
        if result.0 {
            // Balance changed — refresh atomics.
            let bal = self.managed_state.balance();
            self.balance.update(&bal);
        }
        result
    }

    fn monitor_revision(&self) -> u64 {
        self.managed_state.monitor_revision()
    }
}

// ---------------------------------------------------------------------------
// WalletTransactionChecker — delegate to `self.managed_state`
// ---------------------------------------------------------------------------

#[async_trait]
impl WalletTransactionChecker for PlatformWalletInfo {
    async fn check_core_transaction(
        &mut self,
        tx: &Transaction,
        context: TransactionContext,
        update_state: bool,
        update_balance: bool,
    ) -> TransactionCheckResult {
        let result = self
            .managed_state
            .check_core_transaction(tx, context, update_state, update_balance)
            .await;

        // If balance was updated, refresh the lock-free atomics.
        if update_balance && result.is_relevant {
            let bal = self.managed_state.balance();
            self.balance.update(&bal);
        }

        result
    }
}

// ---------------------------------------------------------------------------
// ManagedAccountOperations — delegate to `self.managed_state`
// ---------------------------------------------------------------------------

impl ManagedAccountOperations for PlatformWalletInfo {
    fn add_managed_account(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
    ) -> key_wallet::Result<()> {
        self.managed_state.add_managed_account(wallet, account_type)
    }

    fn add_managed_account_with_passphrase(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
        passphrase: &str,
    ) -> key_wallet::Result<()> {
        self.managed_state
            .add_managed_account_with_passphrase(wallet, account_type, passphrase)
    }

    fn add_managed_account_from_xpub(
        &mut self,
        account_type: AccountType,
        account_xpub: ExtendedPubKey,
    ) -> key_wallet::Result<()> {
        self.managed_state
            .add_managed_account_from_xpub(account_type, account_xpub)
    }

    #[cfg(feature = "bls")]
    fn add_managed_bls_account(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
    ) -> key_wallet::Result<()> {
        self.managed_state
            .add_managed_bls_account(wallet, account_type)
    }

    #[cfg(feature = "bls")]
    fn add_managed_bls_account_with_passphrase(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
        passphrase: &str,
    ) -> key_wallet::Result<()> {
        self.managed_state
            .add_managed_bls_account_with_passphrase(wallet, account_type, passphrase)
    }

    #[cfg(feature = "bls")]
    fn add_managed_bls_account_from_public_key(
        &mut self,
        account_type: AccountType,
        bls_public_key: [u8; 48],
    ) -> key_wallet::Result<()> {
        self.managed_state
            .add_managed_bls_account_from_public_key(account_type, bls_public_key)
    }

    #[cfg(feature = "eddsa")]
    fn add_managed_eddsa_account(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
    ) -> key_wallet::Result<()> {
        self.managed_state
            .add_managed_eddsa_account(wallet, account_type)
    }

    #[cfg(feature = "eddsa")]
    fn add_managed_eddsa_account_with_passphrase(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
        passphrase: &str,
    ) -> key_wallet::Result<()> {
        self.managed_state
            .add_managed_eddsa_account_with_passphrase(wallet, account_type, passphrase)
    }

    #[cfg(feature = "eddsa")]
    fn add_managed_eddsa_account_from_public_key(
        &mut self,
        account_type: AccountType,
        ed25519_public_key: [u8; 32],
    ) -> key_wallet::Result<()> {
        self.managed_state
            .add_managed_eddsa_account_from_public_key(account_type, ed25519_public_key)
    }
}

// ---------------------------------------------------------------------------
// Debug
// ---------------------------------------------------------------------------

impl std::fmt::Debug for PlatformWalletInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformWalletInfo")
            .field("wallet_id", &hex::encode(self.managed_state.wallet_id()))
            .field("identity_count", &self.identity_manager.identities.len())
            .finish()
    }
}
