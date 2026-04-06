//! Identity wallet for managing Platform identities.
//!
//! Provides methods for the full identity lifecycle: registration, discovery
//! (gap-limit scan), top-up, withdrawal, and credit transfer.

use std::collections::BTreeMap;
use std::sync::Arc;

use dashcore::Address as DashAddress;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::v0::IdentityV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::{AssetLockProof, Identifier};
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use tokio::sync::RwLock;

use dpp::identity::signer::Signer;

use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::transition::top_up_identity::TopUpIdentity;
use dash_sdk::platform::transition::top_up_identity_from_addresses::TopUpIdentityFromAddresses;
use dash_sdk::platform::transition::transfer::TransferToIdentity;
use dash_sdk::platform::transition::transfer_to_addresses::TransferToAddresses;
use dash_sdk::platform::transition::withdraw_from_identity::WithdrawFromIdentity;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;

use crate::error::PlatformWalletError;
use crate::wallet::core::asset_lock_manager::AssetLockManager;
use crate::wallet::platform_addresses::PlatformAddressWallet;
use crate::wallet::signer::{IdentitySigner, ManagedIdentitySigner};

use super::funding::{IdentityFunding, IdentityFundingMethod, TopUpFundingMethod};
use super::manager::IdentityManager;

/// Default gap limit for identity discovery scanning.
const IDENTITY_GAP_LIMIT: u32 = 5;

/// Derive the 20-byte RIPEMD160(SHA256) hash of the public key at the given
/// identity authentication path.
///
/// Path format: `base_path / key_type' / identity_index' / key_index'`
/// where `base_path` is `m/9'/COIN_TYPE'/5'/0'` (mainnet or testnet).
fn derive_identity_auth_key_hash(
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
    pub(crate) wallet: Arc<RwLock<Wallet>>,
    pub(crate) wallet_info: Arc<RwLock<ManagedWalletInfo>>,
    pub(crate) identity_manager: Arc<RwLock<IdentityManager>>,
    /// Shared asset lock manager for building, broadcasting, and tracking
    /// asset lock transactions. Used by funding methods that build asset
    /// locks from wallet UTXOs.
    pub(crate) asset_locks: Arc<AssetLockManager>,
}

impl IdentityWallet {
    /// Create an [`IdentitySigner`] for the given identity index.
    ///
    /// The returned signer implements `Signer<IdentityPublicKey>` and derives
    /// private keys on-the-fly from the wallet using the DIP-9 identity
    /// authentication path.
    pub fn signer_for_identity(&self, identity_index: u32) -> IdentitySigner {
        IdentitySigner::new(self.wallet.clone(), self.sdk.network, identity_index)
    }

    /// Create a [`ManagedIdentitySigner`] for a managed identity by its ID.
    ///
    /// The signer resolves keys from the identity's `key_storage`, falling
    /// back to the standard DIP-9 derivation when a key is not in storage.
    pub async fn signer_for(
        &self,
        identity_id: &Identifier,
    ) -> Result<ManagedIdentitySigner, PlatformWalletError> {
        let manager = self.identity_manager.read().await;
        let managed = manager
            .managed_identity(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
        Ok(managed.signer(self.wallet.clone(), self.sdk.network))
    }

    /// Get a read-lock handle to the [`IdentityManager`].
    ///
    /// This allows callers to inspect managed identities (e.g. after a
    /// [`sync()`](Self::sync) call) without exposing the internal `RwLock`
    /// directly.
    pub async fn identity_manager(&self) -> tokio::sync::RwLockReadGuard<'_, IdentityManager> {
        self.identity_manager.read().await
    }

    /// Get a write-lock handle to the [`IdentityManager`].
    ///
    /// This allows callers to mutate managed identities (e.g. adding or
    /// updating identities from an external persistence layer).
    pub async fn identity_manager_mut(&self) -> tokio::sync::RwLockWriteGuard<'_, IdentityManager> {
        self.identity_manager.write().await
    }

    /// Try to acquire a write-lock on the [`IdentityManager`] without blocking.
    ///
    /// Returns `None` if the lock is currently held by another task.
    /// Useful for synchronous callers that cannot await.
    pub fn try_identity_manager_mut(
        &self,
    ) -> Option<tokio::sync::RwLockWriteGuard<'_, IdentityManager>> {
        self.identity_manager.try_write().ok()
    }

    /// Extract the transaction ID from an asset lock proof.
    ///
    /// For instant proofs, this is the txid of the embedded transaction.
    /// For chain proofs, this is the txid from the out_point.
    /// Returns `None` only if the instant proof has no valid out_point.
    fn txid_from_proof(proof: &AssetLockProof) -> Option<dashcore::Txid> {
        match proof {
            AssetLockProof::Instant(instant) => Some(instant.transaction().txid()),
            AssetLockProof::Chain(chain) => Some(chain.out_point.txid),
        }
    }
}

impl std::fmt::Debug for IdentityWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentityWallet").finish()
    }
}

