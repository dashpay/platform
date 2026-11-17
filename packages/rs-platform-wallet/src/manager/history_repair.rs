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
/// releases: the coin's funding and its claimant were both replayed, the
/// claimant in a plain block above the restored chain lock and without an
/// InstantSend lock, and the claim was not stamped at or below the chain lock.
/// The claimant's account then records the spend of the coin, and a fork
/// dropping the claimant gives the coin back; a claim would keep it spent.
/// Without the funding the spend is only an observed spend, which finality
/// prunes, so the claim stays.
fn restorable_claims(
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
    let replayed: Vec<_> = state
        .recorded_history
        .transactions
        .iter()
        .filter(|stored| {
            let body = stored.transaction.txid();
            !replay.retired.contains(&stored.txid) && !replay.retired.contains(&body)
        })
        .collect();
    let funded: BTreeSet<Txid> = replayed
        .iter()
        .map(|stored| stored.transaction.txid())
        .collect();
    let reversible: BTreeSet<Txid> = replayed
        .iter()
        .filter(|stored| match &stored.context {
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
        authorize, checked_instant_lock, confirmed_spend_fixture,
    };
    use dashcore::bls_sig_utils::BLSSignature;
    use dashcore::ephemerealdata::chain_lock::ChainLock;
    use dashcore::{Address, BlockHash, Transaction};
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
                unused_asset_locks: Default::default(),
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
                true,
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
