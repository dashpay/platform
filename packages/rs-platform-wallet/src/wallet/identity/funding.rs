//! Funding method enums for identity registration and top-up.
//!
//! These enums describe *how* an identity operation is funded, decoupling the
//! funding source from the identity lifecycle logic.
//!
//! ## Type overview
//!
//! * [`IdentityFunding`] — unified funding enum used by the new
//!   `create_funded_asset_lock_proof` flow. Covers wallet-balance and
//!   pre-existing asset locks.
//! * [`IdentityFundingMethod`] / [`TopUpFundingMethod`] — original per-operation
//!   enums consumed by `register_identity_with_funding` and
//!   `top_up_identity_with_funding`. Retained for backwards compatibility.

use dashcore::{OutPoint, PrivateKey};
use dpp::prelude::AssetLockProof;

// ─── Unified funding enum ────────────────────────────────────────────────────

/// How to fund an identity operation (registration, top-up, etc.).
///
/// This is the *unified* enum consumed by
/// [`CoreWallet::create_funded_asset_lock_proof`](crate::wallet::core::CoreWallet::create_funded_asset_lock_proof).
/// It replaces the earlier pattern of having separate funding enums per
/// operation type.
#[derive(Debug, Clone)]
pub enum IdentityFunding {
    /// Build an asset lock from wallet UTXOs for the given amount.
    FromWalletBalance {
        /// Amount to lock (in duffs).
        amount_duffs: u64,
    },
    /// Resume from a tracked asset lock identified by its outpoint (txid + output index).
    ///
    /// The asset lock must already be tracked by the [`AssetLockManager`].
    /// The manager will resume from whatever stage the lock is at (built,
    /// broadcast, IS-locked, or chain-locked) and re-derive the private key.
    FromExistingAssetLock {
        /// The outpoint identifying the tracked asset lock (txid + output index).
        out_point: OutPoint,
    },
}

// ─── Per-operation funding enums (original API) ──────────────────────────────

/// Funding method for identity registration.
pub enum IdentityFundingMethod {
    /// Use a pre-existing asset lock proof (e.g. one tracked by
    /// [`CoreWallet::tracked_asset_locks`]).
    UseAssetLock {
        /// The asset lock proof (IS or CL).
        proof: AssetLockProof,
        /// The one-time private key from the asset lock payload.
        private_key: PrivateKey,
    },
    /// Build an asset lock from wallet UTXOs for the given amount (in duffs).
    ///
    /// This is the default path used by the convenience wrapper.
    FundWithWallet {
        /// Amount to lock (in duffs).
        amount_duffs: u64,
    },
    // NOTE: FundFromAddresses (platform address funding, no asset lock) is
    // intentionally omitted for now. It requires a different state transition
    // type (`IdentityCreateFromAddressesTransition`) and a different signer
    // (`Signer<PlatformAddress>`), making it a substantially different code
    // path. It can be added in a follow-up PR.
}

/// Funding method for identity top-up.
pub enum TopUpFundingMethod {
    /// Use a pre-existing asset lock proof.
    UseAssetLock {
        /// The asset lock proof (IS or CL).
        proof: AssetLockProof,
        /// The one-time private key from the asset lock payload.
        private_key: PrivateKey,
    },
    /// Build an asset lock from wallet UTXOs for the given amount (in duffs).
    ///
    /// This is the default path used by the convenience wrapper.
    FundWithWallet {
        /// Amount to lock (in duffs).
        amount_duffs: u64,
    },
}
