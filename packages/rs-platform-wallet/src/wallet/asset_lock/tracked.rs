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

use crate::changeset::{AssetLockEntry, ShieldedRecipientRaw};
use dpp::prelude::Identifier;

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
/// The authoritative SQLite `lifecycle_blob` stores this enum; never reorder
/// existing variants. The `asset_locks.status` TEXT column is a query projection
/// (see `migrations/V001__initial.rs`). `DrainCompleted` projects to `consumed`
/// while its distinct appended blob discriminant retains the proof receipt.
/// Changes MUST update `sqlite::schema::asset_locks::status_str` and its parity
/// test; a new SQL label also requires `ASSET_LOCK_STATUS_LABELS` and a migration.
/// Verified by the `asset_lock_status_labels_match_enum` unit test in
/// that same module.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AssetLockStatus {
    Built,
    /// A broadcast attempt was not definitely rejected before dispatch. The
    /// status does not assert that a peer or the network accepted it.
    Broadcast,
    InstantSendLocked,
    ChainLocked,
    /// Successfully consumed by an identity registration / top-up.
    /// Terminal state. The entry persists for historical lookup
    /// (amount, identity index, funding tx) but is excluded from any
    /// "still actionable" predicate.
    Consumed,
    /// Core finality is authenticated, but Platform-side consumption is
    /// unknown. Most entries reach this state after reconstruction from a
    /// **chain-locked** on-chain record rather than live tracking through the
    /// build → broadcast → proof pipeline. It is also the safe state after an
    /// explicit retry receives an unauthenticated "already consumed" report:
    /// the wallet first obtains a ChainLock proof, retains it, and records no
    /// claim that Platform actually consumed the output.
    ///
    /// The restore-scan path (`wallet::asset_lock::sync::reconstruction`)
    /// emits this for finalized asset-lock transactions whose credit outputs
    /// pay this wallet's funding accounts. Non-final detections (mempool /
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
    /// A wallet drain whose Type18 Platform executed, or whose resumed Type18
    /// Platform rejected because this exact outpoint was already consumed by
    /// an earlier attempt. Written only by `complete_wallet_drain`; terminal
    /// and never regressed. Unlike generic `Consumed`, this is a durable
    /// drain-completion receipt.
    DrainCompleted,
}

impl AssetLockStatus {
    /// Terminal for generic asset-lock lifecycle and coin-selection purposes.
    /// Drain success requires `DrainCompleted`, not generic `Consumed`.
    pub fn is_consumed(&self) -> bool {
        matches!(self, Self::Consumed | Self::DrainCompleted)
    }
}

/// A tracked asset lock. Private keys are NOT stored here — they're
/// re-derived from funding_type + identity_index via key-wallet's Wallet.
#[derive(Debug, Clone)]
pub struct TrackedAssetLock {
    /// Stable host operation identifier, retained through consumption and restart.
    pub operation_id: Option<[u8; 16]>,
    /// The outpoint identifying this credit output (txid + vout).
    pub out_point: OutPoint,
    pub transaction: Transaction,
    /// Index the funding sources were resolved at — the standard
    /// (BIP44/BIP32) family index handed to the build. NOT a record of
    /// which accounts ended up supplying inputs: funding is pooled, so a
    /// lock can be funded wholly out of a DashPay contact account, which
    /// carries an index of its own. Consumers that need the funding
    /// transaction search every family (see `funding_tx_record`).
    pub account_index: u32,
    pub funding_type: AssetLockFundingType,
    pub identity_index: u32,
    pub amount: u64,
    pub status: AssetLockStatus,
    /// The proof, available once IS-locked or ChainLocked.
    pub proof: Option<AssetLockProof>,
    /// The Orchard recipient a shielded funding (Type 18) was built to pay,
    /// recorded with the row before the lock is dispatched and immutable
    /// afterwards, so a resume pays the recipient the user confirmed rather
    /// than whatever the resuming call passes. `None` for every other
    /// funding type and for shielded locks tracked before the recipient was
    /// recorded (those keep accepting the resuming caller's recipient).
    pub shielded_recipient: Option<ShieldedRecipientRaw>,
    /// The identity an unbound identity top-up (`IdentityTopUpNotBound`) was
    /// built to pay, recorded with the row before the lock is dispatched, or
    /// assigned later by the user to a lock that had none; immutable once
    /// set. A resume pays this identity and refuses any other. `None` for
    /// every other funding type, and for an unbound lock nothing recorded a
    /// recipient for (one rebuilt from the chain after a restore): such a
    /// lock is unassigned (see [`Self::is_unassigned_identity_top_up`]) and
    /// is resumable only once a recipient is assigned, since its credit
    /// output names no identity and it may have been meant for someone else.
    pub identity_recipient: Option<Identifier>,
}

impl TrackedAssetLock {
    /// An unbound identity top-up with no recorded recipient, not yet
    /// consumed: its funds are recoverable (the wallet holds the credit
    /// output's key), but the identity it was meant to pay is unknown, so no
    /// resume pays it until the user assigns one.
    pub fn is_unassigned_identity_top_up(&self) -> bool {
        self.funding_type == AssetLockFundingType::IdentityTopUpNotBound
            && self.identity_recipient.is_none()
            && !self.status.is_consumed()
    }
}

impl From<&TrackedAssetLock> for AssetLockEntry {
    fn from(lock: &TrackedAssetLock) -> Self {
        Self {
            operation_id: lock.operation_id,
            out_point: lock.out_point,
            transaction: lock.transaction.clone(),
            account_index: lock.account_index,
            funding_type: lock.funding_type,
            identity_index: lock.identity_index,
            amount_duffs: lock.amount,
            status: lock.status.clone(),
            proof: lock.proof.clone(),
            shielded_recipient: lock.shielded_recipient,
            identity_recipient: lock.identity_recipient,
        }
    }
}
