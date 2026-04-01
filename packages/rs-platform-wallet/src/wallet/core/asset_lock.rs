//! Asset lock lifecycle tracking.
//!
//! Tracks asset lock transactions from broadcast through finality (IS/CL)
//! and records their usage for identity registration or top-up.

use dashcore::{Address, PrivateKey, Transaction, Txid};
use dpp::prelude::Identifier;

/// A tracked asset lock with its current lifecycle status.
#[derive(Debug, Clone)]
pub struct TrackedAssetLock {
    /// The full asset lock transaction.
    pub transaction: Transaction,
    /// Transaction ID (cached for convenience).
    pub txid: Txid,
    /// The P2PKH address of the one-time funding key in the asset lock payload.
    pub output_address: Address,
    /// The amount locked (in duffs).
    pub amount_duffs: u64,
    /// The one-time private key whose public key appears in the asset lock payload.
    pub private_key: PrivateKey,
    /// The asset lock proof, populated once IS or CL confirmation arrives.
    pub proof: Option<dpp::prelude::AssetLockProof>,
    /// The identity this lock was used for, if any.
    pub identity_id: Option<Identifier>,
    /// Current lifecycle status.
    pub status: AssetLockStatus,
}

/// Lifecycle status of an asset lock transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetLockStatus {
    /// Transaction has been broadcast but not yet confirmed.
    Broadcast,
    /// Transaction has received an InstantSend lock.
    InstantLocked,
    /// Transaction is included in a chain-locked block.
    ChainLocked,
    /// The asset lock has been consumed by an identity registration.
    UsedForRegistration,
    /// The asset lock has been consumed by an identity top-up.
    UsedForTopUp,
}

impl AssetLockStatus {
    /// Returns `true` if this asset lock has been consumed (used for registration or top-up).
    pub fn is_used(&self) -> bool {
        matches!(
            self,
            AssetLockStatus::UsedForRegistration | AssetLockStatus::UsedForTopUp
        )
    }
}