// ---------------------------------------------------------------------------
// Identity registration
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Register a new identity on Platform.
    ///
    /// Convenience wrapper that uses `FundWithWallet` funding. For other
    /// funding methods, use [`register_identity_with_funding`](Self::register_identity_with_funding).
    ///
    /// # Arguments
    ///
    /// * `amount_duffs` - Amount of Dash (in duffs) to lock for the identity's
    ///   initial credit balance.
    /// * `identity_index` - BIP-9 identity index (hardened) in the key tree.
    /// * `key_count` - Number of authentication keys to register with the
    ///   identity (must be >= 1).
    pub async fn register_identity(
        &self,
        amount_duffs: u64,
        identity_index: u32,
        key_count: u32,
    ) -> Result<Identity, PlatformWalletError> {
        self.register_identity_with_funding(
            IdentityFundingMethod::FundWithWallet { amount_duffs },
            identity_index,
            key_count,
        )
        .await
    }

    /// Register a new identity on Platform with a specified funding method.
    ///
    /// High-level flow:
    /// 1. Obtain an asset lock proof according to the chosen `funding` method.
    /// 2. Generate `key_count` identity authentication keys at DIP-9 paths
    ///    for the given `identity_index`.
    /// 3. Call the SDK's `Identity::put_to_platform_and_wait_for_response()`
    ///    to broadcast the identity-create state transition.
    /// 4. Add the new identity to the local `identity_manager`.
    ///
    /// # Funding methods
    ///
    /// * `UseAssetLock` - Use a pre-existing proof and private key directly.
    /// * `FundWithWallet` - Build an asset lock from wallet UTXOs (default).
    /// * `FundWithUtxo` - Build an asset lock from a specific UTXO (TODO:
    ///   requires a dedicated CoreWallet method; currently falls back to
    ///   `FundWithWallet` using the UTXO's value).
    ///
    /// # IS -> CL fallback
    ///
    /// When the Platform submission fails because an InstantSend proof has
    /// expired, callers should retry with a ChainLock proof. The fallback
    /// logic lives in the error-handling layer above this method (e.g. in the
    /// `PlatformWalletManager`) because it requires waiting for chain-lock
    /// confirmation via DAPI queries that are not available at this level.
    /// The [`PlatformWalletError::AssetLockExpired`] and
    /// [`PlatformWalletError::AssetLockNotChainLocked`] error variants are
    /// provided for this purpose.
    pub async fn register_identity_with_funding(
        &self,
        funding: IdentityFundingMethod,
        identity_index: u32,
        key_count: u32,
    ) -> Result<Identity, PlatformWalletError> {
        if key_count == 0 {
            return Err(PlatformWalletError::InvalidIdentityData(
                "key_count must be at least 1".to_string(),
            ));
        }

        // Step 1: Obtain the asset lock proof and private key.
        let (asset_lock_proof, asset_lock_private_key) = match funding {
            IdentityFundingMethod::UseAssetLock { proof, private_key } => (proof, private_key),
            IdentityFundingMethod::FundWithWallet { amount_duffs } => {
                use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
                let (proof, key, _txid) = self
                    .asset_locks
                    .create_funded_asset_lock_proof(
                        amount_duffs,
                        0,
                        AssetLockFundingType::IdentityRegistration,
                        identity_index,
                    )
                    .await?;
                (proof, key)
            }
            IdentityFundingMethod::FundWithUtxo {
                outpoint: _,
                txout,
                address: _,
            } => {
                // TODO: Add an AssetLockManager method that builds an asset lock from
                // a specific UTXO instead of selecting from the full UTXO set.
                // For now, fall back to FundWithWallet using the UTXO's value.
                use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
                let amount_duffs = txout.value;
                let (proof, key, _txid) = self
                    .asset_locks
                    .create_funded_asset_lock_proof(
                        amount_duffs,
                        0,
                        AssetLockFundingType::IdentityRegistration,
                        identity_index,
                    )
                    .await?;
                (proof, key)
            }
        };

        // Step 2: Derive identity authentication keys at DIP-9 paths.
        let mut keys_map: BTreeMap<u32, IdentityPublicKey> = BTreeMap::new();
        {
            use dashcore::secp256k1::Secp256k1;
            use key_wallet::bip32::{
                ChildNumber, DerivationPath, ExtendedPubKey, KeyDerivationType,
            };
            use key_wallet::dip9::{
                IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
            };

            let wallet = self.wallet.read().await;
            let base_path: DerivationPath = match self.sdk.network {
                key_wallet::Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
                _ => IDENTITY_AUTHENTICATION_PATH_TESTNET,
            }
            .into();

            let key_type_index: u32 = KeyDerivationType::ECDSA.into();

            let secp = Secp256k1::new();

            for key_index in 0..key_count {
                let full_path = base_path.extend([
                    ChildNumber::from_hardened_idx(key_type_index).map_err(|e| {
                        PlatformWalletError::InvalidIdentityData(format!(
                            "Invalid key type index: {}",
                            e
                        ))
                    })?,
                    ChildNumber::from_hardened_idx(identity_index).map_err(|e| {
                        PlatformWalletError::InvalidIdentityData(format!(
                            "Invalid identity index: {}",
                            e
                        ))
                    })?,
                    ChildNumber::from_hardened_idx(key_index).map_err(|e| {
                        PlatformWalletError::InvalidIdentityData(format!(
                            "Invalid key index: {}",
                            e
                        ))
                    })?,
                ]);

                let ext_priv = wallet
                    .derive_extended_private_key(&full_path)
                    .map_err(|e| {
                        PlatformWalletError::InvalidIdentityData(format!(
                            "Failed to derive authentication key: {}",
                            e
                        ))
                    })?;

                let ext_pub = ExtendedPubKey::from_priv(&secp, &ext_priv);
                let compressed_pubkey = ext_pub.public_key.serialize();

                // First key is MASTER, remaining keys are HIGH.
                let security_level = if key_index == 0 {
                    SecurityLevel::MASTER
                } else {
                    SecurityLevel::HIGH
                };

                let identity_public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
                    id: key_index,
                    purpose: Purpose::AUTHENTICATION,
                    security_level,
                    contract_bounds: None,
                    key_type: KeyType::ECDSA_SECP256K1,
                    read_only: false,
                    data: BinaryData::new(compressed_pubkey.to_vec()),
                    disabled_at: None,
                });

                keys_map.insert(key_index, identity_public_key);
            }
        }

        // Step 3: Build the Identity object and submit it to Platform.
        let identity = Identity::V0(IdentityV0 {
            id: Identifier::default(), // SDK fills this from the asset lock
            public_keys: keys_map,
            balance: 0,
            revision: 0,
        });

        let signer = self.signer_for_identity(identity_index);

        // Extract the txid before consuming the proof, in case we need to
        // build a ChainLock proof for recovery.
        let txid = Self::txid_from_proof(&asset_lock_proof);

        let identity = match identity
            .put_to_platform_and_wait_for_response(
                &self.sdk,
                asset_lock_proof,
                &asset_lock_private_key,
                &signer,
                None,
            )
            .await
        {
            Ok(identity) => identity,
            Err(e) if crate::error::is_instant_lock_proof_invalid(&e) => {
                // IS-lock proof was rejected — try to upgrade to ChainLock.
                if let Some(txid) = txid {
                    tracing::warn!(
                        "IS-lock proof rejected for identity registration (tx {}), \
                         retrying with ChainLock proof",
                        txid
                    );
                    let chain_proof = self.asset_locks.upgrade_to_chain_lock_proof(&txid).await?;
                    identity
                        .put_to_platform_and_wait_for_response(
                            &self.sdk,
                            chain_proof,
                            &asset_lock_private_key,
                            &signer,
                            None,
                        )
                        .await
                        .map_err(|e| {
                            PlatformWalletError::InvalidIdentityData(format!(
                                "Failed to register identity on Platform (ChainLock retry): {}",
                                e
                            ))
                        })?
                } else {
                    return Err(PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to register identity on Platform: {}",
                        e
                    )));
                }
            }
            Err(e) => {
                return Err(PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to register identity on Platform: {}",
                    e
                )));
            }
        };

        // Step 4: Add the identity to the local manager (with its HD index).
        let mut manager = self.identity_manager.write().await;
        manager.add_identity(identity.clone(), identity_index)?;

        Ok(identity)
    }

    /// Register a new identity using an externally-provided identity, asset
    /// lock proof, and signer.
    ///
    /// Unlike [`register_identity_with_funding`](Self::register_identity_with_funding),
    /// this method does **not** derive keys or manage the internal
    /// `IdentityManager`. The caller supplies a fully-constructed `Identity`
    /// object, the asset lock proof + private key, and a `Signer`
    /// implementation directly.
    ///
    /// This is useful when the caller manages identities outside of the
    /// platform-wallet `IdentityManager` (e.g. evo-tool's
    /// `QualifiedIdentity`).
    ///
    /// Returns the confirmed `Identity` from Platform.
    pub async fn register_identity_with_signer<S: Signer<IdentityPublicKey>>(
        &self,
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: &dashcore::PrivateKey,
        signer: &S,
    ) -> Result<Identity, dash_sdk::Error> {
        identity
            .put_to_platform_and_wait_for_response(
                &self.sdk,
                asset_lock_proof,
                asset_lock_private_key,
                signer,
                None,
            )
            .await
    }

    /// Top up an identity's credit balance using an externally-provided
    /// identity and asset lock proof.
    ///
    /// Unlike [`top_up_identity_with_funding`](Self::top_up_identity_with_funding),
    /// this method does **not** look up the identity in the internal
    /// `IdentityManager`. The caller supplies the `Identity` object and the
    /// asset lock proof + private key directly.
    ///
    /// This is useful when the caller manages identities outside of the
    /// platform-wallet `IdentityManager` (e.g. evo-tool's
    /// `QualifiedIdentity`).
    ///
    /// Returns the new credit balance.
    pub async fn top_up_identity_with_signer(
        &self,
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: &dashcore::PrivateKey,
    ) -> Result<u64, dash_sdk::Error> {
        identity
            .top_up_identity(
                &self.sdk,
                asset_lock_proof,
                asset_lock_private_key,
                None, // user_fee_increase
                None, // settings
            )
            .await
    }

    /// Register a new identity using an [`IdentityFunding`] variant and an
    /// externally-provided identity + signer.
    ///
    /// This method unifies funding resolution and Platform submission in a
    /// single call:
    ///
    /// * **`FromWalletBalance`** — builds an asset lock from wallet UTXOs via
    ///   [`AssetLockManager::create_funded_asset_lock_proof`], then submits the
    ///   identity registration to Platform.
    /// * **`FromExistingAssetLock`** — uses the supplied proof and private key
    ///   directly.
    /// * **`FromUtxo`** — not yet implemented; returns an error.
    ///
    /// Unlike [`register_identity_with_funding`](Self::register_identity_with_funding),
    /// this method does **not** derive keys or manage the internal
    /// `IdentityManager`. The caller supplies a fully-constructed `Identity`
    /// and a `Signer` implementation, making it suitable for callers that
    /// manage identities externally (e.g. evo-tool's `QualifiedIdentity`).
    ///
    /// Returns the confirmed `Identity` from Platform.
    pub async fn funded_register_identity<S: Signer<IdentityPublicKey>>(
        &self,
        identity: &Identity,
        funding: IdentityFunding,
        identity_index: u32,
        signer: &S,
    ) -> Result<Identity, PlatformWalletError> {
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

        let (asset_lock_proof, asset_lock_private_key, tracked_txid) = match funding {
            IdentityFunding::FromWalletBalance { amount_duffs } => {
                let (proof, key, txid) = self
                    .asset_locks
                    .create_funded_asset_lock_proof(
                        amount_duffs,
                        0,
                        AssetLockFundingType::IdentityRegistration,
                        identity_index,
                    )
                    .await?;
                (proof, key, Some(txid))
            }
            IdentityFunding::FromExistingAssetLock {
                transaction: _,
                proof,
                private_key,
            } => (proof, private_key, None),
            IdentityFunding::FromUtxo { .. } => {
                return Err(PlatformWalletError::InvalidIdentityData(
                    "FromUtxo funding is not yet implemented for funded_register_identity"
                        .to_string(),
                ));
            }
        };

        // Extract the txid before consuming the proof, in case we need to
        // build a ChainLock proof for recovery.
        let txid = Self::txid_from_proof(&asset_lock_proof);

        let result = match self
            .register_identity_with_signer(
                identity,
                asset_lock_proof,
                &asset_lock_private_key,
                signer,
            )
            .await
        {
            Ok(identity) => identity,
            Err(e) if crate::error::is_instant_lock_proof_invalid(&e) => {
                if let Some(txid) = txid {
                    tracing::warn!(
                        "IS-lock proof rejected for funded identity registration (tx {}), \
                         retrying with ChainLock proof",
                        txid
                    );
                    let chain_proof = self.asset_locks.upgrade_to_chain_lock_proof(&txid).await?;
                    self.register_identity_with_signer(
                        identity,
                        chain_proof,
                        &asset_lock_private_key,
                        signer,
                    )
                    .await
                    .map_err(PlatformWalletError::Sdk)?
                } else {
                    return Err(PlatformWalletError::Sdk(e));
                }
            }
            Err(e) => return Err(PlatformWalletError::Sdk(e)),
        };

        // Clean up the tracked asset lock after successful consumption.
        if let Some(txid) = tracked_txid {
            self.asset_locks.remove_asset_lock(&txid).await;
        }

        Ok(result)
    }

    /// Top up an identity using an [`IdentityFunding`] variant and an
    /// externally-provided identity.
    ///
    /// This method unifies funding resolution and Platform submission in a
    /// single call:
    ///
    /// * **`FromWalletBalance`** — builds an asset lock from wallet UTXOs via
    ///   [`AssetLockManager::create_funded_asset_lock_proof`], then submits the
    ///   top-up to Platform.
    /// * **`FromExistingAssetLock`** — uses the supplied proof and private key
    ///   directly.
    /// * **`FromUtxo`** — not yet implemented; returns an error.
    ///
    /// Unlike [`top_up_identity_with_funding`](Self::top_up_identity_with_funding),
    /// this method does **not** look up the identity in the internal
    /// `IdentityManager`. The caller supplies the `Identity` object directly,
    /// making it suitable for callers that manage identities externally
    /// (e.g. evo-tool's `QualifiedIdentity`).
    ///
    /// Returns the new credit balance.
    pub async fn funded_top_up_identity(
        &self,
        identity: &Identity,
        funding: IdentityFunding,
        identity_index: u32,
    ) -> Result<u64, PlatformWalletError> {
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

        let (asset_lock_proof, asset_lock_private_key, tracked_txid) = match funding {
            IdentityFunding::FromWalletBalance { amount_duffs } => {
                let (proof, key, txid) = self
                    .asset_locks
                    .create_funded_asset_lock_proof(
                        amount_duffs,
                        0,
                        AssetLockFundingType::IdentityTopUp,
                        identity_index,
                    )
                    .await?;
                (proof, key, Some(txid))
            }
            IdentityFunding::FromExistingAssetLock {
                transaction: _,
                proof,
                private_key,
            } => (proof, private_key, None),
            IdentityFunding::FromUtxo { .. } => {
                return Err(PlatformWalletError::InvalidIdentityData(
                    "FromUtxo funding is not yet implemented for funded_top_up_identity"
                        .to_string(),
                ));
            }
        };

        // Extract the txid before consuming the proof, in case we need to
        // build a ChainLock proof for recovery.
        let txid = Self::txid_from_proof(&asset_lock_proof);

        let new_balance = match self
            .top_up_identity_with_signer(identity, asset_lock_proof, &asset_lock_private_key)
            .await
        {
            Ok(balance) => balance,
            Err(e) if crate::error::is_instant_lock_proof_invalid(&e) => {
                if let Some(txid) = txid {
                    tracing::warn!(
                        "IS-lock proof rejected for funded identity top-up (tx {}), \
                         retrying with ChainLock proof",
                        txid
                    );
                    let chain_proof = self.asset_locks.upgrade_to_chain_lock_proof(&txid).await?;
                    self.top_up_identity_with_signer(identity, chain_proof, &asset_lock_private_key)
                        .await
                        .map_err(PlatformWalletError::Sdk)?
                } else {
                    return Err(PlatformWalletError::Sdk(e));
                }
            }
            Err(e) => return Err(PlatformWalletError::Sdk(e)),
        };

        // Clean up the tracked asset lock after successful consumption.
        if let Some(txid) = tracked_txid {
            self.asset_locks.remove_asset_lock(&txid).await;
        }

        Ok(new_balance)
    }
}

