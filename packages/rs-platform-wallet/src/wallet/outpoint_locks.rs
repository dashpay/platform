//! Outpoints a wallet keeps out of coin selection, and their persistence.
//!
//! The lock set lives on `key_wallet`'s `ManagedWalletInfo` (see
//! [`ManagedWalletInfo::locked_outpoints`]): a masternode registration
//! (ProRegTx) the wallet processes locks its collateral, because spending the
//! collateral would end the registration, and coin selection, the asset-lock
//! builder and special-transaction funding all skip a locked coin. A
//! collateral the ProRegTx names is locked once the ProRegTx is in a block; a
//! collateral it creates, as its own output, from any sighting. This module
//! carries that set to the persister and exposes explicit lock and unlock.
//!
//! Locks reach the persister through [`CoreChangeSet::outpoint_locks`] on
//! three paths:
//!
//! - A transaction check. The `BlockProcessed` or `TransactionDetected` event
//!   of the check names the outpoints it locked, and the wallet-event adapter
//!   stores them in that event's changeset, so a lock is stored with the
//!   block or transaction that made it.
//! - [`PlatformWallet::lock_outpoint`] and [`PlatformWallet::unlock_outpoint`],
//!   which store the change themselves.
//! - The pass over the masternodes a wallet already knows (below), whose
//!   caller stores what it locked with [`persist_outpoint_locks`].
//!
//! An event still waiting in the adapter's channel when an outpoint is
//! unlocked by hand is stored after the unlock, so the store keeps that
//! outpoint locked and the next load locks it again.
//!
//! On load each backend hands every stored lock back through
//! [`ManagedWalletInfo::lock_outpoint`].
//!
//! # Masternodes the wallet already knows
//!
//! A ProRegTx locks its collateral only when a transaction check sees it (in a
//! block, for a collateral it names). Two kinds of registration escape that:
//!
//! - One restored into the transaction history without a check: the mobile
//!   restore stages provider transactions directly, and a wallet from before
//!   locks were kept has no stored locks. On load,
//!   [`PlatformWalletInfo::lock_registered_collaterals`] locks the collateral
//!   of every registration in the wallet's history.
//! - One the wallet never processed, such as a registration funded and signed
//!   elsewhere for a collateral this wallet holds. Key-wallet locks such a
//!   collateral only when it is handed the registration's block while holding
//!   the coin, and a compact-filter client is not handed that block. The
//!   registration is known only when the user tracks the masternode and a
//!   refresh has fetched it.
//!
//! For the second kind each wallet keeps the collaterals of the tracked
//! masternodes ([`PlatformWalletInfo::tracked_collaterals`]) and locks one
//! only when it holds the coin: when the collateral becomes tracked (on load,
//! when the wallet is registered, when a refresh learns the registration),
//! and when a transaction check brings the coin in. A wallet that never holds
//! the coin never lists it. A collateral already tracked is not locked again
//! by a later refresh, so an unlock by hand lasts until the next load.
//!
//! [`ManagedWalletInfo::locked_outpoints`]: key_wallet::wallet::ManagedWalletInfo::locked_outpoints
//! [`ManagedWalletInfo::lock_outpoint`]: key_wallet::wallet::ManagedWalletInfo::lock_outpoint

use std::collections::{BTreeMap, BTreeSet};

use dashcore::blockdata::transaction::special_transaction::TransactionPayload;
use dashcore::{OutPoint, Transaction, Txid};
use key_wallet::transaction_checking::account_checker::TransactionCheckResult;
use key_wallet_manager::WalletManager;
use tokio::sync::RwLock;

use crate::changeset::{CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence};
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::{PlatformWallet, PlatformWalletInfo, WalletId};

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

/// On load: lock, in every wallet `wallet_manager` holds, the collateral of
/// each masternode registration in that wallet's transaction history, and
/// start tracking the collaterals in `tracked` (see
/// [`PlatformWalletInfo::track_collaterals`]). Returns the locks it added,
/// by wallet, for the caller to store with [`persist_outpoint_locks`].
pub(crate) async fn lock_known_masternode_collaterals(
    wallet_manager: &RwLock<WalletManager<PlatformWalletInfo>>,
    tracked: &BTreeSet<OutPoint>,
) -> BTreeMap<WalletId, Vec<OutPoint>> {
    lock_in_every_wallet(wallet_manager, |info| {
        info.lock_known_masternode_collaterals(tracked)
    })
    .await
}

