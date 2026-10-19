//! Trait implementations for `PlatformWalletInfo`.
//!
//! Implements [`WalletInfoInterface`], [`WalletTransactionChecker`], and
//! [`ManagedAccountOperations`] by delegating to the inner `ManagedWalletInfo`.

use std::collections::{BTreeMap, BTreeSet};

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
    ApplyChainLockOutcome, Truncation, WalletInfoInterface,
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
            dashpay_backfill: Default::default(),
            rewind_barrier: Default::default(),
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
            dashpay_backfill: Default::default(),
            rewind_barrier: Default::default(),
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

    // `first_loaded_at` lives on `WalletMetadata` and is read/written
    // directly there; `WalletInfoInterface` has no accessors for it and
    // `ManagedWalletInfo` has no backing methods, so nothing is
    // delegated here.

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

    fn unrecorded_spend_heights(&self, tx: &Transaction) -> BTreeSet<u32> {
        self.core_wallet.unrecorded_spend_heights(tx)
    }

    fn immature_transactions(&self) -> Vec<Transaction> {
        self.core_wallet.immature_transactions()
    }

    fn last_processed_height(&self) -> CoreBlockHeight {
        self.core_wallet.last_processed_height()
    }

    /// Forwarded, plus the receival accounts platform inserts itself: the
    /// filter pipeline snapshots this per batch and refuses to certify a
    /// batch whose snapshot predates an account add, because the scan never
    /// tested that account's scripts (dashpay/rust-dashcore#649). The trait
    /// default is a constant `0`, which turned the guard off for every
    /// platform wallet — so a batch in flight across a contact-account
    /// registration could certify coverage the DashPay backfill record then
    /// vouched for (dashpay/platform#4302 review).
    ///
    /// Also offset past a removed same-id predecessor's final generation
    /// (see `RewindBarrier::inherit`): both counters restart with the wallet,
    /// so without the offset a batch reconciled against the predecessor's
    /// account set could match this wallet's generation and be certified.
    fn account_generation(&self) -> u64 {
        self.core_wallet
            .account_generation()
            .wrapping_add(self.rewind_barrier.account_registrations())
            .wrapping_add(self.rewind_barrier.generation_base())
    }

    fn synced_height(&self) -> CoreBlockHeight {
        self.core_wallet.synced_height()
    }

    fn update_last_processed_height(&mut self, current_height: u32) {
        self.core_wallet
            .update_last_processed_height(current_height);
    }

    /// The engine's only caller is `update_wallet_synced_height`, which emits
    /// a `SyncHeightAdvanced` straight after this returns, under the same
    /// manager write lock — so this is where emissions are counted for the
    /// [`RewindBarrier`](crate::changeset::core_bridge::RewindBarrier).
    /// Platform code that moves the cursor calls `core_wallet` directly and
    /// emits nothing.
    fn update_synced_height(&mut self, current_height: u32) {
        self.rewind_barrier.note_emitted();
        self.core_wallet.update_synced_height(current_height);
    }

    fn truncate_above(&mut self, height: CoreBlockHeight) -> Result<Truncation, key_wallet::Error> {
        self.core_wallet.truncate_above(height)
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
        // key-wallet at the pinned revision rewrites the record's context to
        // `InstantSend` even when the transaction is already in a block. That
        // context carries no block, so a chain lock no longer promotes the
        // record and the persister would store a mined send as unconfirmed.
        // A lock can reach a mined transaction: dash-spv applies one for as
        // long as its mempool manager tracks the transaction, which ends
        // only when that manager handles the block's event. Keep each
        // confirmed slice's own context across the call: the slices of one
        // transaction share the block but not always the finality, so one
        // slice's context must not stand in for another's. As upstream, the
        // mined record then does not carry the lock: an asset lock mined
        // before its lock is proven by the chain lock, not by the
        // InstantSend lock. Fixed upstream in dashpay/rust-dashcore#1037;
        // drop this once the pin carries it (tracked in #5430).
        let confirmed: BTreeMap<_, _> = self
            .core_wallet
            .accounts
            .all_accounts()
            .into_iter()
            .filter_map(|account| account.transactions().get(txid))
            .filter(|record| record.is_confirmed())
            .map(|record| (record.account_type, record.context.clone()))
            .collect();
        let changed = self.core_wallet.mark_instant_send_utxos(txid, lock);
        if confirmed.is_empty() {
            return changed;
        }
        for mut account in self.core_wallet.accounts.all_accounts_mut() {
            let Some(record) = account.transactions_mut().get_mut(txid) else {
                continue;
            };
            if let Some(context) = confirmed.get(&record.account_type) {
                record.update_context(context.clone());
            }
        }
        changed
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

    // The trait default is a no-op, which would keep every observed spend
    // until the first chain lock is applied.
    fn note_chain_lock_height(&mut self, height: CoreBlockHeight) {
        self.core_wallet.note_chain_lock_height(height);
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
        let before = self.core_wallet.metadata.synced_height;
        let added = self.core_wallet.add_managed_account(wallet, account_type);
        self.note_account_add_rewind(before);
        added
    }

    fn add_managed_account_from_xpub(
        &mut self,
        account_type: AccountType,
        account_xpub: ExtendedPubKey,
    ) -> key_wallet::Result<()> {
        let before = self.core_wallet.metadata.synced_height;
        let added = self
            .core_wallet
            .add_managed_account_from_xpub(account_type, account_xpub);
        self.note_account_add_rewind(before);
        added
    }

    #[cfg(feature = "bls")]
    fn add_managed_bls_account(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
    ) -> key_wallet::Result<()> {
        let before = self.core_wallet.metadata.synced_height;
        let added = self
            .core_wallet
            .add_managed_bls_account(wallet, account_type);
        self.note_account_add_rewind(before);
        added
    }

    #[cfg(feature = "bls")]
    fn add_managed_bls_account_from_public_key(
        &mut self,
        account_type: AccountType,
        bls_public_key: [u8; 48],
    ) -> key_wallet::Result<()> {
        let before = self.core_wallet.metadata.synced_height;
        let added = self
            .core_wallet
            .add_managed_bls_account_from_public_key(account_type, bls_public_key);
        self.note_account_add_rewind(before);
        added
    }

    #[cfg(feature = "eddsa")]
    fn add_managed_eddsa_account(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
    ) -> key_wallet::Result<()> {
        let before = self.core_wallet.metadata.synced_height;
        let added = self
            .core_wallet
            .add_managed_eddsa_account(wallet, account_type);
        self.note_account_add_rewind(before);
        added
    }

    #[cfg(feature = "eddsa")]
    fn add_managed_eddsa_account_from_public_key(
        &mut self,
        account_type: AccountType,
        ed25519_public_key: [u8; 32],
    ) -> key_wallet::Result<()> {
        let before = self.core_wallet.metadata.synced_height;
        let added = self
            .core_wallet
            .add_managed_eddsa_account_from_public_key(account_type, ed25519_public_key);
        self.note_account_add_rewind(before);
        added
    }
}