// ---------------------------------------------------------------------------
// Identity discovery (gap-limit scan)
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Discover identities owned by this wallet via gap-limit scanning.
    ///
    /// Starting from the last scanned index stored in the identity manager,
    /// derives consecutive ECDSA authentication keys from the wallet's BIP-32
    /// tree and queries Platform for registered identities. For each identity
    /// index, key indices 0 through 11 are scanned (covering the typical range
    /// of authentication keys an identity may have been registered with).
    /// Scanning stops after `IDENTITY_GAP_LIMIT` (5) consecutive identity-index
    /// misses (i.e. none of the 12 key indices matched).
    ///
    /// For every discovered identity this method also:
    /// - queries DPNS for associated usernames,
    /// - stores the matched derivation path in the identity's key storage,
    /// - records the wallet seed hash, and
    /// - sets the identity status to `Active`.
    ///
    /// Any discovered identities are added to the local identity manager and
    /// returned. The `last_scanned_index` is updated so subsequent calls
    /// resume where this one left off.
    pub async fn sync(&self) -> Result<Vec<Identity>, PlatformWalletError> {
        use super::managed_identity::key_storage::{DpnsNameInfo, IdentityStatus, PrivateKeyData};
        use dash_sdk::platform::types::identity::PublicKeyHash;
        use dash_sdk::platform::Fetch;
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        use dpp::util::hash::ripemd160_sha256;
        use key_wallet::bip32::{ChildNumber, DerivationPath, KeyDerivationType};
        use key_wallet::dip9::{
            IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
        };

        /// Number of key indices to scan per identity index.
        const KEY_INDEX_SCAN_LIMIT: u32 = 12;

        let network = {
            let wallet = self.wallet.read().await;
            wallet.network
        };

        let start_index = {
            let manager = self.identity_manager.read().await;
            manager.last_scanned_index()
        };

        // Use the wallet ID as the seed hash — it is a 32-byte identifier
        // derived from the wallet seed during wallet creation.
        let wallet_seed_hash: [u8; 32] = {
            let info = self.wallet_info.read().await;
            info.wallet_id
        };

        let mut consecutive_misses = 0u32;
        let mut identity_index = start_index;
        let mut discovered: Vec<Identity> = Vec::new();

        while consecutive_misses < IDENTITY_GAP_LIMIT {
            let mut found_at_this_index = false;

            // Scan key indices 0..KEY_INDEX_SCAN_LIMIT for this identity index.
            for key_index in 0..KEY_INDEX_SCAN_LIMIT {
                let key_hash_array = {
                    let wallet = self.wallet.read().await;
                    derive_identity_auth_key_hash(&wallet, network, identity_index, key_index)?
                };

                // Query Platform for an identity registered with this key hash.
                // No locks are held during this network call.
                match Identity::fetch(&self.sdk, PublicKeyHash(key_hash_array)).await {
                    Ok(Some(identity)) => {
                        let identity_id = identity.id();

                        // Build the full derivation path for the matched key.
                        let base_path: DerivationPath = match network {
                            key_wallet::Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
                            _ => IDENTITY_AUTHENTICATION_PATH_TESTNET,
                        }
                        .into();
                        let key_type_index: u32 = KeyDerivationType::ECDSA.into();
                        let full_path = base_path.extend([
                            ChildNumber::from_hardened_idx(key_type_index).map_err(|e| {
                                PlatformWalletError::InvalidIdentityData(format!(
                                    "Invalid key type index: {}",
                                    e
                                ))
                            })?,
                            ChildNumber::from_hardened_idx(identity_index).map_err(|e| {
                                PlatformWalletError::InvalidIdentityData(format!(
                                    "Invalid identity index: {}",
                                    e
                                ))
                            })?,
                            ChildNumber::from_hardened_idx(key_index).map_err(|e| {
                                PlatformWalletError::InvalidIdentityData(format!(
                                    "Invalid key index: {}",
                                    e
                                ))
                            })?,
                        ]);

                        // Find which KeyID in the on-chain identity matches this
                        // key hash so we can store the derivation path against it.
                        let matched_key_id_and_pub = identity
                            .public_keys()
                            .iter()
                            .find(|(_, pk)| {
                                let pk_hash = ripemd160_sha256(pk.data().as_slice());
                                pk_hash.as_slice() == key_hash_array
                            })
                            .map(|(kid, pk)| (*kid, pk.clone()));

                        // Acquire write lock to add/enrich the identity.
                        let mut manager = self.identity_manager.write().await;
                        let is_new = manager.identity(&identity_id).is_none();
                        if is_new {
                            manager.add_identity(identity.clone(), identity_index)?;
                        }

                        if let Some(managed) = manager.managed_identity_mut(&identity_id) {
                            managed.set_status(IdentityStatus::Active);
                            managed.wallet_seed_hash = Some(wallet_seed_hash);

                            if let Some((kid, pub_key)) = matched_key_id_and_pub {
                                managed.add_key(
                                    kid,
                                    pub_key,
                                    PrivateKeyData::AtWalletDerivationPath {
                                        wallet_seed_hash,
                                        derivation_path: full_path,
                                    },
                                );
                            }
                        }
                        drop(manager);

                        if is_new {
                            discovered.push(identity.clone());
                        }
                        found_at_this_index = true;

                        // An identity was found at this key_index; no need to
                        // continue scanning further key indices for this
                        // identity_index.
                        break;
                    }
                    Ok(None) => {
                        // This key_index did not match; try the next one.
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to query identity at index {} key {}: {}",
                            identity_index,
                            key_index,
                            e
                        );
                        // Treat individual key-index errors as a miss and
                        // continue scanning the remaining key indices.
                    }
                }
            }

            if found_at_this_index {
                consecutive_misses = 0;
            } else {
                consecutive_misses += 1;
            }

            identity_index += 1;
        }

        // --- DPNS lookup for all discovered identities ---
        for identity in &discovered {
            let identity_id = identity.id();
            match self
                .sdk
                .get_dpns_usernames_by_identity(identity_id, None)
                .await
            {
                Ok(usernames) => {
                    let mut manager = self.identity_manager.write().await;
                    if let Some(managed) = manager.managed_identity_mut(&identity_id) {
                        for username in usernames {
                            managed.add_dpns_name(DpnsNameInfo {
                                label: username.label,
                                acquired_at: None,
                            });
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to fetch DPNS names for identity {}: {}",
                        identity_id,
                        e
                    );
                }
            }
        }

        // Update the last scanned index so the next sync resumes here.
        let mut manager = self.identity_manager.write().await;
        manager.set_last_scanned_index(identity_index);

        Ok(discovered)
    }
}

