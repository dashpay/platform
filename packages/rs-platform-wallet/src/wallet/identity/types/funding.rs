//! Funding source enum for identity registration and top-up.
//!
//! The single source of funding for any identity lifecycle operation
//! (register, top up) is [`IdentityFunding`]. The funded-but-not-yet-
//! consumed asset lock is the central concept — every variant ends up
//! resolved to `(AssetLockProof, DerivationPath)` before submission to
//! Platform.
//!
//! ## Historical note
//!
//! Earlier iterations carried two parallel funding enums
//! (`IdentityFundingMethod` / `TopUpFundingMethod`) consumed by
//! per-operation helpers. They were merged into [`IdentityFunding`]
//! once the registration and top-up high-level helpers grew identical
//! funding-resolution + IS→CL fallback shapes — at which point the
//! per-operation enums were dead weight. The merge happened in iter
//! 4 of the swift-funding-with-asset-lock series.

use dashcore::OutPoint;

/// How to fund an identity operation (registration, top-up).
///
/// Resolved by the high-level `register_identity_with_funding` /
/// `top_up_identity_with_funding` helpers into an
/// `(AssetLockProof, DerivationPath, OutPoint)` triple that the
/// `_with_signer` SDK methods can consume. The `OutPoint` is retained
/// for cleanup (so the tracked-asset-lock row can be removed on
/// success) and for IS→CL fallback (so the consumed lock can be
/// looked up by outpoint when the IS proof times out or is rejected).
///
/// Every variant produces a lock tracked by this wallet's
/// [`AssetLockManager`](crate::wallet::asset_lock::manager::AssetLockManager).
/// The IS→CL fallback paths (300s IS-timeout in the resolver, Platform
/// IS-rejection retry in the submission layer) require the lock to be
/// tracked so they can look it up by outpoint and drive the wait. An
/// earlier variant (`UseAssetLock`) accepted an externally-built proof
/// and skipped tracking — it broke the IS→CL fallback unrecoverably
/// because the lock was invisible to `upgrade_to_chain_lock_proof`
/// (which short-circuits with `Asset lock {} is not tracked`). The
/// variant was removed; future callers that hold an external proof
/// should register it through `AssetLockManager` first, then use
/// `FromExistingAssetLock`.
#[derive(Debug, Clone)]
pub enum IdentityFunding {
    /// Build an asset lock from wallet UTXOs for the given amount.
    ///
    /// The helper picks the appropriate funding account
    /// (`identity_registration` for register, `identity_topup` for top
    /// up), builds the asset-lock tx, broadcasts it, waits for an
    /// IS-lock proof, and falls back to ChainLock if the IS-lock times
    /// out (300s) or is rejected at Platform.
    ///
    /// `account_index` selects which BIP44 *standard* account (by
    /// BIP44 account index) supplies the UTXOs. Only BIP44 standard
    /// accounts are supported today — CoinJoin / BIP32 funding for
    /// identity registration is out of scope and would require
    /// additional plumbing in `create_funded_asset_lock_proof`.
    FromWalletBalance {
        /// Amount to lock (in duffs).
        amount_duffs: u64,
        /// BIP44 standard-account index to draw the funding UTXOs from.
        ///
        /// Only BIP44 standard accounts (`AccountType::Standard` with
        /// `StandardAccountTypeTag::Bip44`) are supported today;
        /// CoinJoin / BIP32 are not.
        account_index: u32,
    },

    /// Resume from a tracked asset lock identified by its outpoint
    /// (txid + output index).
    ///
    /// The asset lock must already be tracked by the
    /// [`AssetLockManager`](crate::wallet::asset_lock::manager::AssetLockManager).
    /// The manager resumes from whatever stage the lock is at (built,
    /// broadcast, IS-locked, or chain-locked) and re-derives the
    /// credit-output derivation path; the signer-driven submission path
    /// then passes that path back to the same signer when constructing
    /// the IdentityCreate / IdentityTopUp transition.
    FromExistingAssetLock {
        /// The outpoint identifying the tracked asset lock (txid + output index).
        out_point: OutPoint,
    },
}
