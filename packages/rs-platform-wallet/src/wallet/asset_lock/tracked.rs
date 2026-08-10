//! Asset lock tracking.
//!
//! Tracks asset lock transactions from build through finality (IS/CL).
//! Once consumed by a successful identity operation, the lock remains as a
//! terminal tombstone so exact-outpoint retries stay distinguishable from
//! foreign or never-tracked locks, including after a wallet restart.
//!
//! Private keys are NOT stored here — they are re-derived from
//! `funding_type` + `identity_index` via the key-wallet's `Wallet`.

use dashcore::{OutPoint, Transaction};
use dpp::prelude::AssetLockProof;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::changeset::AssetLockEntry;

/// Asset lock status on Core chain.
///
/// The wallet tracks asset locks from build through consumption. The
/// terminal [`Consumed`](Self::Consumed) state is kept (rather than
/// removed from storage) so historical UI lookups — e.g. the
/// "Transactions" list mapping a funding tx to its locked amount —
/// still resolve after the identity registration / top-up succeeded.
/// The catch-up scanner and any "ready to fund" UI filter on
/// `< InstantSendLocked` so post-Consumed entries don't generate
/// noise.
///
/// NOT `#[non_exhaustive]` by design: every cross-crate match site
/// (FFI, accessors) uses exhaustive arms intentionally so the
/// compiler catches a new variant addition the way `Consumed` was
/// caught — at every status_to_u8 / status_from_u8 / serializer call.
/// Marking the enum `#[non_exhaustive]` would force wildcard arms
/// and silently lose that signal.
///
/// # Schema coupling
///
/// Variants of this enum are persisted as TEXT in the
/// `platform-wallet-storage` SQLite schema (column `asset_locks.status`
/// — see `migrations/V001__initial.rs`). Any change to this enum
/// (added or renamed variant) MUST also update:
///   - `sqlite::schema::asset_locks::status_str` (writer mapping)
///   - `sqlite::schema::asset_locks::ASSET_LOCK_STATUS_LABELS`
///     (`CHECK` constraint domain)
/// Verified by the `asset_lock_status_labels_match_enum` unit test in
/// that same module.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AssetLockStatus {
    Built,
    Broadcast,
    InstantSendLocked,
    ChainLocked,
    /// Successfully consumed by an identity registration / top-up.
    /// Terminal state. The entry persists for historical lookup
    /// (amount, identity index, funding tx) but is excluded from any
    /// "still actionable" predicate.
    Consumed,
    /// Reconstructed from a **chain-locked** on-chain record rather
    /// than tracked live through the build → broadcast → proof
    /// pipeline — the restore-scan path
    /// (`wallet::asset_lock::sync::reconstruction`) emits this for
    /// finalized asset-lock transactions whose credit outputs pay this
    /// wallet's funding accounts. Non-final detections (mempool /
    /// unconfirmed-block sightings) enter as
    /// [`Broadcast`](Self::Broadcast) /
    /// [`InstantSendLocked`](Self::InstantSendLocked) like any other
    /// pre-finality lock, and are upgraded to this status when a later
    /// record — or the chainlock promotion that buries their block —
    /// proves finality while nothing live is completing them
    /// (`enrich_from_record`).
    ///
    /// # Lifecycle vs live flows
    ///
    /// "Nothing live is completing them" is enforced structurally, not
    /// by a provenance flag: enrichment only touches proof-less
    /// entries, and a lock a live flow drives leaves the proof-less
    /// window as soon as its proof resolves (`wait_for_proof` →
    /// `advance_asset_lock_status`, which overwrites status + proof
    /// unconditionally, and `resume_asset_lock`, which advances from
    /// its step-1 status snapshot). In the one race where a chainlock
    /// promotion reaches a still-waiting live lock first, the live
    /// pipeline's own write lands moments later and wins — the
    /// transient recovery classification never sticks (regression:
    /// `live_flow_advance_overwrites_chain_lock_recovery_classification`).
    /// The only transition OUT of this status is therefore a live
    /// writer: an explicit resume completing the spend (`Consumed` via
    /// `consume_asset_lock`) or re-driving the proof pipeline; a
    /// resume that proves nothing new keeps this status
    /// (`resume_asset_lock` preserves it rather than re-entering the
    /// pending window).
    ///
    /// Core-side finality is therefore guaranteed (a
    /// `ChainAssetLockProof` from the record's height is attached at
    /// creation), but **Platform-side consumption is unknown**: the
    /// lock may have long since funded an identity / address top-up,
    /// or it may be genuinely unspent stranded value. Neither
    /// `ChainLocked` (which UIs read as "in flight") nor `Consumed`
    /// (which claims success) would be truthful, so this is its own
    /// state, excluded from both the pending and the consumed
    /// predicates. An explicit `resume_asset_lock` may consume it —
    /// Platform is the arbiter and rejects an already-spent outpoint
    /// with a typed error.
    RecoveredFromChain,
}