// ---------------------------------------------------------------------------
// Top-up
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Top up an existing identity's credit balance.
    ///
    /// Convenience wrapper that uses `FundWithWallet` funding. For other
    /// funding methods, use [`top_up_identity_with_funding`](Self::top_up_identity_with_funding).
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identifier of the identity to top up.
    /// * `topup_index` - An incrementing index distinguishing successive
    ///   top-ups for the same identity.
    /// * `amount_duffs` - Amount of Dash (in duffs) to add.
    pub async fn top_up_identity(
        &self,
        identity_id: &Identifier,
        topup_index: u32,
        amount_duffs: u64,
    ) -> Result<(), PlatformWalletError> {
        self.top_up_identity_with_funding(
            identity_id,
            TopUpFundingMethod::FundWithWallet { amount_duffs },
            topup_index,
        )
        .await
    }

    /// Top up an existing identity's credit balance with a specified funding method.
    ///
    /// # Funding methods
    ///
    /// * `UseAssetLock` - Use a pre-existing proof and private key directly.
    /// * `FundWithWallet` - Build an asset lock from wallet UTXOs (default).
    /// * `FundWithUtxo` - Build an asset lock from a specific UTXO (TODO:
    ///   requires a dedicated CoreWallet method; currently falls back to
    ///   `FundWithWallet` using the UTXO's value).
    ///
    /// # IS -> CL fallback
    ///
    /// See [`register_identity_with_funding`](Self::register_identity_with_funding)
    /// for details on the IS -> CL fallback strategy.
    pub async fn top_up_identity_with_funding(
        &self,
        identity_id: &Identifier,
        funding: TopUpFundingMethod,
        topup_index: u32,
    ) -> Result<(), PlatformWalletError> {
        // Retrieve the identity and its HD index from the manager.
        let (identity, identity_index) = {
            let manager = self.identity_manager.read().await;
            let identity = manager
                .identity(identity_id)
                .cloned()
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            let index = manager
                .identity_index(identity_id)
                .ok_or(PlatformWalletError::IdentityIndexNotSet(*identity_id))?;
            (identity, index)
        };

        // Step 1: Obtain the asset lock proof and private key.
        let (asset_lock_proof, asset_lock_private_key) = match funding {
            TopUpFundingMethod::UseAssetLock { proof, private_key } => (proof, private_key),
            TopUpFundingMethod::FundWithWallet { amount_duffs } => {
                use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
                let (proof, key, _txid) = self
                    .asset_locks
                    .create_funded_asset_lock_proof(
                        amount_duffs,
                        0,
                        AssetLockFundingType::IdentityTopUp,
                        identity_index,
                    )
                    .await?;
                (proof, key)
            }
            TopUpFundingMethod::FundWithUtxo {
                outpoint: _,
                txout,
                address: _,
            } => {
                // TODO: Add an AssetLockManager method that builds an asset lock from
                // a specific UTXO instead of selecting from the full UTXO set.
                // For now, fall back to FundWithWallet using the UTXO's value.
                use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
                let amount_duffs = txout.value;
                let (proof, key, _txid) = self
                    .asset_locks
                    .create_funded_asset_lock_proof(
                        amount_duffs,
                        0,
                        AssetLockFundingType::IdentityTopUp,
                        identity_index,
                    )
                    .await?;
                (proof, key)
            }
        };

        // Extract the txid before consuming the proof, in case we need to
        // build a ChainLock proof for recovery.
        let txid = Self::txid_from_proof(&asset_lock_proof);

        // Step 2: Submit the top-up state transition.
        let new_balance = match identity
            .top_up_identity(
                &self.sdk,
                asset_lock_proof,
                &asset_lock_private_key,
                None, // user_fee_increase
                None, // settings
            )
            .await
        {
            Ok(balance) => balance,
            Err(e) if crate::error::is_instant_lock_proof_invalid(&e) => {
                // IS-lock proof was rejected — try to upgrade to ChainLock.
                if let Some(txid) = txid {
                    tracing::warn!(
                        "IS-lock proof rejected for identity top-up (tx {}), \
                         retrying with ChainLock proof",
                        txid
                    );
                    let chain_proof = self.asset_locks.upgrade_to_chain_lock_proof(&txid).await?;
                    identity
                        .top_up_identity(
                            &self.sdk,
                            chain_proof,
                            &asset_lock_private_key,
                            None,
                            None,
                        )
                        .await
                        .map_err(|e| {
                            PlatformWalletError::InvalidIdentityData(format!(
                                "Failed to top up identity (ChainLock retry): {}",
                                e
                            ))
                        })?
                } else {
                    return Err(PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to top up identity: {}",
                        e
                    )));
                }
            }
            Err(e) => {
                return Err(PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to top up identity: {}",
                    e
                )));
            }
        };

        // Update the identity's balance in the local manager.
        {
            let mut manager = self.identity_manager.write().await;
            if let Some(identity) = manager.identity_mut(identity_id) {
                identity.set_balance(new_balance);
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Withdrawal
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Withdraw credits from an identity to a Dash address.
    ///
    /// Submits an `IdentityCreditWithdrawalTransition` to Platform that moves
    /// the specified amount (in platform credits) from the identity back to
    /// a Core chain address.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identifier of the identity to withdraw from.
    /// * `amount` - Amount of credits to withdraw.
    /// * `to_address` - The Dash P2PKH address to receive the withdrawal.
    pub async fn withdraw_credits(
        &self,
        identity_id: &Identifier,
        amount: u64,
        to_address: &DashAddress,
    ) -> Result<(), PlatformWalletError> {
        // Retrieve the identity and its HD index from the manager.
        let (identity, identity_index) = {
            let manager = self.identity_manager.read().await;
            let identity = manager
                .identity(identity_id)
                .cloned()
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            let index = manager
                .identity_index(identity_id)
                .ok_or(PlatformWalletError::IdentityIndexNotSet(*identity_id))?;
            (identity, index)
        };

        let signer = self.signer_for_identity(identity_index);

        let new_balance = identity
            .withdraw(
                &self.sdk,
                Some(to_address.clone()),
                amount,
                None, // core_fee_per_byte
                None, // signing_withdrawal_key_to_use
                signer,
                None, // settings
            )
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to withdraw credits: {}",
                    e
                ))
            })?;

        // Update the identity's balance in the local manager.
        {
            let mut manager = self.identity_manager.write().await;
            if let Some(identity) = manager.identity_mut(identity_id) {
                identity.set_balance(new_balance);
            }
        }

        Ok(())
    }

    /// Withdraw credits using an externally-provided identity and signer.
    ///
    /// Unlike [`withdraw_credits`](Self::withdraw_credits), this method does
    /// **not** look up the identity in the internal `IdentityManager`. Instead,
    /// the caller supplies the `Identity` object and a `Signer` implementation
    /// directly. This is useful when the caller manages identities outside of
    /// the platform-wallet `IdentityManager` (e.g. evo-tool's
    /// `QualifiedIdentity`).
    ///
    /// Returns the remaining credit balance after the withdrawal.
    #[allow(clippy::too_many_arguments)]
    pub async fn withdraw_credits_with_signer<S: Signer<IdentityPublicKey> + Send>(
        &self,
        identity: &Identity,
        to_address: Option<DashAddress>,
        amount: u64,
        signing_withdrawal_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
    ) -> Result<u64, dash_sdk::Error> {
        identity
            .withdraw(
                &self.sdk,
                to_address,
                amount,
                Some(1), // core_fee_per_byte
                signing_withdrawal_key_to_use,
                signer,
                None, // settings
            )
            .await
    }
}