impl PlatformWalletInfo {
    /// key-wallet's account-add API rewinds the scan cursor to just below
    /// wallet birth (the new account has no filter coverage). That lowering
    /// emits nothing and is not persisted here, so treat it like any other
    /// in-memory rewind: advances queued before it must not be stored after
    /// it, and the next DashPay backfill record round owes the host the
    /// lowered cursor — otherwise a record-only round could store coverage
    /// beside a host cursor that never came down (dashpay/platform#4302
    /// review).
    fn note_account_add_rewind(&mut self, before: u32) {
        let after = self.core_wallet.metadata.synced_height;
        if after < before {
            self.rewind_barrier.arm();
            let epoch = self.rewind_barrier.epoch();
            self.dashpay_backfill.owe_cursor(after, epoch);
        }
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

#[cfg(test)]
mod tests {
    use dashcore::{BlockHash, OutPoint, TxIn};
    use key_wallet::transaction_checking::BlockInfo;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;

    use super::*;

    fn spending(outpoint: OutPoint) -> Transaction {
        Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: outpoint,
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        }
    }

    /// dash-spv notes a chain-lock height during the initial sync, before a
    /// chain lock is applied. Spends observed at or below the synced height
    /// are final from then on and are forgotten.
    #[tokio::test]
    async fn should_prune_final_observed_spends_when_a_chain_lock_height_is_noted() {
        let mut wallet = Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::None)
            .expect("test wallet");
        let mut info = PlatformWalletInfo::from_wallet(&wallet, 0);
        let final_spend = OutPoint::new(Txid::from([0x01; 32]), 0);
        let open_spend = OutPoint::new(Txid::from([0x03; 32]), 0);
        for (outpoint, height) in [(final_spend, 50), (open_spend, 150)] {
            let block = BlockInfo::new(height, BlockHash::from([0x07; 32]), 1_735_689_600);
            info.check_core_transaction(
                &spending(outpoint),
                TransactionContext::InBlock(block),
                &mut wallet,
                true,
                false,
            )
            .await;
        }
        info.update_synced_height(100);
        assert_eq!(info.core_wallet.observed_spent_outpoints().len(), 2);

        info.note_chain_lock_height(200);

        let observed = info.core_wallet.observed_spent_outpoints();
        assert!(
            !observed.contains_key(&final_spend),
            "a spend at height 50 is final below synced height 100"
        );
        assert!(
            observed.contains_key(&open_spend),
            "a spend at height 150 is above synced height 100"
        );
    }
}
