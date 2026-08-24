//! Anchor sweep tombstones at their winner's mined height and pin the
//! chainlock finality boundary.
//!
//! `core_utxos.winner_mined_height` is the mined height of the
//! transaction that beat an unmaterialised sweep tombstone's outpoint —
//! the placeholder row `apply_sweep` writes for a held input whose
//! funding output has never classified (`height IS NULL AND spent = 1`;
//! no other writer leaves `height` NULL). The height is carried on the
//! sweep event itself (`TransactionsSwept::winner_mined_height`), and
//! `apply_sweep` writes the placeholder for EVERY non-released held
//! input in EVERY sweep context — only the stamp differs. A
//! block-context sweep (winner actually mined) stamps the winner's
//! height, and the collector in `core_state::apply` evicts the row
//! exactly when `min(chainlock_height, synced_height)` reaches that
//! height — `prune_finalized_observed_spends`' condition verbatim, with
//! no observation-age margin. An InstantSend-locked, unmined winner
//! writes the same row with the stamp NULL — under DIP-10 its lock
//! alone settles the input, but it carries no height to key a lifetime
//! on — and the collector never takes an unstamped row: it resolves
//! only through proof, when the funding upsert materialises it, a later
//! block-context sweep re-stamps it into the collectible set, or a
//! release deletes it. See `CORE_SWEEP_REMOVAL` and the `apply_sweep`
//! doc in `core_state.rs` for why an unstamped hold must survive (it is
//! the only durable carrier of upstream's in-memory `spent_outpoints`
//! hold across a restart) and what bounds the foreign-input residue.
//!
//! `core_sync_state.chainlock_height` is the monotonic-max height of
//! the last applied chainlock, mirrored from
//! `CoreChangeSet::last_applied_chain_lock` (previously dropped by
//! this store). It is one half of the collector's finality boundary;
//! rows are never collected before a chainlock has been persisted,
//! matching upstream's "no-op until a chainlock has been applied".
//!
//! The partial index covers exactly the unmaterialised rows — the
//! collector's scan set is the stamped subset of these — so the
//! per-round sweep touches tombstones only, not the wallet's full
//! spent history.
//!
//! Edited in place (formerly `V006__utxo_tombstone_stamp`, column
//! `held_since_height`) under the same pre-release policy V001's test
//! documents: nothing shipped has applied this migration, and a dev
//! database that did apply the old shape fails refinery's divergence
//! check and must be recreated. Renumbered `V006` → `V007` when the
//! mainline's `V006__tracked_masternodes` merged in ahead of this
//! unmerged branch: version numbers, like capability bits, are
//! append-only and the already-merged assignment keeps its slot.

pub fn migration() -> String {
    "ALTER TABLE core_utxos ADD COLUMN winner_mined_height INTEGER;
     ALTER TABLE core_sync_state ADD COLUMN chainlock_height INTEGER;
     CREATE INDEX idx_core_utxos_unmaterialized
         ON core_utxos(wallet_id, winner_mined_height)
         WHERE height IS NULL;"
        .to_string()
}
