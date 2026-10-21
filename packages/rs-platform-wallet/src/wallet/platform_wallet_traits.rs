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

    fn matured_coinbase_records(
        &self,
        old_height: CoreBlockHeight,
        new_height: CoreBlockHeight,
    ) -> Vec<TransactionRecord> {
        self.core_wallet
            .matured_coinbase_records(old_height, new_height)
    }

    fn mark_instant_send_utxos(&mut self, txid: &Txid, lock: &InstantLock) -> bool {
        // Older key-wallet rewrote the record's context to `InstantSend` even
        // when the transaction was already in a block. That context carries no
        // block, so a chain lock no longer promoted the record and the
        // persister would store a mined send as unconfirmed. A lock can reach
        // a mined transaction: dash-spv applies one for as long as its mempool
        // manager tracks the transaction, which ends only when that manager
        // handles the block's event. Keep each confirmed slice's own context
        // across the call: the slices of one transaction share the block but
        // not always the finality, so one slice's context must not stand in
        // for another's. As upstream, the mined record then does not carry the
        // lock: an asset lock mined before its lock is proven by the chain
        // lock, not by the InstantSend lock. Fixed upstream in
        // dashpay/rust-dashcore#1037, which the pinned key-wallet carries: it
        // leaves a confirmed record's context alone, so this restore is now a
        // no-op and can be dropped (tracked in #5430).
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

    fn unrecorded_spend_heights(&self, tx: &Transaction) -> BTreeSet<CoreBlockHeight> {
        self.core_wallet.unrecorded_spend_heights(tx)
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

    /// During the initial sync dash-spv holds chainlocks back and passes on
    /// only their heights. The trait default ignores them, which would leave
    /// the inner wallet's observed spends unpruned until the sync completes;
    /// forwarded, they are pruned as the sync goes.
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
    //! The delegations dash-spv reaches only through `WalletManager`, driven
    //! through that manager so a missing or wrong forward shows up as the
    //! behaviour it breaks.

    use super::*;
    use dashcore::hashes::Hash;
    use dashcore::{Block, BlockHash, OutPoint, TxIn, TxOut, Witness};
    use key_wallet::test_utils::TestWalletContext;
    use key_wallet::transaction_checking::BlockInfo;
    use key_wallet_manager::{WalletId, WalletInterface, WalletManager};

    fn in_block(height: u32) -> TransactionContext {
        TransactionContext::InBlock(BlockInfo::new(
            height,
            BlockHash::dummy(height),
            1_700_000_000,
        ))
    }

    /// A transaction spending `previous_output` to an address that is not
    /// the wallet's.
    fn spend_of(previous_output: OutPoint) -> Transaction {
        Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output,
                script_sig: ScriptBuf::new(),
                sequence: 0xffffffff,
                witness: Witness::new(),
            }],
            output: vec![TxOut {
                value: 90_000,
                script_pubkey: DashAddress::dummy(Network::Testnet, 7).script_pubkey(),
            }],
            special_transaction_payload: None,
        }
    }

    /// A manager holding `ctx`'s wallet as a `PlatformWalletInfo`.
    fn manager_holding(ctx: TestWalletContext) -> (WalletManager<PlatformWalletInfo>, WalletId) {
        let mut info = PlatformWalletInfo::from_wallet(&ctx.wallet, 0);
        info.core_wallet = ctx.managed_wallet;
        let mut manager = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
        let wallet_id = manager
            .insert_wallet(ctx.wallet, info)
            .expect("insert wallet");
        (manager, wallet_id)
    }

    /// During the initial sync dash-spv holds chainlocks back and only
    /// passes their height on. That exact height must reach the inner
    /// wallet, which then forgets the spends it observed at or below both
    /// the noted height and its synced height; otherwise a restore keeps
    /// every spend it saw in memory until the sync completes. The synced
    /// height is set above the noted one so the noted height is the cap.
    #[tokio::test]
    async fn should_prune_observed_spends_up_to_a_noted_chain_lock_height() {
        let mut ctx = TestWalletContext::new_random();
        let below = OutPoint::new(Txid::from_byte_array([0x01; 32]), 0);
        let at = OutPoint::new(Txid::from_byte_array([0x02; 32]), 0);
        let above = OutPoint::new(Txid::from_byte_array([0x03; 32]), 0);
        ctx.check_transaction(&spend_of(below), in_block(50)).await;
        ctx.check_transaction(&spend_of(at), in_block(100)).await;
        ctx.check_transaction(&spend_of(above), in_block(150)).await;
        ctx.managed_wallet.update_synced_height(200);
        let (mut manager, wallet_id) = manager_holding(ctx);

        manager.note_chain_lock_height(100);

        let core = &manager
            .get_wallet_info(&wallet_id)
            .expect("wallet info")
            .core_wallet;
        let observed = core.observed_spent_outpoints();
        assert!(
            !observed.contains_key(&below),
            "a spend below the noted chainlock height is final"
        );
        assert!(
            !observed.contains_key(&at),
            "a spend at the noted chainlock height is final"
        );
        assert!(
            observed.contains_key(&above),
            "a spend above the noted chainlock height is not final yet"
        );
        assert!(
            core.last_applied_chain_lock().is_none(),
            "a noted height is not an applied chainlock"
        );
    }

    /// A block that spends a coin can be processed before the block that
    /// funds it; the spend then matched nothing and was not recorded. When
    /// the funding block lands, the manager must name the spend's block for
    /// dash-spv to apply again, and applying it records the spend.
    #[tokio::test]
    async fn should_ask_to_reapply_the_block_of_a_spend_seen_before_its_funding() {
        let ctx = TestWalletContext::new_random();
        let funding = Transaction::dummy(&ctx.receive_address, 0..1, &[100_000]);
        let spend = spend_of(OutPoint::new(funding.txid(), 0));
        let (mut manager, wallet_id) = manager_holding(ctx);
        let wallets = BTreeSet::from([wallet_id]);
        let spend_block = Block::dummy(200, vec![spend.clone()]);
        let funding_block = Block::dummy(100, vec![funding.clone()]);

        manager
            .process_block_for_wallets(&spend_block, spend_block.block_hash(), 200, &wallets)
            .await;
        let funded = manager
            .process_block_for_wallets(&funding_block, funding_block.block_hash(), 100, &wallets)
            .await;
        assert_eq!(
            funded.reapply_heights,
            BTreeMap::from([(wallet_id, BTreeSet::from([200]))])
        );

        manager
            .process_block_for_wallets(&spend_block, spend_block.block_hash(), 200, &wallets)
            .await;
        let info = manager.get_wallet_info(&wallet_id).expect("wallet info");
        assert!(
            info.transaction_history()
                .iter()
                .any(|record| record.txid == spend.txid()),
            "applying the block again records the spend"
        );
        assert!(info.unrecorded_spend_heights(&funding).is_empty());
    }
}
