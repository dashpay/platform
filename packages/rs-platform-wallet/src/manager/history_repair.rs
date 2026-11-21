//! Wallet-owned replay and repair of one wallet's stored history inside a backend transaction.

use std::collections::BTreeSet;

use dashcore::{OutPoint, Txid};
use key_wallet::transaction_checking::TransactionContext;

use super::history_claims::{
    HistoryReplayEffects, HistoryReplayOperationEffects, StoredReplayClaim,
};
use super::history_replay::{replay_wallet_history_blocking, HistoryReplay, HistoryReplayError};
use crate::changeset::recorded_history::ReplayEvidence;
use crate::changeset::{ClientWalletStartState, PersistenceError, RecordedHistory};

/// One wallet's storage, read and written inside the backend's open transaction.
///
/// Implementations only marshal: [`repair_wallet_history`] decides what is
/// read, replayed, written and restored.
pub trait HistoryRepairStore {
    /// The wallet's current snapshot, or `None` when the wallet does not exist.
    fn read_wallet(&mut self) -> Result<Option<ClientWalletStartState>, PersistenceError>;

    /// The wallet's durable spend claims.
    ///
    /// A backend whose claim rows carry coin material adds it to
    /// `history.owned_prevouts`; the wallet checks its ownership.
    fn read_claims(
        &mut self,
        history: &mut RecordedHistory,
    ) -> Result<Vec<StoredReplayClaim>, PersistenceError>;

    /// Write the replay's storage and operation effects in the open transaction.
    fn write_effects(
        &mut self,
        effects: &HistoryReplayEffects,
        operations: &HistoryReplayOperationEffects,
    ) -> Result<(), PersistenceError>;
}

