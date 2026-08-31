//! Trait implementations for `PlatformWalletInfo`.
//!
//! Implements [`WalletInfoInterface`], [`WalletTransactionChecker`], and
//! [`ManagedAccountOperations`] by delegating to the inner `ManagedWalletInfo`.

use std::collections::BTreeSet;

use async_trait::async_trait;
use dashcore::ephemerealdata::chain_lock::ChainLock;
use dashcore::ephemerealdata::instant_lock::InstantLock;
use dashcore::prelude::CoreBlockHeight;
use dashcore::{Address as DashAddress, ScriptBuf, Transaction, Txid};

use key_wallet::account::AccountType;
use key_wallet::bip32::ExtendedPubKey;
use key_wallet::managed_account::managed_account_collection::ManagedAccountCollection;
use key_wallet::transaction_checking::account_checker::TransactionCheckResult;
use key_wallet::transaction_checking::TransactionContext;
use key_wallet::transaction_checking::WalletTransactionChecker;
use key_wallet::wallet::managed_wallet_info::managed_account_operations::ManagedAccountOperations;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::{
    ApplyChainLockOutcome, WalletInfoInterface,
};
use key_wallet::wallet::managed_wallet_info::TransactionRecord;
use key_wallet::{Network, Utxo, Wallet, WalletCoreBalance};

use super::platform_wallet::PlatformWalletInfo;

// ---------------------------------------------------------------------------
// WalletInfoInterface — delegate to `self.core`
// ---------------------------------------------------------------------------

impl WalletInfoInterface for PlatformWalletInfo {
    fn from_wallet(wallet: &Wallet, birth_height: CoreBlockHeight) -> Self {
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

        let inner = ManagedWalletInfo::from_wallet(wallet, birth_height);
        Self {
            core_wallet: inner,
            generation: std::sync::Arc::new(super::core::WalletGeneration::new()),
            identity_manager: super::identity::IdentityManager::new(),
            tracked_asset_locks: std::collections::BTreeMap::new(),
            observed_input_conflicts: Default::default(),
            dpns_name_states: std::collections::BTreeMap::new(),
        }
    }

    fn from_wallet_with_name(wallet: &Wallet, name: String, birth_height: CoreBlockHeight) -> Self {
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

        let inner = ManagedWalletInfo::from_wallet_with_name(wallet, name, birth_height);
        Self {
            core_wallet: inner,
            generation: std::sync::Arc::new(super::core::WalletGeneration::new()),
            identity_manager: super::identity::IdentityManager::new(),
            tracked_asset_locks: std::collections::BTreeMap::new(),
            observed_input_conflicts: Default::default(),
            dpns_name_states: std::collections::BTreeMap::new(),
        }
    }

    fn network(&self) -> Network {
        self.core_wallet.network()
    }

    fn wallet_id(&self) -> [u8; 32] {
        self.core_wallet.wallet_id()
    }

    fn name(&self) -> Option<&str> {
        self.core_wallet.name()
    }

    fn set_name(&mut self, name: String) {
        self.core_wallet.set_name(name);
    }

    fn description(&self) -> Option<&str> {
        self.core_wallet.description()
    }

    fn set_description(&mut self, description: Option<String>) {
        self.core_wallet.set_description(description);
    }

    fn birth_height(&self) -> CoreBlockHeight {
        self.core_wallet.birth_height()
    }

    // `first_loaded_at` / `set_first_loaded_at` were dropped from
    // `WalletInfoInterface` upstream and have no backing methods on
    // `ManagedWalletInfo` anymore. The field still exists on
    // `WalletMetadata` but is read/written directly there; the trait
    // surface no longer requires delegating accessors here.

    fn update_last_synced(&mut self, timestamp: u64) {
        self.core_wallet.update_last_synced(timestamp);
    }

    fn monitored_addresses(&self) -> Vec<DashAddress> {
        self.core_wallet.monitored_addresses()
    }

    fn monitored_script_pubkeys(&self) -> Vec<ScriptBuf> {
        self.core_wallet.monitored_script_pubkeys()
    }

    fn monitored_filter_elements(&self) -> Vec<Vec<u8>> {
        self.core_wallet.monitored_filter_elements()
    }

    fn utxos(&self) -> BTreeSet<&Utxo> {
        self.core_wallet.utxos()
    }

    fn get_spendable_utxos(&self) -> BTreeSet<&Utxo> {
        self.core_wallet.get_spendable_utxos()
    }

    fn balance(&self) -> WalletCoreBalance {
        self.core_wallet.balance()
    }

    fn update_balance(&mut self) {
        self.core_wallet.update_balance();
    }

    fn transaction_history(&self) -> Vec<&TransactionRecord> {
        self.core_wallet.transaction_history()
    }

    fn accounts_mut(&mut self) -> &mut ManagedAccountCollection {
        self.core_wallet.accounts_mut()
    }

    fn accounts(&self) -> &ManagedAccountCollection {
        self.core_wallet.accounts()
    }

    fn immature_transactions(&self) -> Vec<Transaction> {
        self.core_wallet.immature_transactions()
    }

