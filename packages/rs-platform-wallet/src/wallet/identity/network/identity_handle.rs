//! Identity wallet for managing Platform identities + DashPay state.
//!
//! Provides methods for the full identity lifecycle: registration, discovery
//! (gap-limit scan), top-up, withdrawal, credit transfer — **plus** the
//! DashPay-contract operations that live on the same identity (contact
//! requests, contacts, payments, profile, account labels).
//!
//! Historically DashPay lived on a separate `DashPayWallet<B>` facade, but
//! both views operated on the same underlying state (a single
//! [`ManagedIdentity`](crate::wallet::identity::ManagedIdentity) carries
//! both identity fields and DashPay fields). The two facades were merged to
//! cut handle-juggling at the FFI boundary and to give the DashPay
//! operations access to the same asset-lock / signer plumbing the identity
//! lifecycle already uses.
//!
//! The `B` generic parameter picks the transaction broadcaster used by
//! DashPay payment operations (`send_payment`). It defaults to
//! [`SpvBroadcaster`] — the only production broadcaster — so the majority
//! of call sites don't need to name it. Asset-lock funding still pins to
//! `SpvBroadcaster` because the [`AssetLockManager`] itself is pinned; that
//! invariant lives in `PlatformWallet::new`.

use std::sync::Arc;

use dashcore::secp256k1::PublicKey;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, KeyType};
use dpp::prelude::AssetLockProof;
use key_wallet::bip32::{ChildNumber, DerivationPath, ExtendedPrivKey, KeyDerivationType};
use key_wallet::dip9::{
    IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
};
use key_wallet::wallet::Wallet;
use key_wallet::Network;
use key_wallet_manager::WalletManager;
use zeroize::Zeroizing;

use crate::broadcaster::{SpvBroadcaster, TransactionBroadcaster};
use crate::diagnostics::{InstrumentedRwLock, ReadGuard, WriteGuard};
use crate::error::PlatformWalletError;
use crate::wallet::asset_lock::manager::AssetLockManager;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};

/// Default gap limit for identity discovery scanning.
///
/// Promoted to `pub` so external consumers (in particular the FFI
/// crate's identity-registration-key preview path) can align their
/// preview slot count with the scan window `discover()` walks. Keep
/// this and [`MASTER_KEY_INDEX`] as the single source of truth for
/// the discovery-scan policy.
pub const IDENTITY_GAP_LIMIT: u32 = 5;

/// Identity-authentication key index that the discovery scan probes
/// and that every identity this crate registers uses as its MASTER
/// auth key. Public so preview helpers can align on the same slot.
pub const MASTER_KEY_INDEX: u32 = 0;

/// Build the DIP-9 identity-authentication derivation path for the
/// given `(identity_index, key_index)` on `network`.
///
/// Path format: `m/9'/COIN_TYPE'/5'/0'/ECDSA'/identity_index'/key_index'`
/// where `COIN_TYPE'` picks mainnet vs testnet. ECDSA is hardcoded —
/// this is the only derivation type the discovery scan probes today.
pub(crate) fn identity_auth_derivation_path(
    network: key_wallet::Network,
    identity_index: u32,
    key_index: u32,
) -> Result<DerivationPath, PlatformWalletError> {
    identity_auth_derivation_path_for_type(
        network,
        KeyDerivationType::ECDSA,
        identity_index,
        key_index,
    )
}

