//! Asset lock lifecycle tracking.
//!
//! Tracks asset lock transactions from build through finality (IS/CL)
//! and records their usage for identity registration or top-up.
//!
//! ## Lifecycle
//!
//! An asset lock progresses through these states:
//!
//! ```text
//! Built → Broadcast → ProofAvailable → UsedForRegistration / UsedForTopUp
//! ```
//!
//! Each state carries only the data relevant at that point in the lifecycle.
//! Transitions are performed via [`CoreWallet::advance_asset_lock`] and
//! [`CoreWallet::mark_asset_lock_used`].

use dashcore::{PrivateKey, Transaction, Txid};
use dpp::prelude::{AssetLockProof, Identifier};

/// Multi-step lifecycle for asset lock operations.
///
/// Each variant represents a distinct stage of the asset lock flow:
///
/// 1. **Built** — transaction constructed but not yet broadcast.
/// 2. **Broadcast** — transaction sent to the network, awaiting finality.
/// 3. **ProofAvailable** — IS-lock or chain-lock proof received; ready to use.
/// 4. **UsedForRegistration** — consumed by an identity registration.
/// 5. **UsedForTopUp** — consumed by an identity top-up.
#[derive(Debug, Clone)]
pub enum AssetLockLifecycle {
    /// Transaction has been built but not yet broadcast.
    Built {
        /// The full asset lock transaction.
        tx: Transaction,
        /// The one-time private key whose public key is in the asset lock payload.
        private_key: PrivateKey,
    },
    /// Transaction has been broadcast, awaiting IS-lock or chain-lock.
    Broadcast {
        /// Transaction ID.
        txid: Txid,
        /// The one-time private key for later proof usage.
        private_key: PrivateKey,
    },
    /// Finality proof (IS-lock or chain-lock) has been received.
    ProofAvailable {
        /// The finality proof suitable for identity state transitions.
        proof: AssetLockProof,
        /// The one-time private key for signing the state transition.
        private_key: PrivateKey,
        /// Transaction ID (retained for tracking / changeset generation).
        txid: Txid,
    },
    /// The asset lock was consumed by an identity registration.
    UsedForRegistration {
        /// The identity that was registered with this asset lock.
        identity_id: Identifier,
        /// Transaction ID (retained for audit / changeset).
        txid: Txid,
    },
    /// The asset lock was consumed by an identity top-up.
    UsedForTopUp {
        /// The identity that was topped up.
        identity_id: Identifier,
        /// Transaction ID (retained for audit / changeset).
        txid: Txid,
    },
}

impl AssetLockLifecycle {
    /// Returns `true` if this asset lock has been consumed (used for
    /// registration or top-up).
    pub fn is_used(&self) -> bool {
        matches!(
            self,
            AssetLockLifecycle::UsedForRegistration { .. }
                | AssetLockLifecycle::UsedForTopUp { .. }
        )
    }

    /// Returns the transaction ID for this lifecycle entry, if available.
    ///
    /// `Built` does not store a txid (the tx hasn't been broadcast yet),
    /// so this returns `None` for that variant.
    pub fn txid(&self) -> Option<&Txid> {
        match self {
            AssetLockLifecycle::Built { .. } => None,
            AssetLockLifecycle::Broadcast { txid, .. } => Some(txid),
            AssetLockLifecycle::ProofAvailable { txid, .. } => Some(txid),
            AssetLockLifecycle::UsedForRegistration { txid, .. } => Some(txid),
            AssetLockLifecycle::UsedForTopUp { txid, .. } => Some(txid),
        }
    }

    /// Returns the private key if still available (not consumed).
    pub fn private_key(&self) -> Option<&PrivateKey> {
        match self {
            AssetLockLifecycle::Built { private_key, .. } => Some(private_key),
            AssetLockLifecycle::Broadcast { private_key, .. } => Some(private_key),
            AssetLockLifecycle::ProofAvailable { private_key, .. } => Some(private_key),
            AssetLockLifecycle::UsedForRegistration { .. } => None,
            AssetLockLifecycle::UsedForTopUp { .. } => None,
        }
    }

    /// Returns the proof if available.
    pub fn proof(&self) -> Option<&AssetLockProof> {
        match self {
            AssetLockLifecycle::ProofAvailable { proof, .. } => Some(proof),
            _ => None,
        }
    }
}
