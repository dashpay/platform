//! Identity wallet for managing Platform identities.
//!
//! Provides methods for the full identity lifecycle: registration, discovery
//! (gap-limit scan), top-up, withdrawal, and credit transfer.

use std::sync::Arc;

use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, KeyType};
use dpp::prelude::{AssetLockProof, Identifier};
use key_wallet::bip32::{ChildNumber, DerivationPath, KeyDerivationType};
use key_wallet::dip9::{
    IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
};
use key_wallet::wallet::Wallet;
use key_wallet::Network;
use key_wallet_manager::WalletManager;
use tokio::sync::RwLock;
use zeroize::Zeroizing;

use crate::error::PlatformWalletError;
use crate::wallet::asset_lock::manager::AssetLockManager;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::wallet::signer::{IdentitySigner, ManagedIdentitySigner};

/// Default gap limit for identity discovery scanning.
pub(super) const IDENTITY_GAP_LIMIT: u32 = 5;

/// Derive the 20-byte RIPEMD160(SHA256) hash of the public key at the given
/// identity authentication path.
///
/// Path format: `base_path / key_type' / identity_index' / key_index'`
/// where `base_path` is `m/9'/COIN_TYPE'/5'/0'` (mainnet or testnet).
pub(super) fn derive_identity_auth_key_hash(
    wallet: &Wallet,
    network: key_wallet::Network,
    identity_index: u32,
    key_index: u32,
) -> Result<[u8; 20], PlatformWalletError> {
    use dashcore::secp256k1::Secp256k1;
    use dpp::util::hash::ripemd160_sha256;
    use key_wallet::bip32::{ChildNumber, DerivationPath, ExtendedPubKey, KeyDerivationType};
    use key_wallet::dip9::{
        IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
    };

    let base_path = match network {
        key_wallet::Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
        _ => IDENTITY_AUTHENTICATION_PATH_TESTNET,
    };

    let key_type_index: u32 = KeyDerivationType::ECDSA.into();

    let mut full_path = DerivationPath::from(base_path);
    full_path = full_path.extend([
        ChildNumber::from_hardened_idx(key_type_index).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("Invalid key type index: {}", e))
        })?,
        ChildNumber::from_hardened_idx(identity_index).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("Invalid identity index: {}", e))
        })?,
        ChildNumber::from_hardened_idx(key_index).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("Invalid key index: {}", e))
        })?,
    ]);

    let auth_key = wallet
        .derive_extended_private_key(&full_path)
        .map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to derive authentication key: {}",
                e
            ))
        })?;

    let secp = Secp256k1::new();
    let public_key = ExtendedPubKey::from_priv(&secp, &auth_key);
    let public_key_bytes = public_key.public_key.serialize();
    let key_hash = ripemd160_sha256(&public_key_bytes);

    let mut key_hash_array = [0u8; 20];
    key_hash_array.copy_from_slice(&key_hash);

    Ok(key_hash_array)
}

/// Identity wallet providing identity management functionality.
#[derive(Clone)]
pub struct IdentityWallet {
    pub(crate) sdk: Arc<dash_sdk::Sdk>,
    /// Shared wallet manager holding key material and wallet info.
    pub(crate) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Identifier for the wallet within the wallet manager.
    pub(crate) wallet_id: WalletId,
    /// Shared asset lock manager for building, broadcasting, and tracking
    /// asset lock transactions. Used by funding methods that build asset
    /// locks from wallet UTXOs.
    pub(crate) asset_locks: Arc<AssetLockManager>,
    /// Per-wallet persistence handle for queuing changesets.
    pub(crate) persister: crate::wallet::persister::WalletPersister,
}

impl IdentityWallet {
    /// Create an [`IdentitySigner`] for the given identity index.
    ///
    /// The returned signer implements `Signer<IdentityPublicKey>` and derives
    /// private keys on-the-fly from the wallet using the DIP-9 identity
    /// authentication path.
    pub fn signer_for_identity(&self, identity_index: u32) -> IdentitySigner {
        IdentitySigner::new(
            self.wallet_manager.clone(),
            self.wallet_id,
            self.sdk.network,
            identity_index,
        )
    }

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

    /// Create a [`ManagedIdentitySigner`] for a managed identity by its ID.
    ///
    /// The signer resolves keys from the identity's `key_storage`, falling
    /// back to the standard DIP-9 derivation when a key is not in storage.
    pub async fn signer_for(
        &self,
        identity_id: &Identifier,
    ) -> Result<ManagedIdentitySigner, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
            crate::error::PlatformWalletError::WalletNotFound(
                "Wallet info not found in wallet manager".to_string(),
            )
        })?;
        let managed = info
            .identity_manager
            .managed_identity(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
        Ok(managed.signer(
            self.wallet_manager.clone(),
            self.wallet_id,
            self.sdk.network,
        ))
    }

    /// Get a read-lock handle to the shared [`WalletManager`].
    ///
    /// Access wallet info via `wm.get_wallet_info(&wallet_id)` and key material
    /// via `wm.get_wallet(&wallet_id)` on the returned guard. The identity
    /// manager is on the wallet info: `info.identity_manager`.
    pub async fn wallet_manager_read(
        &self,
    ) -> tokio::sync::RwLockReadGuard<'_, WalletManager<PlatformWalletInfo>> {
        self.wallet_manager.read().await
    }

    /// Get a write-lock handle to the shared [`WalletManager`].
    ///
    /// Access wallet info via `wm.get_wallet_info_mut(&wallet_id)` on the
    /// returned guard. This allows callers to mutate managed identities (e.g.
    /// adding or updating identities from an external persistence layer).
    pub async fn wallet_manager_write(
        &self,
    ) -> tokio::sync::RwLockWriteGuard<'_, WalletManager<PlatformWalletInfo>> {
        self.wallet_manager.write().await
    }

    /// Try to acquire a write-lock on the shared [`WalletManager`] without blocking.
    ///
    /// Returns `None` if the lock is currently held by another task.
    /// Useful for synchronous callers that cannot await.
    pub fn try_wallet_manager_write(
        &self,
    ) -> Option<tokio::sync::RwLockWriteGuard<'_, WalletManager<PlatformWalletInfo>>> {
        self.wallet_manager.try_write().ok()
    }

    /// The wallet ID for this identity wallet's underlying key material.
    pub fn wallet_id(&self) -> &WalletId {
        &self.wallet_id
    }

    /// Extract the outpoint from an asset lock proof.
    ///
    /// For instant proofs, this is the txid of the embedded transaction
    /// combined with the output index from the proof.
    /// For chain proofs, this is the out_point directly.
    fn out_point_from_proof(proof: &AssetLockProof) -> Option<dashcore::OutPoint> {
        match proof {
            AssetLockProof::Instant(instant) => Some(dashcore::OutPoint::new(
                instant.transaction().txid(),
                instant.output_index(),
            )),
            AssetLockProof::Chain(chain) => Some(chain.out_point),
        }
    }
}

impl std::fmt::Debug for IdentityWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentityWallet").finish()
    }
}

// ---------------------------------------------------------------------------
// Sub-modules — each groups a coherent set of identity operations. See the
// per-file doc comment for what lives where.
// ---------------------------------------------------------------------------

mod discovery;
mod dpns;
mod loading;
mod register_from_addresses;
mod registration;
mod top_up;
mod top_up_from_addresses;
mod transfer;
mod transfer_to_addresses;
mod update;
mod withdrawal;
