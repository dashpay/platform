//! Identity registration flows.

use std::collections::BTreeMap;
use std::time::Duration;

use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::signer::Signer;
use dpp::identity::v0::IdentityV0;
use dpp::identity::Identity;
use dpp::identity::IdentityPublicKey;
use dpp::identity::KeyID;
use dpp::identity::KeyType;
use dpp::identity::Purpose;
use dpp::identity::SecurityLevel;
use dpp::platform_value::BinaryData;
use dpp::prelude::AssetLockProof;
use dpp::prelude::Identifier;

use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::top_up_identity::TopUpIdentity;

use crate::error::PlatformWalletError;
use crate::wallet::identity::state::managed_identity::PrivateKeyData;

use crate::wallet::identity::types::funding::IdentityFunding;

use super::*;
use crate::wallet::identity::types::funding::IdentityFundingMethod;

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
        settings: Option<PutSettings>,
    ) -> Result<Identity, PlatformWalletError> {
        self.register_identity_with_funding(
            IdentityFundingMethod::FundWithWallet { amount_duffs },
            identity_index,
            key_count,
            settings,
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
        settings: Option<PutSettings>,
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
                let (proof, key, _out_point) = self
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

            let wm = self.wallet_manager.read().await;
            let wallet = wm.get_wallet(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet not found in wallet manager".to_string(),
                )
            })?;
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

        // Extract the outpoint before consuming the proof, in case we need to
        // build a ChainLock proof for recovery.
        let proof_out_point = Self::out_point_from_proof(&asset_lock_proof);

        let identity = match identity
            .put_to_platform_and_wait_for_response(
                &self.sdk,
                asset_lock_proof,
                &asset_lock_private_key,
                &signer,
                settings,
            )
            .await
        {
            Ok(identity) => identity,
            Err(e) if crate::error::is_instant_lock_proof_invalid(&e) => {
                // IS-lock proof was rejected — try to upgrade to ChainLock.
                if let Some(out_point) = proof_out_point {
                    tracing::warn!(
                        "IS-lock proof rejected for identity registration (tx {}), \
                         retrying with ChainLock proof",
                        out_point.txid
                    );
                    let chain_proof = self
                        .asset_locks
                        .upgrade_to_chain_lock_proof(&out_point, Duration::from_secs(180))
                        .await?;
                    identity
                        .put_to_platform_and_wait_for_response(
                            &self.sdk,
                            chain_proof,
                            &asset_lock_private_key,
                            &signer,
                            settings,
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

        // Step 4: Add the identity to the local manager (with its HD
        // index) and record each key's DIP-9 derivation breadcrumb so
        // the client (iOS keychain, etc.) can re-derive + stash the
        // private key on its own side. No key bytes cross this boundary
        // — `add_key` carries `PrivateKeyData::AtWalletDerivationPath`
        // which projects into `(wallet_id, derivation_indices)` on the
        // emitted changeset.
        {
            use dpp::identity::accessors::IdentityGettersV0;
            use key_wallet::bip32::{ChildNumber, DerivationPath, KeyDerivationType};
            use key_wallet::dip9::{
                IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
            };

            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            info.identity_manager.add_identity(
                identity.clone(),
                identity_index,
                &self.persister,
            )?;

            // Rebuild the DIP-9 auth path once per key (cheap — all
            // hardened ChildNumbers, same logic as step 2 above).
            let network = self.sdk.network;
            let base_path: DerivationPath = match network {
                key_wallet::Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
                _ => IDENTITY_AUTHENTICATION_PATH_TESTNET,
            }
            .into();
            let key_type_index: u32 = KeyDerivationType::ECDSA.into();

            let wallet_id = self.wallet_id;
            let identity_id = identity.id();
            // Clone the public-keys map so the loop doesn't hold a
            // borrow of `identity` across the &mut borrow of `info`.
            let public_keys: Vec<(KeyID, IdentityPublicKey)> = identity
                .public_keys()
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect();

            if let Some(managed) = info.identity_manager.managed_identity_mut(&identity_id) {
                managed.wallet_id = Some(wallet_id);
                for (key_id, pub_key) in public_keys {
                    // KeyID == key_index for identities this client
                    // registers (registration loop uses `key_index` as
                    // both the DIP-9 suffix and the DPP KeyID).
                    let key_index = key_id;
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
                    managed.add_key(
                        key_id,
                        pub_key,
                        PrivateKeyData::AtWalletDerivationPath {
                            wallet_id,
                            derivation_path: full_path,
                            identity_index,
                            key_index,
                        },
                        &self.persister,
                    );
                }
            }
        }

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
        settings: Option<PutSettings>,
    ) -> Result<Identity, dash_sdk::Error> {
        identity
            .put_to_platform_and_wait_for_response(
                &self.sdk,
                asset_lock_proof,
                asset_lock_private_key,
                signer,
                settings,
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
        settings: Option<PutSettings>,
    ) -> Result<u64, dash_sdk::Error> {
        identity
            .top_up_identity(
                &self.sdk,
                asset_lock_proof,
                asset_lock_private_key,
                settings.and_then(|s| s.user_fee_increase),
                settings,
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
    /// * **`FromExistingAssetLock`** — resumes a tracked asset lock by outpoint,
    ///   re-deriving the proof and private key from whatever stage the lock
    ///   is at.
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
        settings: Option<PutSettings>,
    ) -> Result<Identity, PlatformWalletError> {
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

        let (asset_lock_proof, asset_lock_private_key, tracked_out_point) = match funding {
            IdentityFunding::FromWalletBalance { amount_duffs } => {
                let (proof, key, out_point) = self
                    .asset_locks
                    .create_funded_asset_lock_proof(
                        amount_duffs,
                        0,
                        AssetLockFundingType::IdentityRegistration,
                        identity_index,
                    )
                    .await?;
                (proof, key, Some(out_point))
            }
            IdentityFunding::FromExistingAssetLock { out_point } => {
                let (proof, key) = self
                    .asset_locks
                    .resume_asset_lock(&out_point, Duration::from_secs(300))
                    .await?;
                (proof, key, Some(out_point))
            }
        };

        // Extract the outpoint before consuming the proof, in case we need to
        // build a ChainLock proof for recovery.
        let proof_out_point = Self::out_point_from_proof(&asset_lock_proof);

        let result = match self
            .register_identity_with_signer(
                identity,
                asset_lock_proof,
                &asset_lock_private_key,
                signer,
                settings,
            )
            .await
        {
            Ok(identity) => identity,
            Err(e) if crate::error::is_instant_lock_proof_invalid(&e) => {
                if let Some(out_point) = proof_out_point {
                    tracing::warn!(
                        "IS-lock proof rejected for funded identity registration (tx {}), \
                         retrying with ChainLock proof",
                        out_point.txid
                    );
                    let chain_proof = self
                        .asset_locks
                        .upgrade_to_chain_lock_proof(&out_point, Duration::from_secs(180))
                        .await?;
                    self.register_identity_with_signer(
                        identity,
                        chain_proof,
                        &asset_lock_private_key,
                        signer,
                        settings,
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
        if let Some(out_point) = tracked_out_point {
            self.asset_locks.remove_asset_lock(&out_point).await;
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
    /// * **`FromExistingAssetLock`** — resumes a tracked asset lock by outpoint,
    ///   re-deriving the proof and private key from whatever stage the lock
    ///   is at.
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
        settings: Option<PutSettings>,
    ) -> Result<u64, PlatformWalletError> {
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

        let (asset_lock_proof, asset_lock_private_key, tracked_out_point) = match funding {
            IdentityFunding::FromWalletBalance { amount_duffs } => {
                let (proof, key, out_point) = self
                    .asset_locks
                    .create_funded_asset_lock_proof(
                        amount_duffs,
                        0,
                        AssetLockFundingType::IdentityTopUp,
                        identity_index,
                    )
                    .await?;
                (proof, key, Some(out_point))
            }
            IdentityFunding::FromExistingAssetLock { out_point } => {
                let (proof, key) = self
                    .asset_locks
                    .resume_asset_lock(&out_point, Duration::from_secs(300))
                    .await?;
                (proof, key, Some(out_point))
            }
        };

        // Extract the outpoint before consuming the proof, in case we need to
        // build a ChainLock proof for recovery.
        let proof_out_point = Self::out_point_from_proof(&asset_lock_proof);

        let new_balance = match self
            .top_up_identity_with_signer(
                identity,
                asset_lock_proof,
                &asset_lock_private_key,
                settings,
            )
            .await
        {
            Ok(balance) => balance,
            Err(e) if crate::error::is_instant_lock_proof_invalid(&e) => {
                if let Some(out_point) = proof_out_point {
                    tracing::warn!(
                        "IS-lock proof rejected for funded identity top-up (tx {}), \
                         retrying with ChainLock proof",
                        out_point.txid
                    );
                    let chain_proof = self
                        .asset_locks
                        .upgrade_to_chain_lock_proof(&out_point, Duration::from_secs(180))
                        .await?;
                    self.top_up_identity_with_signer(
                        identity,
                        chain_proof,
                        &asset_lock_private_key,
                        settings,
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
        if let Some(out_point) = tracked_out_point {
            self.asset_locks.remove_asset_lock(&out_point).await;
        }

        Ok(new_balance)
    }
}
