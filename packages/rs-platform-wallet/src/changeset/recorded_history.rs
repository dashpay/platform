//! Stored transaction history a persister hands back at load.
//!
//! The replay that consumes it lives in
//! [`history_replay`](crate::manager::history_replay); this module only
//! carries the persister-agnostic input shape.

use std::collections::BTreeMap;

use dashcore::ephemerealdata::instant_lock::InstantLock;
use dashcore::{Transaction, Txid};
use key_wallet::managed_account::transaction_record::{
    InputDetail, TransactionDirection, TransactionRecord,
};
use key_wallet::transaction_checking::TransactionContext;

/// Every stored transaction of one wallet, replayed at load to rebuild spend guards.
///
/// Persisted UTXO rows say which outputs are unspent, but not which outpoints
/// the wallet has seen spent. Without that in-memory state a redelivered
/// funding transaction (rescan, gap-limit rediscovery) re-credits an output
/// that a confirmed spend already consumed. Replaying the stored history
/// through the wallet checker rebuilds it; the persisted UTXO set stays
/// authoritative for which outputs are credited.
///
/// Empty means "no history supplied": load keeps the projection as restored.
#[derive(Debug, Clone, Default)]
pub struct RecordedHistory {
    /// Stored transactions, in any order; the replay orders them.
    pub transactions: Vec<StoredTransaction>,
    /// Persisted InstantSend locks keyed by the txid they lock. A lock that
    /// arrived after its transaction was stored never rewrote the stored
    /// context, so the replay upgrades a matching mempool record itself.
    pub instant_locks: BTreeMap<Txid, InstantLock>,
}

impl RecordedHistory {
    /// Whether the persister supplied no history at all.
    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }
}

/// One stored transaction as its persister keeps it.
#[derive(Debug, Clone)]
pub struct StoredTransaction {
    /// The txid the store keys this row under. Kept apart from
    /// `transaction` because the two are not proof of each other: a lock is
    /// only trusted when both agree with it.
    pub txid: Txid,
    /// The consensus transaction body.
    pub transaction: Transaction,
    /// The stored confirmation context.
    pub context: TransactionContext,
    /// The wallet-level net amount the store holds, when it keeps one.
    pub stored_net_amount: Option<i64>,
    /// The wallet-level direction the store holds, when it keeps one.
    pub stored_direction: Option<TransactionDirection>,
    /// The wallet-owned inputs the store recorded for this transaction.
    ///
    /// Load excludes spent outputs, so a spend whose funding survives only as
    /// a height row replays with no owned input; these let the replay stage
    /// that input and rebuild its spent mark. Empty when the store keeps no
    /// per-input ownership: that spend then rebuilds no guard.
    pub owned_inputs: Vec<InputDetail>,
}

impl From<TransactionRecord> for StoredTransaction {
    fn from(record: TransactionRecord) -> Self {
        Self {
            txid: record.txid,
            stored_net_amount: Some(record.net_amount),
            stored_direction: Some(record.direction),
            owned_inputs: record.input_details,
            transaction: record.transaction,
            context: record.context,
        }
    }
}
