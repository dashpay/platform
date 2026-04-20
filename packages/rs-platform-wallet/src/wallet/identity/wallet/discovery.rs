//! Identity discovery via gap-limit HD scan.

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;

use crate::error::PlatformWalletError;

use super::*;

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
        use super::super::managed_identity::key_storage::DpnsNameInfo;
        use super::super::managed_identity::key_storage::IdentityStatus;
        use super::super::managed_identity::key_storage::PrivateKeyData;
        use dash_sdk::platform::types::identity::PublicKeyHash;
        use dash_sdk::platform::Fetch;
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        use dpp::util::hash::ripemd160_sha256;
        use key_wallet::bip32::ChildNumber;
        use key_wallet::bip32::DerivationPath;
        use key_wallet::bip32::KeyDerivationType;
        use key_wallet::dip9::{
            IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
        };

        /// Number of key indices to scan per identity index.
        const KEY_INDEX_SCAN_LIMIT: u32 = 12;

        let (network, start_index, wallet_id) = {
            let wm = self.wallet_manager.read().await;
            let wallet = wm.get_wallet(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet not found in wallet manager".to_string(),
                )
            })?;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            (
                wallet.network,
                info.identity_manager.last_scanned_index(),
                self.wallet_id,
            )
        };

        let mut consecutive_misses = 0u32;
        let mut identity_index = start_index;
        let mut discovered: Vec<Identity> = Vec::new();

        while consecutive_misses < IDENTITY_GAP_LIMIT {
            let mut found_at_this_index = false;

            // Scan key indices 0..KEY_INDEX_SCAN_LIMIT for this identity index.
            for key_index in 0..KEY_INDEX_SCAN_LIMIT {
                let key_hash_array = {
                    let wm = self.wallet_manager.read().await;
                    let wallet = wm.get_wallet(&self.wallet_id).ok_or_else(|| {
                        crate::error::PlatformWalletError::WalletNotFound(
                            "Wallet not found in wallet manager".to_string(),
                        )
                    })?;
                    derive_identity_auth_key_hash(wallet, network, identity_index, key_index)?
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
                        let mut wm_guard = self.wallet_manager.write().await;
                        let info_guard =
                            wm_guard
                                .get_wallet_info_mut(&self.wallet_id)
                                .ok_or_else(|| {
                                    crate::error::PlatformWalletError::WalletNotFound(
                                        "Wallet info not found in wallet manager".to_string(),
                                    )
                                })?;
                        let is_new = info_guard.identity_manager.identity(&identity_id).is_none();
                        if is_new {
                            info_guard.identity_manager.add_identity(
                                identity.clone(),
                                identity_index,
                                &self.persister,
                            )?;
                        }

                        if let Some(managed) = info_guard
                            .identity_manager
                            .managed_identity_mut(&identity_id)
                        {
                            managed.set_status(IdentityStatus::Active, &self.persister);
                            managed.wallet_id = Some(wallet_id);

                            if let Some((kid, pub_key)) = matched_key_id_and_pub {
                                managed.add_key(
                                    kid,
                                    pub_key,
                                    PrivateKeyData::AtWalletDerivationPath {
                                        wallet_id,
                                        derivation_path: full_path,
                                    },
                                    &self.persister,
                                );
                            }
                        }
                        drop(wm_guard);

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
                    let mut wm_guard = self.wallet_manager.write().await;
                    let info_guard =
                        wm_guard
                            .get_wallet_info_mut(&self.wallet_id)
                            .ok_or_else(|| {
                                crate::error::PlatformWalletError::WalletNotFound(
                                    "Wallet info not found in wallet manager".to_string(),
                                )
                            })?;
                    if let Some(managed) = info_guard
                        .identity_manager
                        .managed_identity_mut(&identity_id)
                    {
                        for username in usernames {
                            managed.add_dpns_name(
                                DpnsNameInfo {
                                    label: username.label,
                                    acquired_at: None,
                                },
                                &self.persister,
                            );
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
        let mut wm_guard = self.wallet_manager.write().await;
        let info_guard = wm_guard
            .get_wallet_info_mut(&self.wallet_id)
            .ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
        info_guard
            .identity_manager
            .set_last_scanned_index(identity_index, &self.persister);

        Ok(discovered)
    }
}
