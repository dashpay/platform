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
///
/// Intended transitions: `Unknown` -> `PendingCreation` -> `Active`,
/// `PendingCreation` -> `FailedCreation` -> `Active` (after a retry), and
/// `Active` -> `NotFound` -> `Active`. Today the library itself sets only
/// `Unknown` (the default) and `Active` (after loading or discovering the
/// identity on Platform); the other variants exist for hosts and are
/// persisted like the rest. The FFI stores each variant as a byte in
/// declaration order (0 to 4), so never reorder them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IdentityStatus {
    /// Not checked against Platform yet.
    #[default]
    Unknown,
    /// Registration submitted, not yet confirmed.
    PendingCreation,
    /// Confirmed on Platform.
    Active,
    /// Registration failed; it can be retried.
    FailedCreation,
    /// Was active, but Platform no longer returns it.
    NotFound,
}

/// DPNS username associated with an identity.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DpnsNameInfo {
    pub label: String,
    pub acquired_at: Option<u64>,
}

/// How much of an identity's owned DPNS names one username fetch returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DpnsFetch {
    /// The identity's whole owned set: paging reached a short page, so a
    /// label missing from the fetch has left the identity.
    Complete,
    /// A lower bound: the page bound was reached, or the platform version
    /// cannot continue a page. A label missing from the fetch proves nothing.
    Partial,
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