// ---------------------------------------------------------------------------
// Credit transfer
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Transfer credits from one identity to another.
    ///
    /// Submits an `IdentityCreditTransferTransition` to Platform that moves
    /// `amount` credits from `from_id` to `to_id`.
    ///
    /// # Arguments
    ///
    /// * `from_id` - The identifier of the sending identity (must be owned
    ///   by this wallet).
    /// * `to_id` - The identifier of the receiving identity.
    /// * `amount` - Amount of credits to transfer.
    pub async fn transfer_credits(
        &self,
        from_id: &Identifier,
        to_id: &Identifier,
        amount: u64,
    ) -> Result<(), PlatformWalletError> {
        // Retrieve the sending identity and its HD index from the manager.
        let (identity, identity_index) = {
            let manager = self.identity_manager.read().await;
            let identity = manager
                .identity(from_id)
                .cloned()
                .ok_or(PlatformWalletError::IdentityNotFound(*from_id))?;
            let index = manager
                .identity_index(from_id)
                .ok_or(PlatformWalletError::IdentityIndexNotSet(*from_id))?;
            (identity, index)
        };

        let signer = self.signer_for_identity(identity_index);

        let (sender_balance, _receiver_balance) = identity
            .transfer_credits(
                &self.sdk, *to_id, amount, None, // signing_transfer_key_to_use
                signer, None, // settings
            )
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to transfer credits: {}",
                    e
                ))
            })?;

        // Update the sender's balance in the local manager.
        {
            let mut manager = self.identity_manager.write().await;
            if let Some(identity) = manager.identity_mut(from_id) {
                identity.set_balance(sender_balance);
            }
        }

        Ok(())
    }

    /// Transfer credits using an externally-provided identity and signer.
    ///
    /// Unlike [`transfer_credits`](Self::transfer_credits), this method does
    /// **not** look up the identity in the internal `IdentityManager`. The
    /// caller supplies the `Identity` and a `Signer` directly.
    ///
    /// Returns `(sender_balance, receiver_balance)` after the transfer.
    pub async fn transfer_credits_with_signer<S: Signer<IdentityPublicKey> + Send>(
        &self,
        identity: &Identity,
        to_id: Identifier,
        amount: u64,
        signing_transfer_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
    ) -> Result<(u64, u64), dash_sdk::Error> {
        identity
            .transfer_credits(
                &self.sdk,
                to_id,
                amount,
                signing_transfer_key_to_use,
                signer,
                None, // settings
            )
            .await
    }
}

