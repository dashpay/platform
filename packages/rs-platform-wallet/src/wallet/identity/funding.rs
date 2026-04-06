//! Funding method enums for identity registration and top-up.
//!
//! These enums describe *how* an identity operation is funded, decoupling the
//! funding source from the identity lifecycle logic.
//!
//! ## Type overview
//!
//! * [`IdentityFunding`] — unified funding enum used by the new
//!   `create_funded_asset_lock_proof` flow. Covers wallet-balance,
//!   pre-existing asset locks, and specific-UTXO funding.
//! * [`IdentityFundingMethod`] / [`TopUpFundingMethod`] — original per-operation
//!   enums consumed by `register_identity_with_funding` and
//!   `top_up_identity_with_funding`. Retained for backwards compatibility.

use dashcore::{Address, OutPoint, PrivateKey, Transaction, TxOut};
use dpp::prelude::AssetLockProof;

// ─── Unified funding enum ────────────────────────────────────────────────────

/// How to fund an identity operation (registration, top-up, etc.).
///
/// This is the *unified* enum consumed by
/// [`CoreWallet::create_funded_asset_lock_proof`](crate::wallet::core::CoreWallet::create_funded_asset_lock_proof).
/// It replaces the earlier pattern of having separate funding enums per
/// operation type.
pub enum IdentityFunding {
    /// Build an asset lock from wallet UTXOs for the given amount.
    FromWalletBalance {
        /// Amount to lock (in duffs).
        amount_duffs: u64,
    },
    /// Use an existing, already-proved asset lock.
    FromExistingAssetLock {
        /// The full asset lock transaction.
        transaction: Transaction,
        /// The finality proof (IS or CL).
        proof: AssetLockProof,
        /// The one-time private key from the asset lock payload.
        private_key: PrivateKey,
    },
    /// Build an asset lock from a specific UTXO (e.g. QR-funded flow).
    FromUtxo {
        /// The outpoint identifying the UTXO to spend.
        outpoint: OutPoint,
        /// The transaction output being spent.
        tx_out: TxOut,
        /// The address that owns the UTXO.
        address: Address,
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
    /// Build an asset lock from a specific UTXO.
    FundWithUtxo {
        /// The outpoint identifying the UTXO to spend.
        outpoint: OutPoint,
        /// The transaction output being spent.
        txout: TxOut,
        /// The address that owns the UTXO.
        address: Address,
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
    /// Build an asset lock from a specific UTXO.
    FundWithUtxo {
        /// The outpoint identifying the UTXO to spend.
        outpoint: OutPoint,
        /// The transaction output being spent.
        txout: TxOut,
        /// The address that owns the UTXO.
        address: Address,
    },
}
