//! Load identities by HD index / DPNS name, refresh state.

use dpp::identity::accessors::IdentityGettersV0;

use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;

use crate::error::PlatformWalletError;

use super::*;

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
        use crate::wallet::identity::state::managed_identity::key_storage::DpnsNameInfo;
        use crate::wallet::identity::state::managed_identity::key_storage::IdentityStatus;
        use crate::wallet::identity::state::managed_identity::key_storage::PrivateKeyData;
        use dash_sdk::platform::types::identity::PublicKeyHash;
        use dash_sdk::platform::Fetch;
        use dpp::util::hash::ripemd160_sha256;
        use key_wallet::bip32::ChildNumber;
        use key_wallet::bip32::DerivationPath;
        use key_wallet::bip32::KeyDerivationType;
        use key_wallet::dip9::{
            IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
        };

        let (network, wallet_id, key_hash_array) = {
            let wm = self.wallet_manager.read().await;
            let wallet = wm.get_wallet(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet not found in wallet manager".to_string(),
                )
            })?;
            let network = wallet.network;
            let wallet_id = self.wallet_id;
            let key_hash_array = derive_identity_auth_key_hash(wallet, network, identity_index, 0)?;
            (network, wallet_id, key_hash_array)
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
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if info.identity_manager.identity(&identity_id).is_none() {
                info.identity_manager.add_identity(
                    identity.clone(),
                    identity_index,
                    &self.persister,
                )?;
            }

            if let Some(managed) = info.identity_manager.managed_identity_mut(&identity_id) {
                managed.set_status(IdentityStatus::Active, &self.persister);
                managed.wallet_id = Some(wallet_id);

                if let Some((kid, pub_key)) = matched_key_id_and_pub {
                    managed.add_key(
                        kid,
                        pub_key,
                        PrivateKeyData::AtWalletDerivationPath {
                            wallet_id,
                            derivation_path: full_path,
                            identity_index,
                            key_index: 0,
                        },
                        &self.persister,
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
                let mut wm = self.wallet_manager.write().await;
                let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                    crate::error::PlatformWalletError::WalletNotFound(
                        "Wallet info not found in wallet manager".to_string(),
                    )
                })?;
                if let Some(managed) = info.identity_manager.managed_identity_mut(&identity_id) {
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
        use crate::wallet::identity::state::managed_identity::key_storage::IdentityStatus;
        use dash_sdk::platform::Fetch;

        // Verify identity exists in the manager.
        {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if info.identity_manager.identity(identity_id).is_none() {
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
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                managed.identity = identity.clone();
                managed.set_status(IdentityStatus::Active, &self.persister);
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
        use crate::wallet::identity::state::managed_identity::key_storage::DpnsNameInfo;

        // Collect identity IDs so we don't hold the lock during network calls.
        let identity_ids: Vec<Identifier> = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            info.identity_manager.identities().keys().copied().collect()
        };

        for identity_id in identity_ids {
            match self
                .sdk
                .get_dpns_usernames_by_identity(identity_id, None)
                .await
            {
                Ok(usernames) => {
                    let mut wm = self.wallet_manager.write().await;
                    let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                        crate::error::PlatformWalletError::WalletNotFound(
                            "Wallet info not found in wallet manager".to_string(),
                        )
                    })?;
                    if let Some(managed) = info.identity_manager.managed_identity_mut(&identity_id)
                    {
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
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            info.identity_manager
                .add_watched_identity(identity.clone())?;
        }

        Ok(Some(identity))
    }
}