/// Build the DIP-9 identity-authentication derivation path for the
/// given `(key_derivation_type, identity_index, key_index)` on
/// `network`. Generalizes the ECDSA-hardcoded
/// [`identity_auth_derivation_path`] so callers building keys for
/// different key types (BLS, EdDSA) can reach the right slot.
///
/// Path format:
/// `m/9'/COIN_TYPE'/5'/0'/key_derivation_type'/identity_index'/key_index'`
///
/// Promoted to `pub` so the FFI crate's mnemonic-driven derivation
/// path can call the library version instead of duplicating the
/// path-building logic.
pub fn identity_auth_derivation_path_for_type(
    network: key_wallet::Network,
    key_derivation_type: KeyDerivationType,
    identity_index: u32,
    key_index: u32,
) -> Result<DerivationPath, PlatformWalletError> {
    let base_path: DerivationPath = match network {
        key_wallet::Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
        _ => IDENTITY_AUTHENTICATION_PATH_TESTNET,
    }
    .into();

    let key_type_index: u32 = key_derivation_type.into();

    Ok(base_path.extend([
        ChildNumber::from_hardened_idx(key_type_index).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("Invalid key type index: {}", e))
        })?,
        ChildNumber::from_hardened_idx(identity_index).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("Invalid identity index: {}", e))
        })?,
        ChildNumber::from_hardened_idx(key_index).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("Invalid key index: {}", e))
        })?,
    ]))
}

/// One ECDSA identity-authentication keypair derived from a master
/// xpriv at a specific `(identity_index, key_index)` slot. Wraps
/// the secret scalar in [`Zeroizing`] so it is wiped on drop —
/// callers that ship the bytes elsewhere (FFI, Keychain) should
/// copy what they need and let this struct fall out of scope to
/// reclaim the secret memory.
pub struct DerivedIdentityAuthKey {
    /// Full DIP-9 path used (so callers can persist the breadcrumb).
    pub derivation_path: DerivationPath,
    /// 32-byte secp256k1 secret scalar.
    pub private_key: Zeroizing<[u8; 32]>,
    /// 33-byte compressed secp256k1 public key.
    pub public_key: [u8; 33],
}

/// Derive a single ECDSA identity-authentication keypair from a
/// master xpriv at `(identity_index, key_index)`. Pure function —
/// no `Wallet` required, so this works for watch-only wallets where
/// the seed is held outside the in-memory wallet manager.
///
/// Used by the FFI's mnemonic-driven derivation paths
/// (`platform_wallet_derive_identity_key_at_slot`,
/// `dash_sdk_derive_identity_keys_from_mnemonic`) so the path
/// builder + secp256k1 derive pass aren't duplicated in the FFI
/// crate. The mnemonic-to-master step still lives in the FFI
/// because mnemonic parsing pulls `key_wallet::mnemonic`, which
/// the library doesn't otherwise need.
pub fn derive_ecdsa_identity_auth_keypair_from_master(
    master: &ExtendedPrivKey,
    network: key_wallet::Network,
    identity_index: u32,
    key_index: u32,
) -> Result<DerivedIdentityAuthKey, PlatformWalletError> {
    use dashcore::secp256k1::Secp256k1;
    use key_wallet::bip32::ExtendedPubKey;

    let path = identity_auth_derivation_path_for_type(
        network,
        KeyDerivationType::ECDSA,
        identity_index,
        key_index,
    )?;
    let secp = Secp256k1::new();
    // `ExtendedPrivKey` doesn't implement `Zeroize`, so we can't
    // wrap it in `Zeroizing` directly — but its inner
    // `secp256k1::SecretKey` does implement `Drop` with a memzero,
    // so the secret scalar is scrubbed when `derived` falls out of
    // scope. The surrounding `chain_code` / `depth` /
    // `parent_fingerprint` / `child_number` are non-secret BIP-32
    // metadata; leaking them on the stack is a non-event. The
    // returned `private_key` is wrapped in `Zeroizing` below so
    // the 32-byte scalar copy crossing the function boundary is
    // also scrubbed on the caller's drop.
    let derived = master.derive_priv(&secp, &path).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!(
            "Failed to derive private key at (identity={identity_index}, key={key_index}): {e}"
        ))
    })?;
    let extended_pub = ExtendedPubKey::from_priv(&secp, &derived);

    Ok(DerivedIdentityAuthKey {
        derivation_path: path,
        private_key: Zeroizing::new(derived.private_key.secret_bytes()),
        public_key: extended_pub.public_key.serialize(),
    })
}

