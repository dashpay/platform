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
use key_wallet::bip32::DerivationPath;

/// How to fund an identity operation (registration, top-up).
///
/// Resolved by the high-level `register_identity_with_funding` /
/// `top_up_identity_with_funding` helpers into an
/// `(AssetLockProof, DerivationPath, OutPoint)` triple that the
/// `_with_signer` SDK methods can consume. The `OutPoint` is retained
/// for cleanup (so the tracked-asset-lock row can be removed on
/// success) and for IS→CL fallback (so the consumed lock can be
/// looked up by outpoint when the IS proof times out or is rejected).
#[derive(Debug, Clone)]
pub enum IdentityFunding {
    /// Build an asset lock from wallet UTXOs for the given amount.
    ///
    /// The helper picks the appropriate funding account
    /// (`identity_registration` for register, `identity_topup` for top
    /// up), builds the asset-lock tx, broadcasts it, waits for an
    /// IS-lock proof, and falls back to ChainLock if the IS-lock times
    /// out (300s) or is rejected at Platform.
    FromWalletBalance {
        /// Amount to lock (in duffs).
        amount_duffs: u64,
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

    /// Use a pre-supplied asset lock proof + derivation path directly.
    ///
    /// The caller has already obtained the proof through some external
    /// flow (e.g. an SDK-side broadcast that ran outside this wallet's
    /// `AssetLockManager`) and just needs the registration / top-up
    /// flow to submit it. No tracking, no fallback, no cleanup — the
    /// caller owns the lifecycle.
    ///
    /// In practice this variant is used by callers that manage asset
    /// locks via a sibling component (evo-tool's tasks, integration
    /// tests, etc.). The Swift app's normal flow goes through
    /// `FromWalletBalance` or `FromExistingAssetLock`.
    UseAssetLock {
        /// The asset lock proof (IS or CL).
        proof: dpp::prelude::AssetLockProof,
        /// Derivation path the credit-output P2PKH was built from. The
        /// signer-driven submission path passes this to the asset-lock
        /// signer when constructing the IdentityCreate / IdentityTopUp
        /// transition.
        derivation_path: DerivationPath,
    },
}
