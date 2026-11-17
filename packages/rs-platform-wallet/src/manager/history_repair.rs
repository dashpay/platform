//! Wallet-owned replay and repair of one wallet's stored history inside a backend transaction.

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
        let remaining = replay_wallet_history_blocking(&mut state, evidence)?;
        if !remaining.sweeps.is_empty()
            || !remaining.authorization.rejected.is_empty()
            || !remaining.withheld_asset_lock_inputs.is_empty()
        {
            return Err(HistoryReplayError::UnsettledAfterReplay.into());
        }
        state.unconfirmed_outgoing_txs =
            outcome.resendable(std::mem::take(&mut state.unconfirmed_outgoing_txs));
    }
    state.wallet_info.restore_spent_outpoints(
        &claims
            .iter()
            .map(|claim| (claim.outpoint, claim.claimant))
            .collect::<Vec<_>>(),
    );
    Ok(Some((state, outcome)))
}
