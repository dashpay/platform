//! Key storage types, identity status, and DPNS name metadata for managed identities.

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
    /// Derive on-demand from wallet seed at this path. Carries the
    /// DIP-9 `(identity_index, key_index)` pair alongside the fully
    /// materialized `derivation_path` so callers that need either
    /// form get it without reparsing.
    AtWalletDerivationPath {
        wallet_id: [u8; 32],
        derivation_path: DerivationPath,
        /// DIP-9 identity index.
        identity_index: u32,
        /// DIP-9 key index within the identity.
        key_index: u32,
    },
}

/// Identity lifecycle status on Platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DpnsNameInfo {
    pub label: String,
    pub acquired_at: Option<u64>,
}

/// Private key storage mapping KeyID to public key metadata + private key data.
///
/// Lives only in transient places — the `IdentityKeysChangeSet` apply
/// path constructs one per replay, the FFI key-preview path uses one
/// internally — but is no longer carried as a field on `ManagedIdentity`.
/// Private keys belong in the iOS Keychain on the client side; the Rust
/// side derives them on demand from the wallet seed via the DIP-9 path
/// recorded in `PrivateKeyData::AtWalletDerivationPath`.
pub type KeyStorage = BTreeMap<KeyID, (IdentityPublicKey, PrivateKeyData)>;