    fn last_processed_height(&self) -> CoreBlockHeight {
        self.core_wallet.last_processed_height()
    }

    fn synced_height(&self) -> CoreBlockHeight {
        self.core_wallet.synced_height()
    }

    fn update_last_processed_height(&mut self, current_height: u32) {
        self.core_wallet
            .update_last_processed_height(current_height);
    }

    fn update_synced_height(&mut self, current_height: u32) {
        self.core_wallet.update_synced_height(current_height);
    }

    fn matured_coinbase_records(
        &self,
        old_height: CoreBlockHeight,
        new_height: CoreBlockHeight,
    ) -> Vec<TransactionRecord> {
        self.core_wallet
            .matured_coinbase_records(old_height, new_height)
    }

    fn mark_instant_send_utxos(&mut self, txid: &Txid, lock: &InstantLock) -> bool {
        self.core_wallet.mark_instant_send_utxos(txid, lock)
    }

    fn monitor_revision(&self) -> u64 {
        self.core_wallet.monitor_revision()
    }

    // Delegate the chain-lock methods to the inner `ManagedWalletInfo`.
    //
    // Without these delegations, `WalletInfoInterface`'s default impls
    // kick in (no-op `apply_chain_lock` returning an empty BTreeMap;
    // `last_applied_chain_lock` returning `None`). That's the bug behind
    // "stuck asset lock #10": upstream's
    // `spawn_chainlock_wallet_dispatch` task receives every validated
    // `ChainLockReceived` event and calls
    // `wallet.write().await.apply_chain_lock(...)`, but our
    // `PlatformWalletInfo` was hitting the trait default — promotion
    // never fired and `metadata.last_applied_chain_lock` stayed `None`.
    fn last_applied_chain_lock(&self) -> Option<&ChainLock> {
        self.core_wallet.last_applied_chain_lock()
    }

    fn apply_chain_lock(&mut self, chain_lock: ChainLock) -> ApplyChainLockOutcome {
        let cl_height = chain_lock.block_height;
        let outcome = self.core_wallet.apply_chain_lock(chain_lock);
        let total_promoted: usize = outcome.locked_transactions.values().map(|v| v.len()).sum();
        tracing::debug!(
            cl_height,
            total_promoted,
            accounts_with_promotions = outcome.locked_transactions.len(),
            metadata_advanced = outcome.metadata_advanced,
            "apply_chain_lock delegated"
        );
        outcome
    }
}

// ---------------------------------------------------------------------------
// WalletTransactionChecker — delegate to `self.core`
// ---------------------------------------------------------------------------

#[async_trait]
impl WalletTransactionChecker for PlatformWalletInfo {
    async fn check_core_transaction(
        &mut self,
        tx: &Transaction,
        context: TransactionContext,
        wallet: &mut Wallet,
        update_state: bool,
        update_balance: bool,
    ) -> TransactionCheckResult {
        // TODO: some logic must here - restore
        self.core_wallet
            .check_core_transaction(tx, context, wallet, update_state, update_balance)
            .await
    }
}

// ---------------------------------------------------------------------------
// ManagedAccountOperations — delegate to `self.core`
// ---------------------------------------------------------------------------

impl ManagedAccountOperations for PlatformWalletInfo {
    fn add_managed_account(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
    ) -> key_wallet::Result<()> {
        self.core_wallet.add_managed_account(wallet, account_type)
    }

    fn add_managed_account_from_xpub(
        &mut self,
        account_type: AccountType,
        account_xpub: ExtendedPubKey,
    ) -> key_wallet::Result<()> {
        self.core_wallet
            .add_managed_account_from_xpub(account_type, account_xpub)
    }

    #[cfg(feature = "bls")]
    fn add_managed_bls_account(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
    ) -> key_wallet::Result<()> {
        self.core_wallet
            .add_managed_bls_account(wallet, account_type)
    }

    #[cfg(feature = "bls")]
    fn add_managed_bls_account_from_public_key(
        &mut self,
        account_type: AccountType,
        bls_public_key: [u8; 48],
    ) -> key_wallet::Result<()> {
        self.core_wallet
            .add_managed_bls_account_from_public_key(account_type, bls_public_key)
    }

    #[cfg(feature = "eddsa")]
    fn add_managed_eddsa_account(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
    ) -> key_wallet::Result<()> {
        self.core_wallet
            .add_managed_eddsa_account(wallet, account_type)
    }

    #[cfg(feature = "eddsa")]
    fn add_managed_eddsa_account_from_public_key(
        &mut self,
        account_type: AccountType,
        ed25519_public_key: [u8; 32],
    ) -> key_wallet::Result<()> {
        self.core_wallet
            .add_managed_eddsa_account_from_public_key(account_type, ed25519_public_key)
    }
}

// ---------------------------------------------------------------------------
// Debug
// ---------------------------------------------------------------------------

impl std::fmt::Debug for PlatformWalletInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformWalletInfo")
            .field("wallet_id", &hex::encode(self.core_wallet.wallet_id()))
            .field("identity_count", &self.identity_manager.identity_count())
            .finish()
    }
}