/// Start tracking the collaterals in `tracked` in every wallet
/// `wallet_manager` holds (see [`PlatformWalletInfo::track_collaterals`]).
/// Returns the locks it added, by wallet, for the caller to store with
/// [`persist_outpoint_locks`].
pub(crate) async fn track_masternode_collaterals(
    wallet_manager: &RwLock<WalletManager<PlatformWalletInfo>>,
    tracked: &BTreeSet<OutPoint>,
) -> BTreeMap<WalletId, Vec<OutPoint>> {
    lock_in_every_wallet(wallet_manager, |info| info.track_collaterals(tracked)).await
}

/// Run `lock` on every wallet `wallet_manager` holds and collect the locks
/// it reports adding, by wallet.
async fn lock_in_every_wallet(
    wallet_manager: &RwLock<WalletManager<PlatformWalletInfo>>,
    lock: impl Fn(&mut PlatformWalletInfo) -> Vec<OutPoint>,
) -> BTreeMap<WalletId, Vec<OutPoint>> {
    let mut wm = wallet_manager.write().await;
    let wallet_ids: Vec<_> = wm.list_wallets().into_iter().copied().collect();
    let mut added = BTreeMap::new();
    for wallet_id in wallet_ids {
        if let Some(info) = wm.get_wallet_info_mut(&wallet_id) {
            let locked = lock(info);
            if !locked.is_empty() {
                added.insert(wallet_id, locked);
            }
        }
    }
    added
}

/// Store `locks`, the outpoints locked in each wallet outside a transaction
/// check, so they survive a restart.
///
/// A store that fails is logged and nothing else: the wallet holds the lock
/// in memory either way, and the next load makes it again from the same
/// registration.
pub(crate) fn persist_outpoint_locks(
    persister: &dyn PlatformWalletPersistence,
    locks: BTreeMap<WalletId, Vec<OutPoint>>,
) {
    for (wallet_id, locked) in locks {
        if locked.is_empty() {
            continue;
        }
        let changeset = PlatformWalletChangeSet {
            core: Some(CoreChangeSet {
                outpoint_locks: locked
                    .into_iter()
                    .map(|outpoint| (outpoint, true))
                    .collect(),
                ..CoreChangeSet::default()
            }),
            ..PlatformWalletChangeSet::default()
        };
        if let Err(error) = persister.store(wallet_id, changeset) {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                %error,
                "failed to store masternode collateral locks; they are made again on the next load"
            );
        }
    }
}

impl PlatformWalletInfo {
    /// Lock the collateral of the masternode registrations in this wallet's
    /// transaction history and start tracking the collaterals in `tracked`.
    /// Returns the locks it added, for the caller to store.
    pub(crate) fn lock_known_masternode_collaterals(
        &mut self,
        tracked: &BTreeSet<OutPoint>,
    ) -> Vec<OutPoint> {
        let mut added = self.lock_registered_collaterals();
        added.extend(self.track_collaterals(tracked));
        added
    }

    /// Lock the collateral of the masternode registrations in this wallet's
    /// transaction history. Returns the locks it added.
    ///
    /// The same rule as a transaction check: a collateral a registration
    /// names is locked only when its record is in a block, since an
    /// unconfirmed ProRegTx is not a registration yet; a collateral it
    /// creates, as its own output, from any record. The check that confirms
    /// an unconfirmed registration locks the collateral it names.
    pub(crate) fn lock_registered_collaterals(&mut self) -> Vec<OutPoint> {
        let collaterals: BTreeSet<OutPoint> = self
            .core_wallet
            .accounts
            .all_accounts()
            .iter()
            .flat_map(|account| account.transactions().values())
            .filter_map(|record| {
                let collateral = registration_collateral(&record.transaction)?;
                let creates_it = collateral.txid == record.txid;
                (creates_it || record.is_confirmed()).then_some(collateral)
            })
            .collect();
        self.lock_and_publish(collaterals)
    }

    /// Start tracking `collaterals`, the collateral of tracked masternodes
    /// (see [`Self::tracked_collaterals`]), and lock the newly tracked ones
    /// this wallet holds. Returns the locks it added.
    ///
    /// A collateral the wallet does not hold is only remembered:
    /// [`Self::lock_arrived_tracked_collaterals`] locks it when its coin
    /// arrives. One that was already tracked is left as it is, so a refresh
    /// does not lock again what the user unlocked.
    pub(crate) fn track_collaterals(&mut self, collaterals: &BTreeSet<OutPoint>) -> Vec<OutPoint> {
        let mut held = Vec::new();
        for outpoint in collaterals {
            if self.tracked_collaterals.insert(*outpoint) && self.holds_coin(outpoint) {
                held.push(*outpoint);
            }
        }
        self.lock_and_publish(held)
    }

