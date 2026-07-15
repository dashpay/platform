//! Drop pre-fix proof-bearing asset-lock rows (#4133).
//!
//! Before the `AssetLockEntryWire` fix, an `AssetLockEntry` carrying
//! `proof: Some(AssetLockProof)` was written to `asset_locks.lifecycle_blob`
//! by routing the internally-tagged proof enum through `bincode::serde` —
//! a non-self-describing format that never records the field structure the
//! proof's `deserialize_any`-based decode needs on the way back. Those rows
//! are unrecoverable **by construction**: no decoder can reconstruct them, so
//! any wallet holding one already hard-fails its whole load today. This is not
//! a regression the fix introduces; it is the pre-existing defect.
//!
//! The only two statuses that can carry a `Some` proof and feed the production
//! `load_unconsumed` rehydration path are `is_locked` and `chain_locked`, so
//! the cleanup keys on the `status` column (deterministic, SQL-only) rather
//! than attempting to decode the opaque blob. `consumed` rows are already
//! SQL-filtered out of rehydration and are decode-attempted only by test-only
//! readers, so they need not be deleted.
//!
//! Deleting is safe: the asset-lock lifecycle is a chain-derived cache, not a
//! source of truth — a dropped unconsumed lock re-derives from Core on the
//! next SPV sync. A pre-fix `proof: None` row of those statuses is collateral
//! (it decodes fine today) but re-derives just the same, so scoping to the
//! status column rather than the blob content is the safe, auditable choice.
//!
//! `max_supported_version()` lifts from 3 to 4 automatically — it is derived
//! from the embedded migration list.

pub fn migration() -> String {
    "DELETE FROM asset_locks WHERE status IN ('is_locked', 'chain_locked');".to_string()
}
