//! Asset lock tracking.
//!
//! Tracks asset lock transactions from build through finality (IS/CL).
//! Once consumed by a successful identity operation, the lock is removed.
//!
//! Private keys are NOT stored here — they are re-derived from
//! `funding_type` + `identity_index` via the key-wallet's `Wallet`.
/// TODO: Shall we move to state module
use dashcore::{OutPoint, Transaction};
use dpp::prelude::AssetLockProof;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::changeset::AssetLockEntry;

/// Asset lock status on Core chain. Tracked until consumed, then removed.
///
/// Lifecycle: `Built` → `Broadcast` → either `InstantSendLocked` (fast
/// path, IS proof available) and/or `ChainLocked` (final fallback).
#[derive(Debug, Clone, PartialEq)]
pub enum AssetLockStatus {
    /// Transaction has been built and signed but not yet broadcast.
    Built,
    /// Transaction has been pushed to the network; awaiting finality.
    Broadcast,
    /// An InstantSend lock has been observed — usable as an asset-lock
    /// proof immediately.
    InstantSendLocked,
    /// The transaction is included in a ChainLocked block — usable as a
    /// fallback proof when the IS lock isn't available or has aged out.
    ChainLocked,
}

/// A tracked asset lock. Private keys are NOT stored here — they're
/// re-derived from funding_type + identity_index via key-wallet's Wallet.
#[derive(Debug, Clone)]
pub struct TrackedAssetLock {
    /// The outpoint identifying this credit output (txid + vout).
    pub out_point: OutPoint,
    /// The full asset-lock transaction. Kept around so the proof
    /// builder and recovery paths can re-derive outputs without a
    /// network round trip.
    pub transaction: Transaction,
    /// BIP44 account index that funded this asset lock (UTXO source).
    pub account_index: u32,
    /// Whether this lock was built for identity registration or a
    /// top-up of an existing identity (drives the derivation path the
    /// signer uses to redeem it).
    pub funding_type: AssetLockFundingType,
    /// HD identity index this lock targets (BIP-9 inner key).
    pub identity_index: u32,
    /// Locked amount, in duffs.
    pub amount: u64,
    /// Current status on the Core chain; see [`AssetLockStatus`].
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
