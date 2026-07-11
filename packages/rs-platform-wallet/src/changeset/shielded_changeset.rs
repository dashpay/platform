//! Delta of shielded-wallet state for the persister callback.
//!
//! Buffered into [`PlatformWalletChangeSet::shielded`] from the
//! `FileBackedShieldedStore` whenever a sync pass discovers a new
//! note, marks one spent (via scan-based nullifier matching), or
//! advances a per-subwallet sync watermark. The host persister
//! flushes these to its durable store (SwiftData on iOS) so cold
//! starts can rehydrate the in-memory `SubwalletState` without
//! re-decrypting the chain from genesis.
//!
//! Scope:
//! - **In** this changeset: per-subwallet decrypted notes, spent
//!   marks, sync watermarks.
//! - **Out** of this changeset: the commitment tree itself
//!   (already persisted in `ClientPersistentCommitmentTree`'s
//!   SQLite file at the host-supplied `db_path`).

use std::collections::BTreeMap;

use crate::changeset::merge::Merge;
use crate::wallet::shielded::{
    ShieldedActivityEntry, ShieldedNote, ShieldedOutgoingNote, SubwalletId,
};

/// Aggregated delta of shielded state for one persister flush.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// Outgoing (sent) notes recovered via OVK during the scan, per
    /// subwallet. Keyed by `(wallet_id, account_index)`; the persister
    /// upserts by `(SubwalletId, cmx)` (append-only send history, no
    /// mutation).
    pub outgoing_notes: BTreeMap<SubwalletId, Vec<ShieldedOutgoingNote>>,
    /// Latest per-subwallet `last_synced_note_index`. Last write
    /// wins on merge (sync only ever advances this monotonically).
    pub synced_indices: BTreeMap<SubwalletId, u64>,
    /// Derived activity-log entries to persist, per subwallet. Keyed by
    /// `(wallet_id, account_index)`; the persister upserts by
    /// `entry.id` (sha256 of the visible output cmxs), so a `Pending`
    /// entry's later `Confirmed`/`Failed` re-emit, or a scan-derived
    /// `ShieldedSpend`'s later refinement, overwrites the existing row.
    /// Defaults empty so every pre-activity flow rides the existing
    /// changeset path unchanged.
    pub activity_entries: BTreeMap<SubwalletId, Vec<ShieldedActivityEntry>>,
}

impl ShieldedChangeSet {
    /// `true` iff this changeset carries no shielded deltas.
    pub fn is_empty(&self) -> bool {
        self.notes_saved.is_empty()
            && self.nullifiers_spent.is_empty()
            && self.outgoing_notes.is_empty()
            && self.synced_indices.is_empty()
            && self.activity_entries.is_empty()
    }

    /// Accumulator helper: record a saved note for `id`.
    pub fn record_note(&mut self, id: SubwalletId, note: ShieldedNote) {
        self.notes_saved.entry(id).or_default().push(note);
    }

    /// Accumulator helper: record a nullifier seen as spent on `id`.
    pub fn record_nullifier_spent(&mut self, id: SubwalletId, nullifier: [u8; 32]) {
        self.nullifiers_spent.entry(id).or_default().push(nullifier);
    }

    /// Accumulator helper: record an outgoing (sent) note for `id`.
    pub fn record_outgoing_note(&mut self, id: SubwalletId, note: ShieldedOutgoingNote) {
        self.outgoing_notes.entry(id).or_default().push(note);
    }

    /// Accumulator helper: record a derived activity entry for `id`.
    /// The persister upserts by `entry.id`, so re-recording the same id
    /// with a refined kind / flipped status replaces the prior row.
    pub fn record_activity_entry(&mut self, id: SubwalletId, entry: ShieldedActivityEntry) {
        self.activity_entries.entry(id).or_default().push(entry);
    }

    /// Accumulator helper: advance the per-subwallet sync watermark.
    pub fn record_synced_index(&mut self, id: SubwalletId, index: u64) {
        let entry = self.synced_indices.entry(id).or_insert(index);
        if *entry < index {
            *entry = index;
        }
    }