    /// Lock each tracked masternode's collateral that `tx` creates and this
    /// wallet now holds, and report it in `result` like a lock the check
    /// made: the wallet event of the check then names it, and the
    /// wallet-event adapter stores it.
    ///
    /// Call it once the accounts recorded `tx`. Checking the transaction
    /// again locks the collateral again after an unlock, as checking a
    /// registration again does.
    pub(crate) fn lock_arrived_tracked_collaterals(
        &mut self,
        tx: &Transaction,
        result: &mut TransactionCheckResult,
    ) {
        if self.tracked_collaterals.is_empty() {
            return;
        }
        let txid = tx.txid();
        let arrived: Vec<OutPoint> = self
            .tracked_collaterals
            .iter()
            .filter(|outpoint| outpoint.txid == txid && self.holds_coin(outpoint))
            .copied()
            .collect();
        for outpoint in arrived {
            if self.core_wallet.lock_outpoint(outpoint) {
                result.locked_outpoints.push(outpoint);
                // The lock set is persisted wallet state.
                result.state_modified = true;
            }
        }
    }

    /// Whether a funds account of this wallet holds `outpoint` as an unspent
    /// coin.
    fn holds_coin(&self, outpoint: &OutPoint) -> bool {
        self.core_wallet
            .accounts
            .all_funding_accounts()
            .iter()
            .any(|account| account.utxos.contains_key(outpoint))
    }