/// Derive the DIP-9 identity-authentication keypair at
/// `(identity_index, key_index)` on `network`.
///
/// Returns the full derivation path, the extended private key (the
/// caller can wrap it in [`Zeroizing`] if the raw scalar will escape
/// the crate), and the compressed `secp256k1::PublicKey` matching
/// that private key. Kept in sync with the discovery scan and the
/// FFI-exposed preview so both code paths probe / surface identical
/// key material.
pub fn derive_identity_auth_keypair(
    wallet: &Wallet,
    network: key_wallet::Network,
    identity_index: u32,
    key_index: u32,
) -> Result<(DerivationPath, ExtendedPrivKey, PublicKey), PlatformWalletError> {
    use dashcore::secp256k1::Secp256k1;
    use key_wallet::bip32::ExtendedPubKey;

    let full_path = identity_auth_derivation_path(network, identity_index, key_index)?;
    let auth_key = wallet
        .derive_extended_private_key(&full_path)
        .map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to derive authentication key: {}",
                e
            ))
        })?;

    let secp = Secp256k1::new();
    let extended_pub = ExtendedPubKey::from_priv(&secp, &auth_key);
    Ok((full_path, auth_key, extended_pub.public_key))
}

/// Derive the 20-byte RIPEMD160(SHA256) hash of the public key at the given
/// identity authentication path.
///
/// Path format: `base_path / key_type' / identity_index' / key_index'`
/// where `base_path` is `m/9'/COIN_TYPE'/5'/0'` (mainnet or testnet).
///
/// Thin wrapper over [`derive_identity_auth_keypair`] — shares the
/// path-building + derivation so the FFI-side preview and the live
/// scan can never drift from one another.
pub(crate) fn derive_identity_auth_key_hash(
    wallet: &Wallet,
    network: key_wallet::Network,
    identity_index: u32,
    key_index: u32,
) -> Result<[u8; 20], PlatformWalletError> {
    use dpp::util::hash::ripemd160_sha256;

    let (_path, _priv, public_key) =
        derive_identity_auth_keypair(wallet, network, identity_index, key_index)?;
    let public_key_bytes = public_key.serialize();
    let key_hash = ripemd160_sha256(&public_key_bytes);

    let mut key_hash_array = [0u8; 20];
    key_hash_array.copy_from_slice(&key_hash);

    Ok(key_hash_array)
}

/// Identity + DashPay wallet facade.
///
/// A view onto the shared `PlatformWalletInfo` state inside the wallet
/// manager. Covers both the identity lifecycle (registration, discovery,
/// top-up, transfer, withdrawal) and the DashPay-contract operations that
/// live on the same identity (contact requests, contacts, payments,
/// profile, account labels).
///
/// Generic parameter `B` is the [`TransactionBroadcaster`] used by
/// payment paths (DashPay `send_payment`). Defaults to
/// [`SpvBroadcaster`] — the only production broadcaster. Asset-lock
/// funding uses its own pinned-to-`SpvBroadcaster` manager; the `B`
/// parameter is only for the DashPay payment broadcast surface.
pub struct IdentityWallet<B: TransactionBroadcaster + ?Sized = SpvBroadcaster> {
    pub(crate) sdk: Arc<dash_sdk::Sdk>,
    /// Shared wallet manager holding key material and wallet info.
    pub(crate) wallet_manager: Arc<InstrumentedRwLock<WalletManager<PlatformWalletInfo>>>,
    /// Identifier for the wallet within the wallet manager.
    pub(crate) wallet_id: WalletId,
    /// Shared asset lock manager for building, broadcasting, and tracking
    /// asset lock transactions. Used by funding methods that build asset
    /// locks from wallet UTXOs.
    // Pinned to `SpvBroadcaster` to match the parent `PlatformWallet`.
    // If/when the outer broadcaster choice changes, flip this line
    // and the corresponding field in `PlatformWallet`.
    pub(crate) asset_locks: Arc<AssetLockManager<SpvBroadcaster>>,
    /// Per-wallet persistence handle for queuing changesets.
    pub(crate) persister: crate::wallet::persister::WalletPersister,
    /// Broadcaster for DashPay payment transactions. Distinct from the
    /// asset-lock broadcaster — the asset-lock manager is always
    /// `SpvBroadcaster`-pinned, while this one picks the broadcaster
    /// used by `send_payment` (static dispatch per call).
    pub(crate) broadcaster: Arc<B>,
}