impl AssetLockStatus {
    /// Position of this variant in the asset lock's forward-only
    /// lifecycle, used to reject a delayed writer's *backward* status
    /// write (see
    /// [`advance_asset_lock_status`](crate::wallet::asset_lock::manager::AssetLockManager::advance_asset_lock_status)).
    ///
    /// A lock only ever moves forward: it is built, broadcast, covered
    /// by an InstantSend lock, then by a ChainLock, then consumed. But
    /// the two proof-bearing stages are produced by *independent*
    /// waiters — an IS-lock arrives from one SPV event and a ChainLock
    /// from another — so nothing about the call order guarantees the
    /// stronger one is written last. A delayed
    /// `InstantSendLocked + IS proof` write landing after a
    /// `ChainLocked + CL proof` one would replace strictly better
    /// evidence with weaker evidence, in memory and durably. Comparing
    /// ranks is what lets the write be refused.
    ///
    /// Ranks are assigned by an exhaustive match on named variants
    /// rather than by `as u8` on the declaration order. The two agree
    /// today, but the enum's order is also load-bearing for the FFI
    /// discriminants (`status_from_u8`) and the SQLite label domain,
    /// and this ordering is a *semantic* claim about the lifecycle —
    /// tying it to declaration position would let a future reordering
    /// silently redefine which writes count as downgrades. The match
    /// has no `_` arm, so adding a variant is a compile error here, the
    /// same signal the enum's `#[non_exhaustive]` note describes.
    ///
    /// `Consumed` ranks highest: it is terminal, and its entry is
    /// dropped from `tracked_asset_locks` outright (so in practice an
    /// advance against it fails the tracked-row lookup first).
    pub(crate) fn lifecycle_rank(&self) -> u8 {
        match self {
            AssetLockStatus::Built => 0,
            AssetLockStatus::Broadcast => 1,
            AssetLockStatus::InstantSendLocked => 2,
            AssetLockStatus::ChainLocked => 3,
            AssetLockStatus::Consumed => 4,
        }
    }
}

/// A tracked asset lock. Private keys are NOT stored here — they're
/// re-derived from funding_type + identity_index via key-wallet's Wallet.
#[derive(Debug, Clone)]
pub struct TrackedAssetLock {
    /// The outpoint identifying this credit output (txid + vout).
    pub out_point: OutPoint,
    pub transaction: Transaction,
    /// BIP44 account index that funded this asset lock (UTXO source).
    pub account_index: u32,
    pub funding_type: AssetLockFundingType,
    pub identity_index: u32,
    pub amount: u64,
    pub status: AssetLockStatus,
    /// The proof, available once IS-locked or ChainLocked.
    pub proof: Option<AssetLockProof>,
}

impl From<&TrackedAssetLock> for AssetLockEntry {
    fn from(lock: &TrackedAssetLock) -> Self {
        Self {
            out_point: lock.out_point,
            transaction: lock.transaction.clone(),
            account_index: lock.account_index,
            funding_type: lock.funding_type,
            identity_index: lock.identity_index,
            amount_duffs: lock.amount,
            status: lock.status.clone(),
            proof: lock.proof.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AssetLockStatus;

    /// The lifecycle order the monotonicity guard enforces, written out
    /// in full. Pinned by value rather than derived from the enum so a
    /// reordering of the declaration (which the FFI discriminants and
    /// the SQLite label domain also depend on) cannot silently redefine
    /// which writes count as downgrades.
    #[test]
    fn lifecycle_rank_pins_the_forward_only_order() {
        assert_eq!(AssetLockStatus::Built.lifecycle_rank(), 0);
        assert_eq!(AssetLockStatus::Broadcast.lifecycle_rank(), 1);
        assert_eq!(AssetLockStatus::InstantSendLocked.lifecycle_rank(), 2);
        assert_eq!(AssetLockStatus::ChainLocked.lifecycle_rank(), 3);
        assert_eq!(AssetLockStatus::Consumed.lifecycle_rank(), 4);
    }

    /// Every variant is strictly ordered against every other, and the
    /// single comparison the guard actually turns on — a ChainLocked row
    /// outranking a late InstantSendLocked write — holds.
    #[test]
    fn lifecycle_rank_is_strictly_increasing_across_the_lifecycle() {
        let lifecycle = [
            AssetLockStatus::Built,
            AssetLockStatus::Broadcast,
            AssetLockStatus::InstantSendLocked,
            AssetLockStatus::ChainLocked,
            AssetLockStatus::Consumed,
        ];
        for pair in lifecycle.windows(2) {
            assert!(
                pair[0].lifecycle_rank() < pair[1].lifecycle_rank(),
                "{:?} must rank strictly below {:?}",
                pair[0],
                pair[1]
            );
        }

        assert!(
            AssetLockStatus::InstantSendLocked.lifecycle_rank()
                < AssetLockStatus::ChainLocked.lifecycle_rank(),
            "an InstantSendLocked write arriving after a ChainLocked one must \
             be recognizable as a downgrade — this is the comparison the \
             monotonicity guard in `advance_asset_lock_status` turns on"
        );
    }
}
