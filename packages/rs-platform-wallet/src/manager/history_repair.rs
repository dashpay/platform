//! Wallet-owned replay and repair of one wallet's stored history inside a backend transaction.

use std::collections::BTreeSet;

use dashcore::Txid;
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
    let chain_locked = state
        .wallet_info
        .metadata
        .last_applied_chain_lock
        .as_ref()
        .map(|lock| lock.block_height);
    let reversible =
        reversible_spends(&state, remaining.as_ref().unwrap_or(&outcome), chain_locked);
    // TODO(restore-observed-spends): a claim stamped above the chain lock whose
    // spend the history lacks stays a claim, which no fork releases; making it
    // an observed spend needs upstream `ManagedWalletInfo::restore_observed_spends`.
    let irreversible: Vec<_> = claims
        .iter()
        .filter(|claim| {
            !claim.claimant.is_some_and(|id| reversible.contains(&id))
                || claim
                    .mined_height
                    .is_some_and(|height| chain_locked.is_some_and(|locked| height <= locked))
        })
        .map(|claim| (claim.outpoint, claim.claimant))
        .collect();
    state.wallet_info.restore_spent_outpoints(&irreversible);
    Ok(Some((state, outcome)))
}

/// Replayed spends in a plain block above the restored chain lock without an InstantSend lock.
///
/// The replay recorded each of their inputs as an observed spend at the
/// block's height, which a fork below that height drops. A claim on such an
/// input would outlive the fork and keep the wallet from crediting the coin
/// again, so it is not restored.
fn reversible_spends(
    state: &ClientWalletStartState,
    replay: &HistoryReplay,
    chain_locked: Option<u32>,
) -> BTreeSet<Txid> {
    state
        .recorded_history
        .transactions
        .iter()
        .filter(|stored| match &stored.context {
            TransactionContext::InBlock(block) => {
                chain_locked.is_none_or(|locked| block.height() > locked)
            }
            _ => false,
        })
        .filter(|stored| !replay.retired.contains(&stored.txid))
        .map(|stored| stored.transaction.txid())
        .filter(|txid| {
            !replay.retired.contains(txid) && !state.wallet_info.instant_send_locks().contains(txid)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changeset::StoredTransaction;
    use crate::manager::history_replay::tests::{checked_instant_lock, confirmed_spend_fixture};
    use dashcore::bls_sig_utils::BLSSignature;
    use dashcore::ephemerealdata::chain_lock::ChainLock;
    use dashcore::{BlockHash, OutPoint, Transaction};
    use key_wallet::transaction_checking::{BlockInfo, WalletTransactionChecker};
    use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
    use key_wallet::wallet::Wallet;

    /// One wallet held in memory; a second snapshot read means effects were written.
    struct MemoryStore {
        state: Option<ClientWalletStartState>,
        claims: Vec<StoredReplayClaim>,
    }

    impl HistoryRepairStore for MemoryStore {
        fn read_wallet(&mut self) -> Result<Option<ClientWalletStartState>, PersistenceError> {
            Ok(Some(self.state.take().expect("no effects to re-read")))
        }

        fn read_claims(
            &mut self,
            _history: &mut RecordedHistory,
        ) -> Result<Vec<StoredReplayClaim>, PersistenceError> {
            Ok(self.claims.clone())
        }

        fn write_effects(
            &mut self,
            _effects: &HistoryReplayEffects,
            _operations: &HistoryReplayOperationEffects,
        ) -> Result<(), PersistenceError> {
            panic!("a settled history writes no effects")
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

    /// The fixture's spend of `spent` at height 101, its funding at 100, and
    /// the stored claim naming that spend.
    struct ClaimedSpend {
        wallet: Wallet,
        info: key_wallet::wallet::ManagedWalletInfo,
        funding: Transaction,
        spender: StoredTransaction,
        spent: OutPoint,
    }

    async fn claimed_spend() -> ClaimedSpend {
        let (wallet, info, funding, records, spent) = confirmed_spend_fixture().await;
        let spender = records
            .into_iter()
            .find(|record| record.txid != funding.txid())
            .map(|record| StoredTransaction {
                owned_inputs: Vec::new(),
                ..StoredTransaction::from(record)
            })
            .unwrap();
        assert_eq!(spender.context, block(101));
        ClaimedSpend {
            wallet,
            info,
            funding,
            spender,
            spent,
        }
    }

    /// Load `history` with one stored claim on `spent`, as a backend would.
    fn load(
        fixture: &ClaimedSpend,
        info: key_wallet::wallet::ManagedWalletInfo,
        history: Vec<StoredTransaction>,
        claim: StoredReplayClaim,
        evidence: &ReplayEvidence,
    ) -> ClientWalletStartState {
        let mut store = MemoryStore {
            state: Some(ClientWalletStartState {
                wallet: fixture.wallet.clone(),
                wallet_info: info,
                identity_manager: Default::default(),
                unused_asset_locks: Default::default(),
                unconfirmed_outgoing_txs: Vec::new(),
                recorded_history: RecordedHistory {
                    transactions: history,
                    ..Default::default()
                },
                dashpay_backfill: Default::default(),
            }),
            claims: vec![claim],
        };
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
    /// chain lock must not outlive a fork that drops that spend.
    #[tokio::test]
    async fn should_credit_funding_after_a_fork_drops_a_claimed_reversible_spend() {
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

        state.wallet_info.truncate_above(100).unwrap();

        assert!(
            credits_funding(&mut state, &fixture.funding).await,
            "the fork dropped the claimed spend, so its coin is unspent again"
        );
    }

    /// A stored claim keeps guarding the coin once finality has pruned the
    /// observed spend, whenever no fork can drop its spend or the load cannot
    /// tell.
    #[tokio::test]
    async fn should_keep_stored_claim_unless_its_spend_is_a_replayed_reversible_block() {
        let fixture = claimed_spend().await;
        let (lock, evidence) = checked_instant_lock(&fixture.spender.transaction).await;
        let foreign = Txid::from_byte_array([77; 32]);
        let spender = |context| StoredTransaction {
            context,
            ..fixture.spender.clone()
        };
        for (case, chain_locked, history, claimant, mined_height) in [
            (
                "spend in the chain-locked block",
                Some(101),
                vec![spender(block(101))],
                fixture.spender.txid,
                None,
            ),
            (
                "spend in a block below the chain lock",
                Some(150),
                vec![spender(block(101))],
                fixture.spender.txid,
                None,
            ),
            (
                "InstantSend-locked spend",
                None,
                vec![spender(TransactionContext::InstantSend(lock.clone()))],
                fixture.spender.txid,
                None,
            ),
            (
                "unconfirmed spend",
                None,
                vec![spender(TransactionContext::Mempool)],
                fixture.spender.txid,
                None,
            ),
            (
                "spend absent from history",
                Some(100),
                vec![],
                foreign,
                None,
            ),
            (
                "stamped spend absent from history",
                Some(100),
                vec![],
                foreign,
                Some(200),
            ),
        ] {
            let mut info = fixture.info.clone();
            info.metadata.last_applied_chain_lock = chain_locked.map(chain_lock);
            let claim = StoredReplayClaim {
                outpoint: fixture.spent,
                claimant: Some(claimant),
                mined_height,
            };
            let mut state = load(&fixture, info, history, claim, &evidence);
            state.wallet_info.apply_chain_lock(chain_lock(300));
            state.wallet_info.update_synced_height(300);

            assert!(
                !credits_funding(&mut state, &fixture.funding).await,
                "{case}: the stored claim must keep the spent coin from being credited"
            );
        }
    }
}
