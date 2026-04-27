//! Key storage types, identity status, and DPNS name metadata for managed identities.

use dpp::identity::IdentityPublicKey;
use dpp::identity::KeyID;
use key_wallet::bip32::DerivationPath;
use std::collections::BTreeMap;
use zeroize::Zeroizing;

/// How a private key is stored/resolved.
///
/// # Security
///
/// `Clear` material is zeroized on drop via [`Zeroizing`]. Prefer
/// [`AtWalletDerivationPath`](Self::AtWalletDerivationPath) wherever
/// possible — that variant carries no key bytes at all and lets the
/// signer re-derive on demand from the encrypted wallet seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivateKeyData {
    /// Raw key bytes in memory (zeroized on drop).
    Clear(Zeroizing<[u8; 32]>),
    /// Derive on-demand from wallet seed at this path. Carries the
    /// DIP-9 `(identity_index, key_index)` pair alongside the fully
    /// materialized `derivation_path` so callers that need either
    /// form get it without reparsing.
    AtWalletDerivationPath {
        /// Wallet that owns the seed used for derivation.
        wallet_id: [u8; 32],
        /// Fully materialized BIP-32 derivation path.
        derivation_path: DerivationPath,
        /// DIP-9 identity index.
        identity_index: u32,
        /// DIP-9 key index within the identity.
        key_index: u32,
    },
}

/// Identity lifecycle status on Platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IdentityStatus {
    /// Status has not been determined yet (e.g. fresh import, no sync
    /// has confirmed presence on Platform).
    #[default]
    Unknown,
    /// Registration state transition has been broadcast; the identity
    /// is awaiting confirmation.
    PendingCreation,
    /// Identity is registered and confirmed on Platform.
    Active,
    /// Registration was attempted and failed terminally (e.g. asset
    /// lock proof rejected). The identity will not appear on chain.
    FailedCreation,
    /// Platform confirmed the identity does not exist (lookup miss
    /// after registration window closed).
    NotFound,
}

/// DPNS username associated with an identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DpnsNameInfo {
    /// The DPNS label registered for the identity (e.g. `"alice"` for
    /// `alice.dash`).
    pub label: String,
    /// Unix-second timestamp the name was acquired, when known.
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