// ---------------------------------------------------------------------------
// Identity update (add/disable keys)
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Update an identity by adding or disabling public keys.
    ///
    /// Builds an `IdentityUpdateTransition`, signs it with the identity's
    /// master key, and broadcasts it to Platform.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity to update.
    /// * `add_public_keys` - New keys to add (key IDs are auto-assigned).
    /// * `disable_public_keys` - Key IDs to disable.
    pub async fn update_identity(
        &self,
        identity_id: &Identifier,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<u32>,
    ) -> Result<(), PlatformWalletError> {
        use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
        use dpp::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
        use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
        use dpp::state_transition::proof_result::StateTransitionProofResult;

        let (mut identity, identity_index) = {
            let manager = self.identity_manager.read().await;
            let identity = manager
                .identity(identity_id)
                .cloned()
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            let index = manager
                .identity_index(identity_id)
                .ok_or(PlatformWalletError::IdentityIndexNotSet(*identity_id))?;
            (identity, index)
        };

        // Increment revision for the update transition.
        let original_revision = identity.revision();
        identity.set_revision(original_revision + 1);

        // Find a master key that the signer can use.
        let signer = self.signer_for_identity(identity_index);

        let master_key_id = identity
            .public_keys()
            .iter()
            .find(|(_, key)| {
                key.purpose() == Purpose::AUTHENTICATION
                    && key.security_level() == SecurityLevel::MASTER
                    && key.key_type() == KeyType::ECDSA_SECP256K1
            })
            .map(|(id, _)| *id)
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "No signable master key found on identity".to_string(),
                )
            })?;

        // Get identity nonce from Platform.
        let identity_nonce = self
            .sdk
            .get_identity_nonce(identity.id(), true, None)
            .await?;

        // Build the update transition.
        let state_transition = IdentityUpdateTransition::try_from_identity_with_signer(
            &identity,
            &master_key_id,
            add_public_keys,
            disable_public_keys,
            identity_nonce,
            0, // user_fee_increase
            &signer,
            self.sdk.version(),
            None,
        )
        .map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to create identity update transition: {}",
                e
            ))
        })?;

        // Broadcast and wait for confirmation.
        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&self.sdk, None)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to broadcast identity update: {}",
                    e
                ))
            })?;

        Ok(())
    }

    /// Update an identity using an externally-provided identity and signer.
    ///
    /// Unlike [`update_identity`](Self::update_identity), this method does
    /// **not** look up the identity in the internal `IdentityManager`. The
    /// caller supplies the `Identity`, master key ID, and a `Signer` directly.
    ///
    /// Returns the [`StateTransitionProofResult`] from the broadcast so callers
    /// can inspect proof-verified outcomes (e.g. updated keys, balance).
    pub async fn update_identity_with_signer<S: Signer<IdentityPublicKey>>(
        &self,
        identity: &Identity,
        master_key_id: &u32,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<u32>,
        signer: &S,
    ) -> Result<dpp::state_transition::proof_result::StateTransitionProofResult, dash_sdk::Error>
    {
        use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
        use dpp::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
        use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;

        // Get identity nonce from Platform.
        let identity_nonce = self
            .sdk
            .get_identity_nonce(identity.id(), true, None)
            .await?;

        // Build the update transition.
        let state_transition = IdentityUpdateTransition::try_from_identity_with_signer(
            identity,
            master_key_id,
            add_public_keys,
            disable_public_keys,
            identity_nonce,
            0, // user_fee_increase
            signer,
            self.sdk.version(),
            None,
        )
        .map_err(|e| dash_sdk::Error::Protocol(e))?;

        // Broadcast and wait for confirmation.
        let result = state_transition.broadcast_and_wait(&self.sdk, None).await?;

        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Top-up from platform addresses
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Top up an identity by spending platform address balances.
    ///
    /// Uses the `TopUpIdentityFromAddresses` SDK trait. Address nonces are
    /// looked up automatically.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity to top up.
    /// * `inputs` - Map of platform addresses to credit amounts to spend.
    /// * `platform_address_wallet` - The platform address wallet (provides signing).
    pub async fn top_up_from_addresses(
        &self,
        identity_id: &Identifier,
        inputs: BTreeMap<PlatformAddress, Credits>,
        platform_address_wallet: &PlatformAddressWallet,
    ) -> Result<Credits, PlatformWalletError> {
        let identity = {
            let manager = self.identity_manager.read().await;
            manager
                .identity(identity_id)
                .cloned()
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?
        };

        let (_address_infos, new_balance) = identity
            .top_up_from_addresses(
                &self.sdk,
                inputs,
                platform_address_wallet,
                None, // settings
            )
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to top up identity from addresses: {}",
                    e
                ))
            })?;

        // Update the identity's balance in the local manager.
        {
            let mut manager = self.identity_manager.write().await;
            if let Some(identity) = manager.identity_mut(identity_id) {
                identity.set_balance(new_balance);
            }
        }

        Ok(new_balance)
    }
}