    /// Split a consolidated shielded changeset into one
    /// `ShieldedChangeSet` per `WalletId`. Used by the
    /// network-scoped coordinator's sync path: the free
    /// function `sync_notes_across` builds a single
    /// `ShieldedChangeSet` spanning every touched subwallet
    /// (saves, spends, and synced indices); the caller splits it
    /// here so each per-wallet `WalletPersister.store(...)` only
    /// sees its own `wallet_id`'s deltas. Empty per-wallet entries
    /// are skipped so callers don't queue no-op changesets.
    pub fn split_by_wallet_id(
        self,
    ) -> BTreeMap<crate::wallet::platform_wallet::WalletId, ShieldedChangeSet> {
        let ShieldedChangeSet {
            notes_saved,
            nullifiers_spent,
            outgoing_notes,
            synced_indices,
            activity_entries,
        } = self;
        let mut out: BTreeMap<crate::wallet::platform_wallet::WalletId, ShieldedChangeSet> =
            BTreeMap::new();
        for (id, notes) in notes_saved {
            out.entry(id.wallet_id)
                .or_default()
                .notes_saved
                .insert(id, notes);
        }
        for (id, nfs) in nullifiers_spent {
            out.entry(id.wallet_id)
                .or_default()
                .nullifiers_spent
                .insert(id, nfs);
        }
        for (id, outs) in outgoing_notes {
            out.entry(id.wallet_id)
                .or_default()
                .outgoing_notes
                .insert(id, outs);
        }
        for (id, idx) in synced_indices {
            out.entry(id.wallet_id)
                .or_default()
                .synced_indices
                .insert(id, idx);
        }
        for (id, entries) in activity_entries {
            out.entry(id.wallet_id)
                .or_default()
                .activity_entries
                .insert(id, entries);
        }
        // Defensive: drop empty entries so the persister doesn't
        // see noise. `split_by_wallet_id` is called on the result
        // of a sync pass where at least one map is non-empty
        // (otherwise the caller would have short-circuited), but
        // a future caller could legitimately split an
        // already-empty changeset.
        out.retain(|_, cs| !cs.is_empty());
        out
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
        for (id, outs) in other.outgoing_notes {
            self.outgoing_notes.entry(id).or_default().extend(outs);
        }
        for (id, idx) in other.synced_indices {
            let entry = self.synced_indices.entry(id).or_insert(idx);
            if *entry < idx {
                *entry = idx;
            }
        }
        // Activity entries append; the persister upserts by `entry.id`,
        // so a later flip/refinement of the same id (appended after the
        // original) wins at persist time without needing to dedupe here.
        for (id, entries) in other.activity_entries {
            self.activity_entries.entry(id).or_default().extend(entries);
        }
    }

    fn is_empty(&self) -> bool {
        ShieldedChangeSet::is_empty(self)
    }
}

#[cfg(test)]
mod activity_changeset_tests {
    use super::*;
    use crate::wallet::shielded::{
        ShieldedActivityEntry, ShieldedActivityKind, ShieldedActivityStatus, ShieldedDirection,
    };

    fn sub(account: u32) -> SubwalletId {
        SubwalletId::new([0xDD; 32], account)
    }

    fn entry(id: u8, status: ShieldedActivityStatus) -> ShieldedActivityEntry {
        ShieldedActivityEntry {
            id: [id; 32],
            kind: ShieldedActivityKind::Sent,
            direction: ShieldedDirection::Out,
            amount: 100,
            fee: Some(1),
            counterparty: None,
            memo: None,
            block_height: None,
            status,
            created_at_ms: 0,
            note_cmxs: vec![[id; 32]],
            spent_nullifiers: vec![],
        }
    }

    /// A default `ShieldedChangeSet` (no activity) stays empty — the new
    /// field must not perturb the old flush short-circuit.
    #[test]
    fn default_changeset_with_no_activity_is_empty() {
        let cs = ShieldedChangeSet::default();
        assert!(cs.is_empty());
        assert!(crate::changeset::merge::Merge::is_empty(&cs));
    }

    /// Recording an activity entry makes the changeset non-empty so it
    /// rides the existing flush.
    #[test]
    fn recording_activity_makes_changeset_nonempty() {
        let mut cs = ShieldedChangeSet::default();
        cs.record_activity_entry(sub(0), entry(1, ShieldedActivityStatus::Pending));
        assert!(!cs.is_empty());
        assert_eq!(cs.activity_entries.get(&sub(0)).map(|v| v.len()), Some(1));
    }

    /// Merge appends activity entries; the persister upserts by id, so a
    /// later Confirmed re-emit of the same id appears after the Pending
    /// one and wins at persist time.
    #[test]
    fn merge_appends_activity_entries_in_order() {
        let mut a = ShieldedChangeSet::default();
        a.record_activity_entry(sub(0), entry(7, ShieldedActivityStatus::Pending));
        let mut b = ShieldedChangeSet::default();
        b.record_activity_entry(sub(0), entry(7, ShieldedActivityStatus::Confirmed));

        crate::changeset::merge::Merge::merge(&mut a, b);
        let entries = a.activity_entries.get(&sub(0)).expect("entries present");
        assert_eq!(
            entries.len(),
            2,
            "merge appends both (upsert-by-id at flush)"
        );
        assert_eq!(entries[0].status, ShieldedActivityStatus::Pending);
        assert_eq!(
            entries[1].status,
            ShieldedActivityStatus::Confirmed,
            "the Confirmed re-emit lands after the Pending one"
        );
    }

    /// `split_by_wallet_id` routes activity entries to the owning wallet.
    #[test]
    fn split_by_wallet_id_routes_activity() {
        let wallet_a = [0x01; 32];
        let wallet_b = [0x02; 32];
        let mut cs = ShieldedChangeSet::default();
        cs.record_activity_entry(
            SubwalletId::new(wallet_a, 0),
            entry(1, ShieldedActivityStatus::Confirmed),
        );
        cs.record_activity_entry(
            SubwalletId::new(wallet_b, 0),
            entry(2, ShieldedActivityStatus::Confirmed),
        );

        let split = cs.split_by_wallet_id();
        assert_eq!(split.len(), 2);
        assert!(split[&wallet_a]
            .activity_entries
            .contains_key(&SubwalletId::new(wallet_a, 0)));
        assert!(split[&wallet_b]
            .activity_entries
            .contains_key(&SubwalletId::new(wallet_b, 0)));
        // No cross-leakage.
        assert!(!split[&wallet_a]
            .activity_entries
            .contains_key(&SubwalletId::new(wallet_b, 0)));
    }
}
