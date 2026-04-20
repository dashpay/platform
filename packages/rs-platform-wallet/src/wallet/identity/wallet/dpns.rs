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
