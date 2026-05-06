//! Delta of shielded-wallet state for the persister callback.
//!
//! Buffered into [`PlatformWalletChangeSet::shielded`] from the
//! `FileBackedShieldedStore` whenever a sync pass discovers a new
//! note, marks one spent, advances a per-subwallet sync watermark,
//! or records a nullifier-sync checkpoint. The host persister
//! flushes these to its durable store (SwiftData on iOS) so cold
//! starts can rehydrate the in-memory `SubwalletState` without
//! re-decrypting the chain from genesis.
//!
//! Scope:
//! - **In** this changeset: per-subwallet decrypted notes, spent
//!   marks, sync watermarks, nullifier checkpoints.
//! - **Out** of this changeset: the commitment tree itself
//!   (already persisted in `ClientPersistentCommitmentTree`'s
//!   SQLite file at the host-supplied `db_path`).

use std::collections::BTreeMap;

use crate::changeset::merge::Merge;
use crate::wallet::shielded::{ShieldedNote, SubwalletId};

/// Aggregated delta of shielded state for one persister flush.
#[derive(Debug, Clone, Default)]
pub struct ShieldedChangeSet {
    /// Notes discovered (or re-saved with updated state) per
    /// subwallet. Keyed by `(wallet_id, account_index)`. Order
    /// inside the `Vec` is insertion order — the persister can
    /// upsert by `(SubwalletId, position)`.
    pub notes_saved: BTreeMap<SubwalletId, Vec<ShieldedNote>>,
    /// Nullifiers freshly observed as spent on-chain, keyed by
    /// the subwallet that owns the corresponding note. The
    /// persister flips that note's `is_spent` row to true.
    pub nullifiers_spent: BTreeMap<SubwalletId, Vec<[u8; 32]>>,
    /// Latest per-subwallet `last_synced_note_index`. Last write
    /// wins on merge (sync only ever advances this monotonically).
    pub synced_indices: BTreeMap<SubwalletId, u64>,
    /// Latest per-subwallet `(height, timestamp)` nullifier sync
    /// checkpoint. Last write wins on merge.
    pub nullifier_checkpoints: BTreeMap<SubwalletId, (u64, u64)>,
}

impl ShieldedChangeSet {
    /// `true` iff this changeset carries no shielded deltas.
    pub fn is_empty(&self) -> bool {
        self.notes_saved.is_empty()
            && self.nullifiers_spent.is_empty()
            && self.synced_indices.is_empty()
            && self.nullifier_checkpoints.is_empty()
    }

    /// Accumulator helper: record a saved note for `id`.
    pub fn record_note(&mut self, id: SubwalletId, note: ShieldedNote) {
        self.notes_saved.entry(id).or_default().push(note);
    }

    /// Accumulator helper: record a nullifier seen as spent on `id`.
    pub fn record_nullifier_spent(&mut self, id: SubwalletId, nullifier: [u8; 32]) {
        self.nullifiers_spent.entry(id).or_default().push(nullifier);
    }

    /// Accumulator helper: advance the per-subwallet sync watermark.
    pub fn record_synced_index(&mut self, id: SubwalletId, index: u64) {
        let entry = self.synced_indices.entry(id).or_insert(index);
        if *entry < index {
            *entry = index;
        }
    }

    /// Accumulator helper: record the latest nullifier sync checkpoint.
    pub fn record_nullifier_checkpoint(&mut self, id: SubwalletId, height: u64, timestamp: u64) {
        self.nullifier_checkpoints.insert(id, (height, timestamp));
    }
}

impl Merge for ShieldedChangeSet {
    fn merge(&mut self, other: Self) {
        for (id, notes) in other.notes_saved {
            self.notes_saved.entry(id).or_default().extend(notes);
        }
        for (id, nfs) in other.nullifiers_spent {
            self.nullifiers_spent.entry(id).or_default().extend(nfs);
        }
        for (id, idx) in other.synced_indices {
            let entry = self.synced_indices.entry(id).or_insert(idx);
            if *entry < idx {
                *entry = idx;
            }
        }
        // Last write wins for nullifier checkpoints.
        self.nullifier_checkpoints
            .extend(other.nullifier_checkpoints);
    }

    fn is_empty(&self) -> bool {
        ShieldedChangeSet::is_empty(self)
    }
}
