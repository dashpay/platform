//! Outpoints a wallet keeps out of coin selection, and their persistence.
//!
//! The lock set lives on `key_wallet`'s `ManagedWalletInfo` (see
//! [`ManagedWalletInfo::locked_outpoints`]): every masternode registration
//! (ProRegTx) the wallet processes locks its collateral, because spending the
//! collateral would end the registration, and coin selection, the asset-lock
//! builder and special-transaction funding all skip a locked coin. This module
//! carries that set to the persister and exposes explicit lock and unlock.
//!
//! Locks reach the persister through [`CoreChangeSet::outpoint_locks`] on two
//! paths:
//!
//! - A transaction check. `check_core_transaction` queues the outpoints it
//!   locked on [`PlatformWalletInfo`], since no `WalletEvent` carries them, and
//!   the wallet-event adapter drains the queue into the next changeset it
//!   stores for the wallet. The `SyncHeightAdvanced` watermark that certifies
//!   a block is a later event for the same wallet, so a lock made in a block
//!   is stored with that watermark or before it.
//! - [`PlatformWallet::lock_outpoint`] and [`PlatformWallet::unlock_outpoint`],
//!   which store the change themselves.
//!
//! On load each backend hands every stored lock back through
//! [`ManagedWalletInfo::lock_outpoint`].
//!
//! # Masternodes the wallet already knows
//!
//! A ProRegTx locks its collateral only when a transaction check sees it.
//! Two kinds of registration can escape that: one restored into the
//! transaction history without a check (the mobile restore stages provider
//! transactions directly, and a wallet from before locks were kept has no
//! stored locks), and one the wallet never processed at all, such as a
//! registration funded and signed elsewhere for a collateral this wallet
//! holds. The first is in the wallet's transaction history; the second is
//! known only when the user tracks the masternode and its registration has
//! been fetched. [`lock_known_masternode_collaterals`] locks both kinds in
//! every wallet of a manager, on load, when a wallet is registered, and when
//! a tracked masternode's registration becomes known.
//!
//! [`ManagedWalletInfo::locked_outpoints`]: key_wallet::wallet::ManagedWalletInfo::locked_outpoints
//! [`ManagedWalletInfo::lock_outpoint`]: key_wallet::wallet::ManagedWalletInfo::lock_outpoint

use std::collections::{BTreeMap, BTreeSet};

use dashcore::blockdata::transaction::special_transaction::TransactionPayload;
use dashcore::hashes::Hash;
use dashcore::{OutPoint, Transaction, Txid};
use key_wallet_manager::WalletManager;
use tokio::sync::RwLock;

use crate::changeset::{CoreChangeSet, PlatformWalletChangeSet};
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::{PlatformWallet, PlatformWalletInfo};

/// The collateral `tx` registers when it is a ProRegTx: the outpoint its
/// payload names, or, when that names a null txid, the ProRegTx's own output
/// at that index. `None` for any other transaction, and for an own-output
/// index past the outputs (such a registration names no coin).
pub(crate) fn registration_collateral(tx: &Transaction) -> Option<OutPoint> {
    let Some(TransactionPayload::ProviderRegistrationPayloadType(registration)) =
        &tx.special_transaction_payload
    else {
        return None;
    };
    let named = registration.collateral_outpoint;
    if named.txid != Txid::all_zeros() {
        return Some(named);
    }
    ((named.vout as usize) < tx.output.len()).then(|| OutPoint::new(tx.txid(), named.vout))
}

/// The collateral of a registration known by its proTxHash and the outpoint
/// its payload names, both as wire-order bytes, the way the tracked registry
/// keeps them. A null txid names the ProRegTx's own output, and the ProRegTx's
/// txid is the proTxHash.
pub(crate) fn named_collateral(pro_tx_hash: &[u8; 32], (txid, vout): ([u8; 32], u32)) -> OutPoint {
    let txid = if txid == [0u8; 32] {
        *pro_tx_hash
    } else {
        txid
    };
    OutPoint::new(Txid::from_byte_array(txid), vout)
}

