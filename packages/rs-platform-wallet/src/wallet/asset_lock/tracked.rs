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