    /// Lock `outpoints`, mirror the balance to the UI when any lock was
    /// added, and return the ones that were not locked before.
    fn lock_and_publish(&mut self, outpoints: impl IntoIterator<Item = OutPoint>) -> Vec<OutPoint> {
        let added: Vec<OutPoint> = outpoints
            .into_iter()
            .filter(|outpoint| self.core_wallet.lock_outpoint(*outpoint))
            .collect();
        if !added.is_empty() {
            self.publish_core_balance();
        }
        added
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
    /// order: the collateral of the masternode registrations the wallet has
    /// processed (see the module docs for when), and every outpoint locked
    /// with [`Self::lock_outpoint`].
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
        // The write lock is held through the store, so no transaction check
        // changes this outpoint's lock between the change and its
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
        coin_worth, funded_wallet_manager_with_outputs, masternode_registration,
        NoopTestEventHandler, WalletSigner, MASTERNODE_COLLATERAL_DUFFS,
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

    /// A wallet holding a 1,000 DASH collateral and 5 DASH, with a ProRegTx
    /// naming the collateral restored into its history in `context` without a
    /// transaction check (the mobile restore stages provider transactions
    /// directly). Returns the manager, the wallet and the collateral.
    async fn wallet_with_restored_registration(
        context: TransactionContext,
    ) -> (
        Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        WalletId,
        OutPoint,
    ) {
        let (manager, wallet_id, _generation, _signer) = funded_wallet_manager_with_outputs(
            StandardAccountType::BIP44Account,
            &[MASTERNODE_COLLATERAL_DUFFS, 5 * DASH],
        )
        .await;
        let collateral = coin_worth(
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
                context,
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
        (manager, wallet_id, collateral)
    }

    /// A registration restored into the history without a transaction check
    /// has its collateral locked by the known-masternode pass, once.
    #[tokio::test]
    async fn should_lock_the_collateral_of_a_registration_restored_into_the_history() {
        let (manager, wallet_id, collateral) = wallet_with_restored_registration(block(2)).await;

        assert_eq!(
            lock_known_masternode_collaterals(&manager, &BTreeSet::new()).await,
            BTreeMap::from([(wallet_id, vec![collateral])]),
            "the new lock is handed back for persistence"
        );
        assert!(
            lock_known_masternode_collaterals(&manager, &BTreeSet::new())
                .await
                .is_empty(),
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
    }

    /// An unconfirmed ProRegTx in the history is not a registration yet: the
    /// known-masternode pass leaves the collateral it names spendable, as a
    /// transaction check does, and the block holding it locks the collateral.
    #[tokio::test]
    async fn should_lock_a_named_collateral_of_a_restored_registration_only_once_it_is_mined() {
        let (manager, wallet_id, collateral) =
            wallet_with_restored_registration(TransactionContext::Mempool).await;

        assert!(
            lock_known_masternode_collaterals(&manager, &BTreeSet::new())
                .await
                .is_empty()
        );
        {
            let wm = manager.read().await;
            let info = wm.get_wallet_info(&wallet_id).expect("wallet");
            assert!(!info.core_wallet.is_outpoint_locked(&collateral));
            assert_eq!(
                info.core_wallet.balance.spendable(),
                MASTERNODE_COLLATERAL_DUFFS + 5 * DASH
            );
        }

        let mut wm = manager.write().await;
        let checked = wm
            .check_transaction_in_all_wallets(
                &masternode_registration(collateral),
                block(3),
                true,
                true,
            )
            .await;
        assert_eq!(
            checked.per_wallet_locked_outpoints,
            BTreeMap::from([(wallet_id, vec![collateral])]),
            "the check reports the lock, which its event carries to the persister"
        );
        let info = wm.get_wallet_info(&wallet_id).expect("wallet");
        assert!(info.core_wallet.is_outpoint_locked(&collateral));
        assert_eq!(
            info.core_wallet.balance.locked(),
            MASTERNODE_COLLATERAL_DUFFS
        );
    }

    /// A wallet holding one 5 DASH coin, and a transaction paying it a 1,000
    /// DASH coin it has not seen yet. Returns the manager, the wallet, that
    /// transaction and the coin it creates.
    async fn wallet_awaiting_a_collateral() -> (
        Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        WalletId,
        Transaction,
        OutPoint,
    ) {
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
        (manager, wallet_id, incoming, collateral)
    }

    /// A tracked masternode's collateral the wallet does not hold is not
    /// locked there and is not listed. It is locked by the transaction check
    /// that brings the coin in, which reports the lock for its wallet event.
    #[tokio::test]
    async fn should_lock_a_tracked_collateral_only_once_the_wallet_holds_its_coin() {
        let (manager, wallet_id, incoming, collateral) = wallet_awaiting_a_collateral().await;

        assert!(
            track_masternode_collaterals(&manager, &BTreeSet::from([collateral]))
                .await
                .is_empty(),
            "nothing is locked in a wallet that does not hold the coin"
        );
        let mut wm = manager.write().await;
        assert!(wm
            .get_wallet_info(&wallet_id)
            .expect("wallet")
            .core_wallet
            .locked_outpoints()
            .is_empty());

        let checked = wm
            .check_transaction_in_all_wallets(&incoming, block(3), true, true)
            .await;
        assert_eq!(
            checked.per_wallet_locked_outpoints,
            BTreeMap::from([(wallet_id, vec![collateral])]),
            "the check that brings the coin in reports its lock"
        );
        let info = wm.get_wallet_info(&wallet_id).expect("wallet");
        assert_eq!(
            info.core_wallet.balance.locked(),
            MASTERNODE_COLLATERAL_DUFFS
        );
        assert_eq!(info.core_wallet.balance.spendable(), 5 * DASH);
    }

    /// A coin that is no tracked masternode's collateral arrives spendable.
    #[tokio::test]
    async fn should_leave_an_arriving_coin_spendable_when_no_tracked_masternode_names_it() {
        let (manager, wallet_id, incoming, collateral) = wallet_awaiting_a_collateral().await;
        let other = OutPoint::new(collateral.txid, 1);
        track_masternode_collaterals(&manager, &BTreeSet::from([other])).await;

        let mut wm = manager.write().await;
        let checked = wm
            .check_transaction_in_all_wallets(&incoming, block(3), true, true)
            .await;
        assert!(checked.per_wallet_locked_outpoints.is_empty());
        let info = wm.get_wallet_info(&wallet_id).expect("wallet");
        assert_eq!(info.core_wallet.balance.locked(), 0);
        assert_eq!(
            info.core_wallet.balance.spendable(),
            MASTERNODE_COLLATERAL_DUFFS + 5 * DASH
        );
    }

    /// Tracking a masternode whose collateral the wallet already holds locks
    /// it at once, in that wallet. A later refresh of the same masternode
    /// leaves an unlock by hand alone.
    #[tokio::test]
    async fn should_lock_a_held_collateral_when_it_becomes_tracked_and_not_again_on_a_refresh() {
        let (manager, wallet_id, _generation, _signer) = funded_wallet_manager_with_outputs(
            StandardAccountType::BIP44Account,
            &[MASTERNODE_COLLATERAL_DUFFS, 5 * DASH],
        )
        .await;
        let collateral = coin_worth(
            &manager,
            &wallet_id,
            StandardAccountType::BIP44Account,
            MASTERNODE_COLLATERAL_DUFFS,
        )
        .await;
        let tracked = BTreeSet::from([collateral]);

        assert_eq!(
            track_masternode_collaterals(&manager, &tracked).await,
            BTreeMap::from([(wallet_id, vec![collateral])]),
            "the new lock is handed back for persistence"
        );
        {
            let mut wm = manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet");
            assert_eq!(
                info.core_wallet.balance.locked(),
                MASTERNODE_COLLATERAL_DUFFS
            );
            assert!(info.core_wallet.unlock_outpoint(&collateral));
        }

        assert!(
            track_masternode_collaterals(&manager, &tracked)
                .await
                .is_empty(),
            "a refresh does not lock again what was unlocked by hand"
        );
        let wm = manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("wallet");
        assert!(!info.core_wallet.is_outpoint_locked(&collateral));
    }
}
