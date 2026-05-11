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
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AssetLockStatus {
    Built,
    Broadcast,
    InstantSendLocked,
    ChainLocked,
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