/// Lock, in every wallet `wallet_manager` holds, the collateral of each
/// masternode registration in that wallet's transaction history and every
/// outpoint in `tracked`, and queue the new locks for persistence. Returns
/// how many locks were added across all wallets.
///
/// A tracked collateral is locked in every wallet, whether or not the wallet
/// holds the coin yet: an entry needs no coin behind it, so a collateral
/// that reaches a wallet later (a restore still syncing) arrives locked.
pub(crate) async fn lock_known_masternode_collaterals(
    wallet_manager: &RwLock<WalletManager<PlatformWalletInfo>>,
    tracked: &BTreeSet<OutPoint>,
) -> usize {
    let mut wm = wallet_manager.write().await;
    let wallet_ids: Vec<_> = wm.list_wallets().into_iter().copied().collect();
    let mut added = 0;
    for wallet_id in &wallet_ids {
        if let Some(info) = wm.get_wallet_info_mut(wallet_id) {
            added += info.lock_known_masternode_collaterals(tracked);
        }
    }
    added
}

impl PlatformWalletInfo {
    /// Lock the collateral of every masternode registration in this wallet's
    /// transaction history, and every outpoint in `tracked`. The new locks
    /// are queued for persistence. Returns how many were added.
    pub(crate) fn lock_known_masternode_collaterals(
        &mut self,
        tracked: &BTreeSet<OutPoint>,
    ) -> usize {
        let mut collaterals: BTreeSet<OutPoint> = self
            .core_wallet
            .accounts
            .all_accounts()
            .iter()
            .flat_map(|account| account.transactions().values())
            .filter_map(|record| registration_collateral(&record.transaction))
            .collect();
        collaterals.extend(tracked.iter().copied());
        let added: Vec<OutPoint> = collaterals
            .into_iter()
            .filter(|outpoint| self.core_wallet.lock_outpoint(*outpoint))
            .collect();
        if !added.is_empty() {
            self.publish_core_balance();
        }
        let count = added.len();
        self.queue_outpoint_locks(added);
        count
    }

    /// Queue locks a transaction check made for the wallet-event adapter to
    /// persist.
    pub(crate) fn queue_outpoint_locks(&self, outpoints: impl IntoIterator<Item = OutPoint>) {
        let mut outpoints = outpoints.into_iter().peekable();
        if outpoints.peek().is_none() {
            return;
        }
        self.pending_outpoint_locks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .extend(outpoints);
    }