/// Why [`repair_wallet_history`] could not produce a settled wallet.
#[derive(Debug, thiserror::Error)]
pub enum HistoryRepairError {
    /// The replay rejected the stored history.
    #[error(transparent)]
    Replay(#[from] HistoryReplayError),
    /// The backend failed to read or write.
    #[error(transparent)]
    Store(#[from] PersistenceError),
    /// The wallet disappeared between the writes and the re-read.
    #[error("wallet disappeared during atomic history replay")]
    WalletVanished,
}

impl From<HistoryRepairError> for PersistenceError {
    fn from(error: HistoryRepairError) -> Self {
        match error {
            HistoryRepairError::Replay(error) => PersistenceError::backend(error),
            HistoryRepairError::Store(error) => error,
            error @ HistoryRepairError::WalletVanished => PersistenceError::backend(error),
        }
    }
}

/// Replay one wallet's stored history, write the repair it needs and restore its spend claims.
///
/// Call inside the backend's transaction; the caller commits on `Ok`, rolls
/// back on `Err`. Returns `None` when the wallet does not exist.
pub fn repair_wallet_history(
    store: &mut impl HistoryRepairStore,
    evidence: &ReplayEvidence,
) -> Result<Option<(ClientWalletStartState, HistoryReplay)>, HistoryRepairError> {
    let Some(mut state) = store.read_wallet()? else {
        return Ok(None);
    };
    let mut claims = store.read_claims(&mut state.recorded_history)?;
    let outcome = replay_wallet_history_blocking(&mut state, evidence)?;
    let records = state.recorded_history.transactions.clone();
    let effects = outcome.storage_effects(&records, &claims)?;
    let has_effects = !effects.removed_records.is_empty()
        || !effects.removed_evidence.is_empty()
        || !effects.claims.is_empty();
    let mut remaining = None;
    if !has_effects {
        if !outcome.sweeps.is_empty() || !outcome.authorization.rejected.is_empty() {
            return Err(HistoryReplayError::UnsettledAfterReplay.into());
        }
    } else {
        let operations = effects.operation_effects(&state);
        store.write_effects(&effects, &operations)?;
        state = store
            .read_wallet()?
            .ok_or(HistoryRepairError::WalletVanished)?;
        claims = store.read_claims(&mut state.recorded_history)?;
        let replay = replay_wallet_history_blocking(&mut state, evidence)?;
        if !replay.sweeps.is_empty()
            || !replay.authorization.rejected.is_empty()
            || !replay.withheld_asset_lock_inputs.is_empty()
        {
            return Err(HistoryReplayError::UnsettledAfterReplay.into());
        }
        state.unconfirmed_outgoing_txs =
            outcome.resendable(std::mem::take(&mut state.unconfirmed_outgoing_txs));
        remaining = Some(replay);
    }
    let restored = restorable_claims(&state, remaining.as_ref().unwrap_or(&outcome), &claims);
    state.wallet_info.restore_spent_outpoints(&restored);
    Ok(Some((state, outcome)))
}

/// The stored claims to restore after `replay` settled `state`.
///
/// A claim is left out only when the replay guards its coin in a way a fork
/// releases after the claimant is mined: the coin's funding and its claimant
/// both went through the wallet checker and were not retired, the claimant
/// in the mempool or a plain block above the restored chain lock and without an
/// InstantSend lock, and the claim was not stamped at or below the chain lock.
/// The claimant's account then records the spend of the coin, and a fork
/// dropping the claimant gives the coin back; a claim would keep it spent.
/// Without the funding the spend is only an observed spend, which finality
/// prunes, so the claim stays.
pub(super) fn restorable_claims(
    state: &ClientWalletStartState,
    replay: &HistoryReplay,
    claims: &[StoredReplayClaim],
) -> Vec<(OutPoint, Option<Txid>)> {
    let chain_locked = state
        .wallet_info
        .metadata
        .last_applied_chain_lock
        .as_ref()
        .map(|lock| lock.block_height);
    // A record the replay withheld or set aside never reached the checker.
    let funded: BTreeSet<Txid> = replay
        .replayed_txids
        .iter()
        .filter(|txid| !replay.retired.contains(*txid))
        .copied()
        .collect();
    let reversible: BTreeSet<Txid> = state
        .recorded_history
        .transactions
        .iter()
        .filter(|stored| {
            funded.contains(&stored.transaction.txid()) && !replay.retired.contains(&stored.txid)
        })
        .filter(|stored| match &stored.context {
            TransactionContext::Mempool => true,
            TransactionContext::InBlock(block) => {
                chain_locked.is_none_or(|locked| block.height() > locked)
            }
            _ => false,
        })
        .map(|stored| stored.transaction.txid())
        .filter(|txid| !state.wallet_info.instant_send_locks().contains(txid))
        .collect();
    // TODO(restore-observed-spends): a claim on a coin whose funding the history
    // lacks stays a claim even when its spend is reversible, which no fork
    // releases; making it an observed spend that survives finality needs an
    // upstream API such as `ManagedWalletInfo::restore_observed_spends`.
    claims
        .iter()
        .filter(|claim| {
            let released_by_fork = funded.contains(&claim.outpoint.txid)
                && claim.claimant.is_some_and(|id| reversible.contains(&id))
                && !claim
                    .mined_height
                    .is_some_and(|height| chain_locked.is_some_and(|locked| height <= locked));
            !released_by_fork
        })
        .map(|claim| (claim.outpoint, claim.claimant))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changeset::StoredTransaction;
    use crate::manager::history_replay::tests::{
        authorize, checked_instant_lock, confirmed_spend_fixture, wallet_with_receive_address,
    };
    use crate::wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
    use crate::AssetLockFundingType;
    use dashcore::bls_sig_utils::BLSSignature;
    use dashcore::ephemerealdata::chain_lock::ChainLock;
    use dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
    use dashcore::transaction::TransactionPayload;
    use dashcore::{Address, BlockHash, Transaction};
    use dashcore::{TxIn, TxOut};
    use key_wallet::transaction_checking::{BlockInfo, WalletTransactionChecker};
    use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
    use key_wallet::wallet::{ManagedWalletInfo, Wallet};
    use std::collections::BTreeMap;

    /// What a [`MemoryStore`] does with the effects written to it.
    #[derive(Clone, Copy, PartialEq)]
    enum OnWrite {
        /// A settled history writes none.
        Refuse,
        /// Delete the removed records from the stored history.
        Apply,
        /// Keep the stored history as it was.
        Ignore,
        /// Lose the wallet before the re-read.
        Vanish,
    }

    /// One wallet's rows held in memory.
    struct MemoryStore {
        wallet: Wallet,
        info: ManagedWalletInfo,
        history: RecordedHistory,
        asset_locks: BTreeMap<u32, BTreeMap<OutPoint, TrackedAssetLock>>,
        claims: Vec<StoredReplayClaim>,
        on_write: OnWrite,
        writes: usize,
    }

    impl MemoryStore {
        fn new(fixture: &ClaimedSpend, info: ManagedWalletInfo, history: RecordedHistory) -> Self {
            Self {
                wallet: fixture.wallet.clone(),
                info,
                history,
                asset_locks: BTreeMap::new(),
                claims: Vec::new(),
                on_write: OnWrite::Refuse,
                writes: 0,
            }
        }

        fn snapshot(&self) -> ClientWalletStartState {
            ClientWalletStartState {
                wallet: self.wallet.clone(),
                wallet_info: self.info.clone(),
                identity_manager: Default::default(),
                unused_asset_locks: self.asset_locks.clone(),
                unconfirmed_outgoing_txs: Vec::new(),
                recorded_history: self.history.clone(),
                dashpay_backfill: Default::default(),
            }
        }
    }

    impl HistoryRepairStore for MemoryStore {
        fn read_wallet(&mut self) -> Result<Option<ClientWalletStartState>, PersistenceError> {
            if self.writes > 0 && self.on_write == OnWrite::Vanish {
                return Ok(None);
            }
            Ok(Some(self.snapshot()))
        }

        fn read_claims(
            &mut self,
            _history: &mut RecordedHistory,
        ) -> Result<Vec<StoredReplayClaim>, PersistenceError> {
            Ok(self.claims.clone())
        }

        fn write_effects(
            &mut self,
            effects: &HistoryReplayEffects,
            _operations: &HistoryReplayOperationEffects,
        ) -> Result<(), PersistenceError> {
            assert!(
                self.on_write != OnWrite::Refuse,
                "a settled history writes no effects"
            );
            self.writes += 1;
            if self.on_write == OnWrite::Apply {
                self.history
                    .transactions
                    .retain(|stored| !effects.removed_records.contains(&stored.txid));
            }
            Ok(())
        }
    }

    fn block(height: u32) -> TransactionContext {
        TransactionContext::InBlock(BlockInfo::new(
            height,
            BlockHash::from_byte_array([height as u8; 32]),
            height,
        ))
    }

    fn chain_lock(block_height: u32) -> ChainLock {
        ChainLock {
            block_height,
            block_hash: BlockHash::from_byte_array([block_height as u8; 32]),
            signature: BLSSignature::from([0; 96]),
        }
    }

    /// The fixture's spend of `spent` at height 101 and its funding at 100,
    /// both as stored rows without recorded owned inputs.
    struct ClaimedSpend {
        wallet: Wallet,
        info: ManagedWalletInfo,
        funding: Transaction,
        funding_row: StoredTransaction,
        spender: StoredTransaction,
        spent: OutPoint,
    }

    async fn claimed_spend() -> ClaimedSpend {
        let (wallet, info, funding, records, spent) = confirmed_spend_fixture().await;
        let (funding_rows, spenders): (Vec<_>, Vec<_>) = records
            .into_iter()
            .map(|record| StoredTransaction {
                owned_inputs: Vec::new(),
                ..StoredTransaction::from(record)
            })
            .partition(|stored| stored.txid == funding.txid());
        let [funding_row] = <[_; 1]>::try_from(funding_rows).unwrap();
        let [spender] = <[_; 1]>::try_from(spenders).unwrap();
        assert_eq!(spender.context, block(101));
        ClaimedSpend {
            wallet,
            info,
            funding,
            funding_row,
            spender,
            spent,
        }
    }

    fn stored_history(transactions: Vec<StoredTransaction>) -> RecordedHistory {
        RecordedHistory {
            transactions,
            ..Default::default()
        }
    }

    /// Load `history` with one stored claim on `spent`, as a backend would.
    fn load(
        fixture: &ClaimedSpend,
        info: ManagedWalletInfo,
        history: Vec<StoredTransaction>,
        claim: StoredReplayClaim,
        evidence: &ReplayEvidence,
    ) -> ClientWalletStartState {
        let mut store = MemoryStore::new(fixture, info, stored_history(history));
        store.claims = vec![claim];
        repair_wallet_history(&mut store, evidence)
            .unwrap()
            .expect("the wallet exists")
            .0
    }

    async fn credits_funding(state: &mut ClientWalletStartState, funding: &Transaction) -> bool {
        state
            .wallet_info
            .check_core_transaction(funding, block(100), &mut state.wallet, true, true)
            .await;
        state.wallet_info.accounts.standard_bip44_accounts[&0]
            .utxos
            .contains_key(&OutPoint::new(funding.txid(), 0))
    }

    #[tokio::test]
    async fn should_credit_legacy_status_only_funding_after_its_spend_is_mined_then_reorged() {
        let fixture = claimed_spend().await;
        let mut spender = fixture.spender.clone();
        spender.context = TransactionContext::Mempool;
        let mut history = stored_history(vec![fixture.funding_row.clone(), spender]);
        history.required_instant_locks.insert(fixture.spender.txid);
        let mut store = MemoryStore::new(&fixture, fixture.info.clone(), history);
        let (mut state, _) = repair_wallet_history(&mut store, &ReplayEvidence::default())
            .unwrap()
            .unwrap();

        assert!(!credits_funding(&mut state, &fixture.funding).await);
        state
            .wallet_info
            .check_core_transaction(
                &fixture.spender.transaction,
                block(101),
                &mut state.wallet,
                true,
                true,
            )
            .await;
        assert!(!credits_funding(&mut state, &fixture.funding).await);
        let truncation = state.wallet_info.truncate_above(100).unwrap();
        assert!(truncation.txids.contains(&fixture.spender.txid));
        assert!(
            credits_funding(&mut state, &fixture.funding).await,
            "a fork removing the reversible legacy spend must release its funding"
        );
    }

    #[tokio::test]
    async fn should_keep_legacy_status_only_spends_guarded_after_finality() {
        for funding_present in [true, false] {
            let fixture = claimed_spend().await;
            let mut spender = fixture.spender.clone();
            spender.context = TransactionContext::Mempool;
            let mut history = stored_history(vec![spender]);
            if funding_present {
                history.transactions.push(fixture.funding_row.clone());
            }
            history.required_instant_locks.insert(fixture.spender.txid);
            let mut store = MemoryStore::new(&fixture, fixture.info.clone(), history);
            let (mut state, _) = repair_wallet_history(&mut store, &ReplayEvidence::default())
                .unwrap()
                .unwrap();
            state
                .wallet_info
                .check_core_transaction(
                    &fixture.spender.transaction,
                    block(101),
                    &mut state.wallet,
                    true,
                    true,
                )
                .await;
            state.wallet_info.apply_chain_lock(chain_lock(300));
            state.wallet_info.update_synced_height(300);
            assert!(
                !credits_funding(&mut state, &fixture.funding).await,
                "a final spend must guard its coin; funding in history: {funding_present}"
            );
        }
    }

    /// A stored claim named after a spend in a plain block above the restored
    /// chain lock must not outlive a fork that drops that spend, while the
    /// replayed spend keeps guarding the coin once it is final.
    #[tokio::test]
    async fn should_credit_funding_after_a_fork_drops_a_claimed_reversible_spend() {
        for fork in [true, false] {
            let fixture = claimed_spend().await;
            let mut info = fixture.info.clone();
            info.metadata.last_applied_chain_lock = Some(chain_lock(100));
            let claim = StoredReplayClaim {
                outpoint: fixture.spent,
                claimant: Some(fixture.spender.txid),
                mined_height: None,
            };
            let mut state = load(
                &fixture,
                info,
                vec![fixture.funding_row.clone(), fixture.spender.clone()],
                claim,
                &ReplayEvidence::default(),
            );

            if fork {
                state.wallet_info.truncate_above(100).unwrap();
            } else {
                state.wallet_info.apply_chain_lock(chain_lock(300));
                state.wallet_info.update_synced_height(300);
            }

            assert_eq!(
                credits_funding(&mut state, &fixture.funding).await,
                fork,
                "fork: {fork}: only a fork dropping the claimed spend frees its coin"
            );
        }
    }

    /// Without its funding in the history, the replayed spend guards the coin
    /// only by an observed spend, which finality prunes: the claim must stay.
    #[tokio::test]
    async fn should_not_credit_funding_after_the_dropped_claims_spend_becomes_final() {
        let fixture = claimed_spend().await;
        let mut info = fixture.info.clone();
        info.metadata.last_applied_chain_lock = Some(chain_lock(100));
        let claim = StoredReplayClaim {
            outpoint: fixture.spent,
            claimant: Some(fixture.spender.txid),
            mined_height: None,
        };
        let mut state = load(
            &fixture,
            info,
            vec![fixture.spender.clone()],
            claim,
            &ReplayEvidence::default(),
        );

        state.wallet_info.apply_chain_lock(chain_lock(300));
        state.wallet_info.update_synced_height(300);

        assert!(
            !credits_funding(&mut state, &fixture.funding).await,
            "the spend is final, so its coin must never be credited again"
        );
    }

    /// A chain-final asset lock's funding the replay withholds is not replayed,
    /// so a claim on its change stays even when the change's spend is reversible.
    #[tokio::test]
    async fn should_not_credit_withheld_asset_lock_change_after_its_spend_becomes_final() {
        let (wallet, mut info, address) = wallet_with_receive_address();
        let funding = Transaction {
            version: 3,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::new(Txid::from_byte_array([81; 32]), 0),
                ..Default::default()
            }],
            output: vec![
                TxOut {
                    value: 50_000,
                    script_pubkey: dashcore::script::Builder::new()
                        .push_opcode(dashcore::opcodes::all::OP_RETURN)
                        .into_script(),
                },
                TxOut {
                    value: 40_000,
                    script_pubkey: address.script_pubkey(),
                },
            ],
            special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(
                AssetLockPayload::new(vec![TxOut {
                    value: 50_000,
                    script_pubkey: address.script_pubkey(),
                }]),
            )),
        };
        let change = OutPoint::new(funding.txid(), 1);
        let spender = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: change,
                ..Default::default()
            }],
            output: vec![TxOut {
                value: 39_000,
                script_pubkey: dashcore::ScriptBuf::new(),
            }],
            special_transaction_payload: None,
        };
        let out_point = OutPoint::new(funding.txid(), 0);
        let lock = TrackedAssetLock {
            operation_id: None,
            shielded_recipient: None,
            out_point,
            transaction: funding.clone(),
            account_index: 0,
            funding_type: AssetLockFundingType::IdentityRegistration,
            identity_index: 0,
            amount: 50_000,
            status: AssetLockStatus::ChainLocked,
            proof: None,
        };
        info.metadata.last_applied_chain_lock = Some(chain_lock(100));
        let mut store = MemoryStore {
            wallet,
            info,
            history: stored_history(vec![StoredTransaction {
                txid: spender.txid(),
                transaction: spender.clone(),
                context: block(101),
                stored_net_amount: None,
                stored_direction: None,
                owned_inputs: Vec::new(),
            }]),
            asset_locks: [(0, [(out_point, lock)].into())].into(),
            claims: vec![StoredReplayClaim {
                outpoint: change,
                claimant: Some(spender.txid()),
                mined_height: None,
            }],
            on_write: OnWrite::Refuse,
            writes: 0,
        };
        let (mut state, _) = repair_wallet_history(&mut store, &ReplayEvidence::default())
            .unwrap()
            .unwrap();

        state.wallet_info.apply_chain_lock(chain_lock(300));
        state.wallet_info.update_synced_height(300);
        state
            .wallet_info
            .check_core_transaction(&funding, block(100), &mut state.wallet, true, true)
            .await;

        assert!(
            !state
                .wallet_info
                .utxos()
                .iter()
                .any(|coin| coin.outpoint == change),
            "the change was spent in a final block, so it must never be credited again"
        );
    }

    /// Only a claim the replay can stand in for, and a fork can release, is
    /// left out: every other stored claim is restored.
    #[tokio::test]
    async fn should_restore_a_stored_claim_unless_a_fork_releases_its_replayed_spend() {
        let fixture = claimed_spend().await;
        let (lock, evidence) = checked_instant_lock(&fixture.spender.transaction).await;
        let foreign = Txid::from_byte_array([77; 32]);
        let funding = || fixture.funding_row.clone();
        let spender = |context| StoredTransaction {
            context,
            ..fixture.spender.clone()
        };
        let reversible = || spender(block(101));
        let locks: BTreeMap<_, _> = [(fixture.spender.txid, lock.clone())].into();
        let no_locks = BTreeMap::new;
        let own = fixture.spender.txid;
        for (case, chain_locked, transactions, instant_locks, claimant, stamp, restored) in [
            (
                "reversible spend of a replayed funding",
                Some(100),
                vec![funding(), reversible()],
                no_locks(),
                own,
                None,
                false,
            ),
            (
                "same without any chain lock",
                None,
                vec![funding(), reversible()],
                no_locks(),
                own,
                None,
                false,
            ),
            (
                "reversible spend, funding absent",
                Some(100),
                vec![reversible()],
                no_locks(),
                own,
                None,
                true,
            ),
            (
                "spend in the chain-locked block",
                Some(101),
                vec![funding(), reversible()],
                no_locks(),
                own,
                None,
                true,
            ),
            (
                "spend in a block below the chain lock",
                Some(150),
                vec![funding(), reversible()],
                no_locks(),
                own,
                None,
                true,
            ),
            (
                "mined spend with a verified InstantSend lock",
                Some(100),
                vec![funding(), reversible()],
                locks.clone(),
                own,
                None,
                true,
            ),
            (
                "InstantSend-locked spend",
                None,
                vec![
                    funding(),
                    spender(TransactionContext::InstantSend(lock.clone())),
                ],
                no_locks(),
                own,
                None,
                true,
            ),
            (
                "unconfirmed spend",
                None,
                vec![funding(), spender(TransactionContext::Mempool)],
                no_locks(),
                own,
                None,
                false,
            ),
            (
                "reversible spend stamped at the chain lock",
                Some(100),
                vec![funding(), reversible()],
                no_locks(),
                own,
                Some(100),
                true,
            ),
            (
                "reversible spend stamped above the chain lock",
                Some(100),
                vec![funding(), reversible()],
                no_locks(),
                own,
                Some(101),
                false,
            ),
            (
                "spend absent from history",
                Some(100),
                vec![funding()],
                no_locks(),
                foreign,
                None,
                true,
            ),
            (
                "stamped spend absent from history",
                Some(100),
                vec![funding()],
                no_locks(),
                foreign,
                Some(200),
                true,
            ),
        ] {
            let mut info = fixture.info.clone();
            info.metadata.last_applied_chain_lock = chain_locked.map(chain_lock);
            let mut store = MemoryStore::new(
                &fixture,
                info,
                RecordedHistory {
                    instant_locks,
                    ..stored_history(transactions)
                },
            );
            let mut state = store.snapshot();
            let replay = replay_wallet_history_blocking(&mut state, &evidence).unwrap();
            let claim = StoredReplayClaim {
                outpoint: fixture.spent,
                claimant: Some(claimant),
                mined_height: stamp,
            };
            store.claims = vec![claim];

            assert_eq!(
                restorable_claims(&state, &replay, &store.claims),
                if restored {
                    vec![(claim.outpoint, claim.claimant)]
                } else {
                    vec![]
                },
                "{case}"
            );
        }
    }

    /// A funding withheld from the replay or retired by it guards nothing in
    /// an account, so the claim on its coin is restored.
    #[tokio::test]
    async fn should_restore_the_claim_when_the_funding_did_not_stay_replayed() {
        let fixture = claimed_spend().await;
        let unconfirmed_funding = StoredTransaction {
            context: TransactionContext::Mempool,
            ..fixture.funding_row.clone()
        };
        let mut rival = fixture.funding.clone();
        rival.output.truncate(1);
        rival.output[0].script_pubkey = dashcore::ScriptBuf::new();
        let rival = StoredTransaction {
            txid: rival.txid(),
            transaction: rival,
            context: block(100),
            stored_net_amount: None,
            stored_direction: None,
            owned_inputs: Vec::new(),
        };
        let chain_final = TrackedAssetLock {
            operation_id: None,
            shielded_recipient: None,
            out_point: fixture.spent,
            transaction: fixture.funding.clone(),
            account_index: 0,
            funding_type: AssetLockFundingType::IdentityRegistration,
            identity_index: 0,
            amount: 0,
            status: AssetLockStatus::ChainLocked,
            proof: None,
        };
        for (case, transactions, withheld) in [
            (
                "withheld chain-final funding",
                vec![fixture.spender.clone()],
                true,
            ),
            (
                "funding beaten by a mined rival",
                vec![rival, unconfirmed_funding, fixture.spender.clone()],
                false,
            ),
        ] {
            let mut info = fixture.info.clone();
            info.metadata.last_applied_chain_lock = Some(chain_lock(100));
            let mut store = MemoryStore::new(&fixture, info, stored_history(transactions));
            if withheld {
                store.asset_locks = [(0, [(fixture.spent, chain_final.clone())].into())].into();
            }
            let mut state = store.snapshot();
            let replay =
                replay_wallet_history_blocking(&mut state, &ReplayEvidence::default()).unwrap();
            assert!(
                !replay.replayed_txids.contains(&fixture.funding.txid())
                    || replay.retired.contains(&fixture.funding.txid()),
                "{case}: the funding must not stay replayed"
            );
            let claim = StoredReplayClaim {
                outpoint: fixture.spent,
                claimant: Some(fixture.spender.txid),
                mined_height: None,
            };

            assert_eq!(
                restorable_claims(&state, &replay, &[claim]),
                vec![(claim.outpoint, claim.claimant)],
                "{case}"
            );
        }
    }

    /// A stored conflict loser is deleted in the backend transaction, and the
    /// wallet is read and replayed again to prove the history settled.
    #[tokio::test]
    async fn should_write_effects_and_settle_on_the_re_read() {
        let fixture = claimed_spend().await;
        let address = Address::from_script(
            &fixture.funding.output[0].script_pubkey,
            fixture.wallet.network,
        )
        .unwrap();
        let mut competitor = fixture.spender.transaction.clone();
        competitor.output[0].value = 98_000;
        let competitor = authorize(&fixture.wallet, &fixture.info, &address, competitor);
        let loser = competitor.txid();
        for on_write in [OnWrite::Apply, OnWrite::Ignore, OnWrite::Vanish] {
            let mut store = MemoryStore::new(
                &fixture,
                fixture.info.clone(),
                stored_history(vec![
                    fixture.funding_row.clone(),
                    fixture.spender.clone(),
                    StoredTransaction {
                        txid: loser,
                        transaction: competitor.clone(),
                        context: TransactionContext::Mempool,
                        stored_net_amount: None,
                        stored_direction: None,
                        owned_inputs: Vec::new(),
                    },
                ]),
            );
            store.on_write = on_write;

            let result = repair_wallet_history(&mut store, &ReplayEvidence::default());

            assert_eq!(store.writes, 1);
            match on_write {
                OnWrite::Apply => {
                    let (state, outcome) = result.unwrap().unwrap();
                    assert!(outcome.defeated.contains(&loser));
                    assert!(state
                        .recorded_history
                        .transactions
                        .iter()
                        .all(|stored| stored.txid != loser));
                }
                OnWrite::Ignore => assert!(matches!(
                    result,
                    Err(HistoryRepairError::Replay(
                        HistoryReplayError::UnsettledAfterReplay
                    ))
                )),
                _ => assert!(matches!(result, Err(HistoryRepairError::WalletVanished))),
            }
        }
    }
}
