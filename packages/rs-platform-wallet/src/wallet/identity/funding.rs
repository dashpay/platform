//! Funding method enums for identity registration and top-up.
//!
//! These enums describe *how* an identity operation is funded, decoupling the
//! funding source from the identity lifecycle logic.

use dashcore::{Address, OutPoint, PrivateKey, TxOut};
use dpp::prelude::AssetLockProof;

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