// Manual `Debug`: the derive would require `B: Debug`, which is not part
// of the `TransactionBroadcaster` bound.
impl<B: TransactionBroadcaster + ?Sized> std::fmt::Debug for IdentityWallet<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentityWallet").finish()
    }
}

// Manual `Clone`: the derive would add `where B: Clone`, but `Arc<B>`
// clones without cloning `B` itself, so the bound would be spurious.
// `B: ?Sized` is enough.
impl<B: TransactionBroadcaster + ?Sized> Clone for IdentityWallet<B> {
    fn clone(&self) -> Self {
        Self {
            sdk: Arc::clone(&self.sdk),
            wallet_manager: Arc::clone(&self.wallet_manager),
            wallet_id: self.wallet_id,
            asset_locks: Arc::clone(&self.asset_locks),
            persister: self.persister.clone(),
            broadcaster: Arc::clone(&self.broadcaster),
        }
    }
}

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Build the DIP-9 identity authentication derivation path.
    ///
    /// Path format: `m/9'/coin_type'/5'/0'/key_type'/identity_index'/key_id'`
    pub fn identity_auth_derivation_path(
        network: Network,
        key_derivation_type: KeyDerivationType,
        identity_index: u32,
        key_id: u32,
    ) -> Result<DerivationPath, PlatformWalletError> {
        let base_path: DerivationPath = match network {
            Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
            _ => IDENTITY_AUTHENTICATION_PATH_TESTNET,
        }
        .into();

        let key_type_index: u32 = key_derivation_type.into();

        Ok(base_path.extend([
            ChildNumber::from_hardened_idx(key_type_index).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("Invalid key type index: {}", e))
            })?,
            ChildNumber::from_hardened_idx(identity_index).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("Invalid identity index: {}", e))
            })?,
            ChildNumber::from_hardened_idx(key_id).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("Invalid key ID: {}", e))
            })?,
        ]))
    }

    /// Derive the raw private key bytes for an identity authentication key.
    ///
    /// Determines the correct [`KeyDerivationType`] from the public key's
    /// [`KeyType`], builds the DIP-9 derivation path, and derives the
    /// private key from the wallet.
    ///
    /// Returns the bytes wrapped in [`Zeroizing`] so they are automatically
    /// wiped from memory when the value is dropped.
    pub fn derive_identity_key_bytes(
        wallet: &Wallet,
        network: Network,
        identity_index: u32,
        identity_public_key: &IdentityPublicKey,
    ) -> Result<Zeroizing<[u8; 32]>, PlatformWalletError> {
        let key_id = identity_public_key.id();
        let key_derivation_type = match identity_public_key.key_type() {
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => KeyDerivationType::ECDSA,
            KeyType::BLS12_381 => KeyDerivationType::BLS,
            // EdDSA uses the ECDSA derivation path; the raw bytes are
            // reinterpreted as an Ed25519 seed.
            KeyType::EDDSA_25519_HASH160 => KeyDerivationType::ECDSA,
            KeyType::BIP13_SCRIPT_HASH => {
                return Err(PlatformWalletError::InvalidIdentityData(
                    "BIP13_SCRIPT_HASH keys are not supported for signing".to_string(),
                ));
            }
        };

        let path = Self::identity_auth_derivation_path(
            network,
            key_derivation_type,
            identity_index,
            key_id,
        )?;

        let secret_key = wallet.derive_private_key(&path).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to derive private key for identity key {}: {}",
                key_id, e
            ))
        })?;

        Ok(Zeroizing::new(secret_key.secret_bytes()))
    }

    /// Get a read-lock handle to the shared [`WalletManager`].
    ///
    /// Access wallet info via `wm.get_wallet_info(&wallet_id)` and key material
    /// via `wm.get_wallet(&wallet_id)` on the returned guard. The identity
    /// manager is on the wallet info: `info.identity_manager`.
    pub async fn wallet_manager_read(&self) -> ReadGuard<'_, WalletManager<PlatformWalletInfo>> {
        self.wallet_manager.read().await
    }

    /// Get a write-lock handle to the shared [`WalletManager`].
    ///
    /// Access wallet info via `wm.get_wallet_info_mut(&wallet_id)` on the
    /// returned guard. This allows callers to mutate managed identities (e.g.
    /// adding or updating identities from an external persistence layer).
    pub async fn wallet_manager_write(&self) -> WriteGuard<'_, WalletManager<PlatformWalletInfo>> {
        self.wallet_manager.write().await
    }

    /// Try to acquire a write-lock on the shared [`WalletManager`] without blocking.
    ///
    /// Returns `None` if the lock is currently held by another task.
    /// Useful for synchronous callers that cannot await.
    pub fn try_wallet_manager_write(
        &self,
    ) -> Option<WriteGuard<'_, WalletManager<PlatformWalletInfo>>> {
        self.wallet_manager.try_write().ok()
    }

    /// The wallet ID for this identity wallet's underlying key material.
    pub fn wallet_id(&self) -> &WalletId {
        &self.wallet_id
    }

    /// Derive the ECDH private key for the given identity's encryption
    /// key (DashPay ECDH).
    ///
    /// Uses the DIP-9 identity-authentication derivation path and
    /// returns the raw `secp256k1::SecretKey` needed for ECDH with a
    /// contact.
    ///
    /// The encryption key must be `ECDSA_SECP256K1` or `ECDSA_HASH160`;
    /// other key types are not supported for ECDH derivation.
    pub(super) fn derive_encryption_private_key(
        wallet: &Wallet,
        network: key_wallet::Network,
        identity_index: u32,
        encryption_key: &IdentityPublicKey,
    ) -> Result<dashcore::secp256k1::SecretKey, PlatformWalletError> {
        // Validate that the encryption key type is compatible with ECDH
        // derivation.
        match encryption_key.key_type() {
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => {}
            other => {
                return Err(PlatformWalletError::InvalidIdentityData(format!(
                    "Unsupported key type {:?} for ECDH derivation; \
                     expected ECDSA_SECP256K1 or ECDSA_HASH160",
                    other
                )));
            }
        }

        let path = Self::identity_auth_derivation_path(
            network,
            KeyDerivationType::ECDSA,
            identity_index,
            encryption_key.id(),
        )?;

        let ext_priv = wallet.derive_extended_private_key(&path).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to derive encryption private key: {}",
                e
            ))
        })?;

        // Wrap intermediate private key bytes in `Zeroizing` so they
        // are wiped on drop.
        let secret_bytes = Zeroizing::new(ext_priv.private_key.secret_bytes());

        dashcore::secp256k1::SecretKey::from_slice(&*secret_bytes).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Invalid derived encryption private key: {}",
                e
            ))
        })
    }

    /// Extract the outpoint from an asset lock proof.
    ///
    /// For instant proofs, this is the txid of the embedded transaction
    /// combined with the output index from the proof.
    /// For chain proofs, this is the out_point directly.
    pub(super) fn out_point_from_proof(proof: &AssetLockProof) -> Option<dashcore::OutPoint> {
        match proof {
            AssetLockProof::Instant(instant) => Some(dashcore::OutPoint::new(
                instant.transaction().txid(),
                instant.output_index(),
            )),
            AssetLockProof::Chain(chain) => Some(chain.out_point),
        }
    }
}
