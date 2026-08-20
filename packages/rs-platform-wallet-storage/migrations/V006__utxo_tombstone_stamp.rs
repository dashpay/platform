//! Stamp sweep tombstones and pin the chainlock finality boundary.
//!
//! `core_utxos.held_since_height` is the creation stamp of an
//! unmaterialised sweep tombstone — the placeholder row `apply_sweep`
//! writes for a held input whose funding output has never classified
//! (`height IS NULL AND spent = 1`; no other writer leaves `height`
//! NULL). The stamp is the wallet's best-known processed height at the
//! round that created (or re-pointed) the claim, and exists so the
//! collector in `core_state::apply` can bound the row's lifetime the
//! way key-wallet's `prune_finalized_observed_spends` bounds the
//! equivalent in-memory entries: evict once
//! `min(chainlock_height, synced_height)` passes the stamp by the
//! sweep margin. NULL means "stamped before this column existed" —
//! the collector back-fills it with the current height rather than
//! guessing, so legacy rows wait a full margin from first sight.
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

pub fn migration() -> String {
    "ALTER TABLE core_utxos ADD COLUMN held_since_height INTEGER;
     ALTER TABLE core_sync_state ADD COLUMN chainlock_height INTEGER;
     CREATE INDEX idx_core_utxos_unmaterialized
         ON core_utxos(wallet_id, held_since_height)
         WHERE height IS NULL;"
        .to_string()
}
