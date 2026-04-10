//! Key storage types, identity status, and DPNS name metadata for managed identities.

use dpp::identity::Identity;
use dpp::identity::IdentityPublicKey;
use dpp::identity::KeyID;
use key_wallet::bip32::DerivationPath;
use std::collections::BTreeMap;
use zeroize::Zeroizing;

/// How a private key is stored/resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivateKeyData {
    /// Raw key bytes in memory (zeroized on drop).
    Clear(Zeroizing<[u8; 32]>),
    /// Derive on-demand from wallet seed at this path.
    AtWalletDerivationPath {
        wallet_seed_hash: [u8; 32],
        derivation_path: DerivationPath,
    },
}

/// Identity lifecycle status on Platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IdentityStatus {
    #[default]
    Unknown,
    PendingCreation,
    Active,
    FailedCreation,
    NotFound,
}

/// DPNS username associated with an identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DpnsNameInfo {
    pub label: String,
    pub acquired_at: Option<u64>,
}

/// Private key storage mapping KeyID to public key metadata + private key data.
pub type KeyStorage = BTreeMap<KeyID, (IdentityPublicKey, PrivateKeyData)>;

/// An identity we observe but don't own — read-only, no signing capability.
#[derive(Debug, Clone)]
pub struct WatchedIdentity {
    pub identity: Identity,
    pub dpns_names: Vec<DpnsNameInfo>,
    pub status: IdentityStatus,
}
