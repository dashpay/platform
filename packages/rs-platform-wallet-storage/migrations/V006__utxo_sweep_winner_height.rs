//! Anchor sweep tombstones at their winner's mined height and pin the
//! chainlock finality boundary.
//!
//! `core_utxos.winner_mined_height` is the mined height of the
//! transaction that beat an unmaterialised sweep tombstone's outpoint —
//! the placeholder row `apply_sweep` writes for a held input whose
//! funding output has never classified (`height IS NULL AND spent = 1`;
//! no other writer leaves `height` NULL). The height is carried on the
//! sweep event itself (`TransactionsSwept::winner_mined_height`), and
//! only a block-context sweep — one whose winner actually mined — writes
//! a tombstone at all; an InstantSend-locked winner has no mining
//! deadline and leaves no placeholder, mirroring key-wallet's
//! `observed_spent_outpoints` doctrine ("an unconfirmed spend must not
//! invalidate a coin"). The collector in `core_state::apply` evicts a
//! tombstone exactly when `min(chainlock_height, synced_height)` reaches
//! its winner's height — `prune_finalized_observed_spends`' condition
//! verbatim, with no observation-age margin. NULL is never written by
//! current code; an unstamped held row is held forever rather than
//! guessed collectible.
//!
//! `core_sync_state.chainlock_height` is the monotonic-max height of
//! the last applied chainlock, mirrored from
//! `CoreChangeSet::last_applied_chain_lock` (previously dropped by
//! this store). It is one half of the collector's finality boundary;
//! rows are never collected before a chainlock has been persisted,
//! matching upstream's "no-op until a chainlock has been applied".
//!
//! The partial index covers exactly the unmaterialised rows — the
//! collector's scan set — so the per-round sweep touches tombstones
//! only, not the wallet's full spent history.
//!
//! Edited in place (formerly `V006__utxo_tombstone_stamp`, column
//! `held_since_height`) under the same pre-release policy V001's test
//! documents: nothing shipped has applied this migration, and a dev
//! database that did apply the old shape fails refinery's divergence
//! check and must be recreated.

pub fn migration() -> String {
    "ALTER TABLE core_utxos ADD COLUMN winner_mined_height INTEGER;
     ALTER TABLE core_sync_state ADD COLUMN chainlock_height INTEGER;
     CREATE INDEX idx_core_utxos_unmaterialized
         ON core_utxos(wallet_id, winner_mined_height)
         WHERE height IS NULL;"
        .to_string()
}