// ---------------------------------------------------------------------------
// Transfer credits to platform addresses
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Transfer credits from an identity to multiple platform addresses.
    ///
    /// Uses the `TransferToAddresses` SDK trait.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The sending identity (must be owned by this wallet).
    /// * `recipient_addresses` - Map of platform addresses to credit amounts.
    pub async fn transfer_credits_to_addresses(
        &self,
        identity_id: &Identifier,
        recipient_addresses: BTreeMap<PlatformAddress, Credits>,
    ) -> Result<Credits, PlatformWalletError> {
        let (identity, identity_index) = {
            let manager = self.identity_manager.read().await;
            let identity = manager
                .identity(identity_id)
                .cloned()
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            let index = manager
                .identity_index(identity_id)
                .ok_or(PlatformWalletError::IdentityIndexNotSet(*identity_id))?;
            (identity, index)
        };

        let signer = self.signer_for_identity(identity_index);

        let (_address_infos, new_balance) = identity
            .transfer_credits_to_addresses(
                &self.sdk,
                recipient_addresses,
                None, // signing_transfer_key_to_use
                &signer,
                None, // settings
            )
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to transfer credits to addresses: {}",
                    e
                ))
            })?;

        // Update the sender's balance in the local manager.
        {
            let mut manager = self.identity_manager.write().await;
            if let Some(identity) = manager.identity_mut(identity_id) {
                identity.set_balance(new_balance);
            }
        }

        Ok(new_balance)
    }
}

// ---------------------------------------------------------------------------
// DPNS name operations
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Register a DPNS name for an identity.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity to register the name for.
    /// * `name` - The desired username label (e.g., "alice").
    pub async fn register_name(
        &self,
        identity_id: &Identifier,
        name: &str,
    ) -> Result<String, PlatformWalletError> {
        use dash_sdk::platform::dpns_usernames::RegisterDpnsNameInput;

        let (identity, identity_index, auth_key) = {
            let manager = self.identity_manager.read().await;
            let identity = manager
                .identity(identity_id)
                .cloned()
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            let index = manager
                .identity_index(identity_id)
                .ok_or(PlatformWalletError::IdentityIndexNotSet(*identity_id))?;
            // Use the first authentication key (key_id 0).
            let key = identity
                .get_first_public_key_matching(
                    Purpose::AUTHENTICATION,
                    [SecurityLevel::MASTER, SecurityLevel::HIGH].into(),
                    [KeyType::ECDSA_SECP256K1].into(),
                    false,
                )
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(
                        "No authentication key found on identity".to_string(),
                    )
                })?
                .clone();
            (identity, index, key)
        };

        let signer = self.signer_for_identity(identity_index);

        let input = RegisterDpnsNameInput {
            label: name.to_string(),
            identity,
            identity_public_key: auth_key,
            signer,
            preorder_callback: None,
        };

        let result = self.sdk.register_dpns_name(input).await.map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to register DPNS name '{}': {}",
                name, e
            ))
        })?;

        Ok(result.full_domain_name)
    }

    /// Register a DPNS name using an externally-provided identity and signer.
    ///
    /// Unlike [`register_name`](Self::register_name), this method does **not**
    /// look up the identity in the internal `IdentityManager`. The caller
    /// supplies the `Identity`, the signing key, and a `Signer` directly.
    ///
    /// Returns the full domain name (e.g. "alice.dash").
    pub async fn register_name_with_signer<S: Signer<IdentityPublicKey>>(
        &self,
        identity: Identity,
        name: &str,
        identity_public_key: IdentityPublicKey,
        signer: S,
    ) -> Result<String, dash_sdk::Error> {
        use dash_sdk::platform::dpns_usernames::RegisterDpnsNameInput;

        let input = RegisterDpnsNameInput {
            label: name.to_string(),
            identity,
            identity_public_key,
            signer,
            preorder_callback: None,
        };

        let result = self.sdk.register_dpns_name(input).await?;
        Ok(result.full_domain_name)
    }

    /// Resolve a DPNS name to an identity identifier.
    ///
    /// Accepts both "alice" and "alice.dash" formats.
    pub async fn resolve_name(
        &self,
        name: &str,
    ) -> Result<Option<Identifier>, PlatformWalletError> {
        self.sdk.resolve_dpns_name(name).await.map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to resolve DPNS name '{}': {}",
                name, e
            ))
        })
    }

    /// Search for DPNS names by prefix.
    pub async fn search_names(
        &self,
        prefix: &str,
        limit: Option<u32>,
    ) -> Result<Vec<dash_sdk::platform::dpns_usernames::DpnsUsername>, PlatformWalletError> {
        self.sdk
            .search_dpns_names(prefix, limit)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to search DPNS names with prefix '{}': {}",
                    prefix, e
                ))
            })
    }
}

