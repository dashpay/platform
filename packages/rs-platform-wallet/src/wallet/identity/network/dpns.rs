//! DPNS name registration, resolution, and search.

use dpp::identity::accessors::IdentityGettersV0;

use dpp::identity::Identity;
use dpp::identity::IdentityPublicKey;
use dpp::identity::KeyType;
use dpp::identity::Purpose;
use dpp::identity::SecurityLevel;
use dpp::prelude::Identifier;

use dpp::identity::signer::Signer;

use crate::error::PlatformWalletError;
use crate::wallet::identity::types::key_storage::DpnsNameInfo;

use super::*;

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
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            let manager = &info.identity_manager;
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

        // Record the just-registered name on the `ManagedIdentity` so
        // subsequent reads (and the persisted snapshot) reflect it
        // without an extra round-trip to Platform. `add_dpns_name`
        // emits an `IdentityChangeSet` via the persister handle.
        //
        // The `acquired_at` timestamp is best-effort wall-clock — the
        // DPNS contract carries its own `$createdAt` on the document
        // but the SDK doesn't surface it back on the register result
        // today. If that changes, swap this for the contract-side
        // value.
        let acquired_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .ok();
        let label_to_store = name.to_string();
        {
            let mut wm = self.wallet_manager.write().await;
            if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                    // Skip if we already have this label recorded —
                    // `add_dpns_name` has no idempotency guard of its
                    // own and would emit a duplicate entry otherwise.
                    if !managed
                        .dpns_names
                        .iter()
                        .any(|existing| existing.label == label_to_store)
                    {
                        managed.add_dpns_name(
                            DpnsNameInfo {
                                label: label_to_store,
                                acquired_at,
                            },
                            &self.persister,
                        );
                    }
                }
            }
        }

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

    /// Fetch all DPNS usernames owned by `identity_id` from Platform
    /// and merge them into the local
    /// [`ManagedIdentity.dpns_names`](crate::wallet::identity::ManagedIdentity)
    /// cache.
    ///
    /// Skips labels that are already in the cache, so repeated syncs
    /// don't emit duplicate entries. New labels get an
    /// `acquired_at` timestamp of best-effort wall-clock millis —
    /// DPNS documents carry their own `$createdAt` but
    /// `DpnsUsername` doesn't surface it on the query result today.
    ///
    /// Returns the number of newly-added labels.
    ///
    /// Use this from the iOS load path instead of
    /// [`Sdk::get_dpns_usernames_by_identity`] directly — the wallet
    /// path additionally updates the persister changeset, so
    /// `PersistentIdentity` + in-app views see the refresh via
    /// `on_persist_identities_fn`.
    pub async fn sync_dpns_names(
        &self,
        identity_id: &Identifier,
    ) -> Result<u32, PlatformWalletError> {
        let usernames = self
            .sdk
            .get_dpns_usernames_by_identity(*identity_id, None)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch DPNS usernames for identity {identity_id}: {e}",
                ))
            })?;

        if usernames.is_empty() {
            return Ok(0);
        }

        let acquired_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .ok();

        let mut added = 0u32;
        {
            let mut wm = self.wallet_manager.write().await;
            let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
                return Ok(0);
            };
            let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) else {
                return Ok(0);
            };
            for username in usernames {
                if managed
                    .dpns_names
                    .iter()
                    .any(|existing| existing.label == username.label)
                {
                    continue;
                }
                managed.add_dpns_name(
                    DpnsNameInfo {
                        label: username.label,
                        acquired_at,
                    },
                    &self.persister,
                );
                added += 1;
            }
        }
        Ok(added)
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