    /// Take every queued lock, leaving the queue empty.
    pub(crate) fn take_queued_outpoint_locks(&self) -> BTreeSet<OutPoint> {
        std::mem::take(
            &mut *self
                .pending_outpoint_locks
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
    }

    /// Drop a queued lock for `outpoint`: an explicit lock or unlock stores
    /// the outpoint's state itself, and a queued lock stored after it would
    /// overwrite an unlock.
    fn forget_queued_outpoint_lock(&self, outpoint: &OutPoint) {
        self.pending_outpoint_locks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(outpoint);
    }

    /// Mirror the core balance into the lock-free snapshot the UI reads.
    fn publish_core_balance(&self) {
        let balance = &self.core_wallet.balance;
        self.generation.set(
            balance.confirmed(),
            balance.unconfirmed(),
            balance.immature(),
            balance.locked(),
        );
    }
}

impl PlatformWallet {
    /// The outpoints this wallet keeps out of coin selection, in outpoint
    /// order: the collateral of every masternode registration the wallet has
    /// processed, and every outpoint locked with [`Self::lock_outpoint`].
    ///
    /// An entry does not need a coin behind it: a collateral whose ProRegTx
    /// arrived first, or an outpoint locked before its coin, is listed and
    /// the coin arrives locked.
    pub async fn locked_outpoints(&self) -> Result<Vec<OutPoint>, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm
            .get_wallet_info(&self.wallet_id())
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id())))?;
        Ok(info
            .core_wallet
            .locked_outpoints()
            .iter()
            .copied()
            .collect())
    }

    /// Lock `outpoint`, so no send, asset lock or special-transaction fee
    /// spends it until [`Self::unlock_outpoint`].
    ///
    /// The wallet does not need to hold the coin yet. When it does, the
    /// coin's value moves from the spendable balance to the locked one.
    /// Returns `true` when the outpoint was not locked before.
    ///
    /// The lock is persisted before this returns. The state is stored even
    /// when nothing changed, so calling again repeats a store that failed.
    ///
    /// # Errors
    ///
    /// [`PlatformWalletError::WalletNotFound`] when the wallet is no longer
    /// registered, or the store's error when persisting fails. After a store
    /// failure the wallet still holds the lock in memory, but it does not
    /// survive a restart.
    pub async fn lock_outpoint(&self, outpoint: OutPoint) -> Result<bool, PlatformWalletError> {
        self.set_outpoint_lock(outpoint, true).await
    }

    /// Unlock `outpoint`, so coin selection may spend it again.
    ///
    /// Unlocking a masternode collateral lets a send spend it, and spending
    /// it ends the masternode registration. The wallet never unlocks one on
    /// its own. Processing the registration again (a rescan) locks it again.
    /// Returns `true` when the outpoint was locked.
    ///
    /// Persisted before this returns, like [`Self::lock_outpoint`].
    ///
    /// # Errors
    ///
    /// As [`Self::lock_outpoint`]. After a store failure the outpoint is
    /// unlocked in memory but comes back locked after a restart.
    pub async fn unlock_outpoint(&self, outpoint: OutPoint) -> Result<bool, PlatformWalletError> {
        self.set_outpoint_lock(outpoint, false).await
    }

    async fn set_outpoint_lock(
        &self,
        outpoint: OutPoint,
        locked: bool,
    ) -> Result<bool, PlatformWalletError> {
        // The write lock is held through the store, so a transaction check
        // cannot queue a lock for this outpoint between the change and its
        // persistence.
        let mut wm = self.wallet_manager.write().await;
        let info = wm
            .get_wallet_info_mut(&self.wallet_id())
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id())))?;
        let changed = if locked {
            info.core_wallet.lock_outpoint(outpoint)
        } else {
            info.core_wallet.unlock_outpoint(&outpoint)
        };
        info.forget_queued_outpoint_lock(&outpoint);
        if changed {
            info.publish_core_balance();
        }

        let persister = self.persister();
        let changeset = PlatformWalletChangeSet {
            core: Some(CoreChangeSet {
                outpoint_locks: BTreeMap::from([(outpoint, locked)]),
                ..CoreChangeSet::default()
            }),
            ..PlatformWalletChangeSet::default()
        };
        persister
            .store(changeset)
            .map_err(|e| persister.classify_store_failure(e))?;
        if !persister.store_commits_inline() {
            persister
                .flush()
                .map_err(|e| PlatformWalletError::Persistence(e.to_string()))?;
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use dashcore::{BlockHash, Network, TxOut};
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::managed_account::transaction_record::{
        TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
    use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
    use key_wallet::wallet::Wallet;

    use super::*;
    use crate::changeset::{ClientStartState, PersistenceError, PlatformWalletPersistence};
    use crate::test_support::{
        funded_wallet_manager_with_outputs, masternode_registration, NoopTestEventHandler,
        WalletSigner, MASTERNODE_COLLATERAL_DUFFS,
    };
    use crate::wallet::platform_wallet::WalletId;

    const DASH: u64 = 100_000_000;

    /// Records the outpoint locks of every changeset it stores.
    #[derive(Default)]
    struct LockRecordingPersister {
        stored: Mutex<Vec<BTreeMap<OutPoint, bool>>>,
    }

    impl PlatformWalletPersistence for LockRecordingPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            if let Some(core) = changeset.core {
                if !core.outpoint_locks.is_empty() {
                    self.stored
                        .lock()
                        .expect("stored locks")
                        .push(core.outpoint_locks);
                }
            }
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    fn block(height: u32) -> TransactionContext {
        TransactionContext::InChainLockedBlock(BlockInfo::new(
            height,
            BlockHash::all_zeros(),
            1_700_000_000 + height,
        ))
    }

    #[test]
    fn should_name_the_registrations_own_output_for_a_null_collateral_txid() {
        let pro_tx_hash = [0x5A; 32];
        assert_eq!(
            named_collateral(&pro_tx_hash, ([0u8; 32], 1)),
            OutPoint::new(Txid::from_byte_array(pro_tx_hash), 1)
        );
        assert_eq!(
            named_collateral(&pro_tx_hash, ([0x11; 32], 3)),
            OutPoint::new(Txid::from_byte_array([0x11; 32]), 3)
        );
    }

    /// Unlocking through the wallet persists the unlock and hands the
    /// collateral back to coin selection; locking again persists the lock
    /// and takes it back out.
    #[tokio::test]
    async fn should_persist_explicit_unlock_and_lock_and_spend_the_collateral_only_while_unlocked()
    {
        let persister = Arc::new(LockRecordingPersister::default());
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let manager = Arc::new(crate::PlatformWalletManager::new(
            sdk,
            Arc::clone(&persister),
            Arc::new(NoopTestEventHandler) as _,
        ));
        let seed = [0x3C; 64];
        let wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                &seed,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("create wallet");
        let signer = WalletSigner::for_wallet(
            Wallet::from_seed_bytes(
                seed,
                Network::Testnet,
                WalletAccountCreationOptions::Default,
            )
            .expect("seed wallet"),
        );

        let address = wallet
            .core()
            .next_receive_address_for_account(0)
            .await
            .expect("receive address");
        let funding = Transaction::dummy(&address, 0..1, &[MASTERNODE_COLLATERAL_DUFFS, 5 * DASH]);
        let collateral = OutPoint::new(funding.txid(), 0);
        {
            let mut wm = wallet.wallet_manager().write().await;
            wm.check_transaction_in_all_wallets(&funding, block(1), true, true)
                .await;
            wm.check_transaction_in_all_wallets(
                &masternode_registration(collateral),
                block(2),
                true,
                true,
            )
            .await;
        }
        assert_eq!(
            wallet.locked_outpoints().await.expect("locks"),
            vec![collateral]
        );

        assert!(wallet.unlock_outpoint(collateral).await.expect("unlock"));
        assert!(wallet.locked_outpoints().await.expect("locks").is_empty());
        assert_eq!(
            persister.stored.lock().expect("stored locks").last(),
            Some(&BTreeMap::from([(collateral, false)])),
            "the unlock is persisted"
        );
        {
            let wm = wallet.wallet_manager().read().await;
            let info = wm.get_wallet_info(&wallet.wallet_id()).expect("wallet");
            assert!(
                info.take_queued_outpoint_locks().is_empty(),
                "the unlock drops the lock the registration queued, so the adapter cannot \
                 store it over the unlock"
            );
        }
        assert_eq!(
            wallet.balance().locked(),
            0,
            "the UI balance follows the unlock"
        );

        let finalized = wallet
            .core()
            .finalize_transaction(
                TransactionBuilder::new()
                    .set_selection_strategy(SelectionStrategy::LargestFirst)
                    .add_output(&dashcore::Address::dummy(Network::Testnet, 92), DASH),
                &crate::SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await
            .expect("the unlocked collateral funds the send");
        let inputs: Vec<OutPoint> = finalized
            .transaction()
            .input
            .iter()
            .map(|input| input.previous_output)
            .collect();
        assert_eq!(
            inputs,
            vec![collateral],
            "largest-first takes the unlocked collateral"
        );
        wallet.core().abandon_transaction(&finalized).await;

        assert!(wallet.lock_outpoint(collateral).await.expect("lock"));
        assert!(
            !wallet.lock_outpoint(collateral).await.expect("lock again"),
            "a second lock changes nothing"
        );
        assert_eq!(
            persister.stored.lock().expect("stored locks").last(),
            Some(&BTreeMap::from([(collateral, true)])),
            "the lock is persisted"
        );
        assert_eq!(wallet.balance().locked(), MASTERNODE_COLLATERAL_DUFFS);
        let err = wallet
            .core()
            .finalize_transaction(
                TransactionBuilder::new()
                    .set_selection_strategy(SelectionStrategy::LargestFirst)
                    .add_output(&dashcore::Address::dummy(Network::Testnet, 93), 6 * DASH),
                &crate::SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await
            .expect_err("locked again, the collateral cannot fund 6 DASH");
        assert!(
            matches!(err, PlatformWalletError::CorePooledInsufficientFunds { .. }),
            "expected a funding shortfall, got {err:?}"
        );
    }

    /// A registration restored into the history without a transaction check
    /// (the mobile restore stages provider transactions directly) has its
    /// collateral locked by the known-masternode pass, once.
    #[tokio::test]
    async fn should_lock_the_collateral_of_a_registration_restored_into_the_history() {
        let (manager, wallet_id, _generation, _signer) = funded_wallet_manager_with_outputs(
            StandardAccountType::BIP44Account,
            &[MASTERNODE_COLLATERAL_DUFFS, 5 * DASH],
        )
        .await;
        let collateral = crate::test_support::coin_worth(
            &manager,
            &wallet_id,
            StandardAccountType::BIP44Account,
            MASTERNODE_COLLATERAL_DUFFS,
        )
        .await;
        let registration = masternode_registration(collateral);
        {
            let mut wm = manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet");
            let account = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .expect("bip44 account");
            let record = TransactionRecord::new(
                registration.clone(),
                account.managed_account_type().to_account_type(),
                block(2),
                TransactionType::ProviderRegistration,
                TransactionDirection::Internal,
                Vec::new(),
                Vec::new(),
                0,
            );
            account
                .transactions_mut()
                .insert(registration.txid(), record);
            assert!(!info.core_wallet.is_outpoint_locked(&collateral));
        }

        assert_eq!(
            lock_known_masternode_collaterals(&manager, &BTreeSet::new()).await,
            1
        );
        assert_eq!(
            lock_known_masternode_collaterals(&manager, &BTreeSet::new()).await,
            0,
            "a lock is added once"
        );
        let wm = manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("wallet");
        assert!(info.core_wallet.is_outpoint_locked(&collateral));
        assert_eq!(
            info.core_wallet.balance.locked(),
            MASTERNODE_COLLATERAL_DUFFS
        );
        assert_eq!(info.core_wallet.balance.spendable(), 5 * DASH);
        assert_eq!(
            info.take_queued_outpoint_locks(),
            BTreeSet::from([collateral]),
            "the new lock is queued for persistence"
        );
    }

    /// A tracked masternode's collateral is locked in a wallet that does not
    /// hold the coin yet, so the coin arrives locked when it syncs.
    #[tokio::test]
    async fn should_lock_a_tracked_collateral_before_its_coin_arrives() {
        let (manager, wallet_id, _generation, _signer) =
            funded_wallet_manager_with_outputs(StandardAccountType::BIP44Account, &[5 * DASH])
                .await;
        let address = {
            let wm = manager.read().await;
            let info = wm.get_wallet_info(&wallet_id).expect("wallet");
            info.core_wallet
                .first_bip44_managed_account()
                .expect("bip44 account")
                .utxos
                .values()
                .next()
                .expect("funded coin")
                .address
                .clone()
        };
        let incoming = Transaction {
            output: vec![TxOut {
                value: MASTERNODE_COLLATERAL_DUFFS,
                script_pubkey: address.script_pubkey(),
            }],
            ..Transaction::dummy(&address, 7..8, &[1])
        };
        let collateral = OutPoint::new(incoming.txid(), 0);

        assert_eq!(
            lock_known_masternode_collaterals(&manager, &BTreeSet::from([collateral])).await,
            1
        );
        let mut wm = manager.write().await;
        wm.check_transaction_in_all_wallets(&incoming, block(3), true, true)
            .await;
        let info = wm.get_wallet_info(&wallet_id).expect("wallet");
        assert_eq!(
            info.core_wallet.balance.locked(),
            MASTERNODE_COLLATERAL_DUFFS
        );
        assert_eq!(info.core_wallet.balance.spendable(), 5 * DASH);
    }
}