// ---------------------------------------------------------------------------
// Identity loading & refresh
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Load a single identity by its BIP-9 HD identity index.
    ///
    /// Derives the authentication key hash at the given `identity_index`
    /// (key_index 0) and queries Platform for an identity registered with
    /// that key. If found the identity is added to the local
    /// [`IdentityManager`] with its derivation-path key storage, status set
    /// to `Active`, DPNS names queried, and wallet seed hash recorded.
    ///
    /// Returns the identity if one was found, or `None` if no identity is
    /// registered at that index.
    pub async fn load_identity_by_index(
        &self,
        identity_index: u32,
    ) -> Result<Option<Identity>, PlatformWalletError> {
        use super::managed_identity::key_storage::{DpnsNameInfo, IdentityStatus, PrivateKeyData};
        use dash_sdk::platform::types::identity::PublicKeyHash;
        use dash_sdk::platform::Fetch;
        use dpp::util::hash::ripemd160_sha256;
        use key_wallet::bip32::{ChildNumber, DerivationPath, KeyDerivationType};
        use key_wallet::dip9::{
            IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
        };

        let network = {
            let wallet = self.wallet.read().await;
            wallet.network
        };

        let wallet_seed_hash: [u8; 32] = {
            let info = self.wallet_info.read().await;
            info.wallet_id
        };

        // Derive auth key hash at key_index 0 for the given identity_index.
        let key_hash_array = {
            let wallet = self.wallet.read().await;
            derive_identity_auth_key_hash(&wallet, network, identity_index, 0)?
        };

        // Query Platform for an identity registered with this key hash.
        let identity = match Identity::fetch(&self.sdk, PublicKeyHash(key_hash_array)).await {
            Ok(Some(identity)) => identity,
            Ok(None) => return Ok(None),
            Err(e) => {
                return Err(PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch identity at index {}: {}",
                    identity_index, e
                )));
            }
        };

        let identity_id = identity.id();

        // Build the full derivation path for the matched key (key_index 0).
        let base_path: DerivationPath = match network {
            key_wallet::Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
            _ => IDENTITY_AUTHENTICATION_PATH_TESTNET,
        }
        .into();
        let key_type_index: u32 = KeyDerivationType::ECDSA.into();
        let full_path = base_path.extend([
            ChildNumber::from_hardened_idx(key_type_index).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("Invalid key type index: {}", e))
            })?,
            ChildNumber::from_hardened_idx(identity_index).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("Invalid identity index: {}", e))
            })?,
            ChildNumber::from_hardened_idx(0u32).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("Invalid key index: {}", e))
            })?,
        ]);

        // Find which KeyID in the on-chain identity matches this key hash.
        let matched_key_id_and_pub = identity
            .public_keys()
            .iter()
            .find(|(_, pk)| {
                let pk_hash = ripemd160_sha256(pk.data().as_slice());
                pk_hash.as_slice() == key_hash_array
            })
            .map(|(kid, pk)| (*kid, pk.clone()));

        // Add the identity to the manager and enrich it.
        {
            let mut manager = self.identity_manager.write().await;
            if manager.identity(&identity_id).is_none() {
                manager.add_identity(identity.clone(), identity_index)?;
            }

            if let Some(managed) = manager.managed_identity_mut(&identity_id) {
                managed.set_status(IdentityStatus::Active);
                managed.wallet_seed_hash = Some(wallet_seed_hash);

                if let Some((kid, pub_key)) = matched_key_id_and_pub {
                    managed.add_key(
                        kid,
                        pub_key,
                        PrivateKeyData::AtWalletDerivationPath {
                            wallet_seed_hash,
                            derivation_path: full_path,
                        },
                    );
                }
            }
        }

        // Query DPNS names for the discovered identity.
        match self
            .sdk
            .get_dpns_usernames_by_identity(identity_id, None)
            .await
        {
            Ok(usernames) => {
                let mut manager = self.identity_manager.write().await;
                if let Some(managed) = manager.managed_identity_mut(&identity_id) {
                    for username in usernames {
                        managed.add_dpns_name(DpnsNameInfo {
                            label: username.label,
                            acquired_at: None,
                        });
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to fetch DPNS names for identity {}: {}",
                    identity_id,
                    e
                );
            }
        }

        Ok(Some(identity))
    }

    /// Refresh an identity that is already in the local manager by
    /// re-fetching it from Platform.
    ///
    /// The identity must already exist in the [`IdentityManager`]. Its
    /// on-chain state (keys, balance, revision) is replaced with the latest
    /// version from Platform and the status is set to `Active`.
    ///
    /// Returns the refreshed identity.
    ///
    /// # Errors
    ///
    /// * [`PlatformWalletError::IdentityNotFound`] if the identity is not in
    ///   the manager.
    /// * An error if Platform does not return the identity (e.g. it was
    ///   deleted).
    pub async fn refresh_identity(
        &self,
        identity_id: &Identifier,
    ) -> Result<Identity, PlatformWalletError> {
        use super::managed_identity::key_storage::IdentityStatus;
        use dash_sdk::platform::Fetch;

        // Verify identity exists in the manager.
        {
            let manager = self.identity_manager.read().await;
            if manager.identity(identity_id).is_none() {
                return Err(PlatformWalletError::IdentityNotFound(*identity_id));
            }
        }

        // Fetch the latest state from Platform.
        let identity = Identity::fetch(&self.sdk, *identity_id)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch identity {} from Platform: {}",
                    identity_id, e
                ))
            })?
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Identity {} not found on Platform",
                    identity_id
                ))
            })?;

        // Update the managed identity.
        {
            let mut manager = self.identity_manager.write().await;
            if let Some(managed) = manager.managed_identity_mut(identity_id) {
                managed.identity = identity.clone();
                managed.set_status(IdentityStatus::Active);
            }
        }

        Ok(identity)
    }

    /// Refresh an identity using an externally-provided identity ID.
    ///
    /// Unlike [`refresh_identity`](Self::refresh_identity), this method does
    /// **not** look up or update the internal `IdentityManager`. It simply
    /// fetches the latest identity from Platform and returns it. This is
    /// useful when the caller manages identities outside of the
    /// platform-wallet `IdentityManager` (e.g. evo-tool's
    /// `QualifiedIdentity`).
    ///
    /// Returns the refreshed identity, or an error if not found on Platform.
    pub async fn refresh_identity_with_signer(
        &self,
        identity_id: &Identifier,
    ) -> Result<Identity, dash_sdk::Error> {
        use dash_sdk::platform::Fetch;

        Identity::fetch(&self.sdk, *identity_id)
            .await?
            .ok_or_else(|| {
                dash_sdk::Error::Generic(format!("Identity {} not found on Platform", identity_id))
            })
    }

    /// Refresh DPNS names for all identities in the manager.
    ///
    /// Iterates every identity in the [`IdentityManager`], queries Platform
    /// for its current DPNS usernames, and replaces the stored
    /// `dpns_names` list with the fresh results.
    pub async fn refresh_dpns_names(&self) -> Result<(), PlatformWalletError> {
        use super::managed_identity::key_storage::DpnsNameInfo;

        // Collect identity IDs so we don't hold the lock during network calls.
        let identity_ids: Vec<Identifier> = {
            let manager = self.identity_manager.read().await;
            manager.identities().keys().copied().collect()
        };

        for identity_id in identity_ids {
            match self
                .sdk
                .get_dpns_usernames_by_identity(identity_id, None)
                .await
            {
                Ok(usernames) => {
                    let mut manager = self.identity_manager.write().await;
                    if let Some(managed) = manager.managed_identity_mut(&identity_id) {
                        managed.dpns_names = usernames
                            .into_iter()
                            .map(|u| DpnsNameInfo {
                                label: u.label,
                                acquired_at: None,
                            })
                            .collect();
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to fetch DPNS names for identity {}: {}",
                        identity_id,
                        e
                    );
                }
            }
        }

        Ok(())
    }

    /// Load an identity by resolving a DPNS name.
    ///
    /// Resolves the given `name` to an identity identifier via
    /// [`resolve_name`](Self::resolve_name), fetches the identity from
    /// Platform, and adds it to the **watched** identities collection (since
    /// the wallet derivation index is unknown for externally-resolved names
    /// and we cannot sign on their behalf).
    ///
    /// Returns the identity if the name resolves successfully, or `None` if
    /// the name does not exist.
    pub async fn load_identity_by_dpns_name(
        &self,
        name: &str,
    ) -> Result<Option<Identity>, PlatformWalletError> {
        use dash_sdk::platform::Fetch;

        // Resolve the DPNS name to an identity ID.
        let identity_id = match self.resolve_name(name).await? {
            Some(id) => id,
            None => return Ok(None),
        };

        // Fetch the identity from Platform.
        let identity = Identity::fetch(&self.sdk, identity_id)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch identity {} for DPNS name '{}': {}",
                    identity_id, name, e
                ))
            })?
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "DPNS name '{}' resolved to identity {} but it was not found on Platform",
                    name, identity_id
                ))
            })?;

        // Add to watched identities (read-only — we don't know the wallet
        // index and cannot sign).
        {
            let mut manager = self.identity_manager.write().await;
            manager.add_watched_identity(identity.clone())?;
        }

        Ok(Some(identity))
    }
}
