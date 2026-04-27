//! Shared lookups for token state-transition methods on
//! [`IdentityWallet`]. All token actions need to (a) fetch the data
//! contract by id and (b) resolve the canonical AUTHENTICATION /
//! MASTER-or-HIGH / ECDSA_SECP256K1 signing key on the actor identity,
//! so those steps live here rather than being copy-pasted across each
//! action file.

use std::sync::Arc;

use dpp::data_contract::DataContract;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::prelude::Identifier;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::network::IdentityWallet;
use crate::wallet::signer::IdentitySigner;

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Resolve the actor identity, an [`IdentitySigner`] bound to its HD
    /// index, and the AUTHENTICATION signing key — used by the plain
    /// (wallet-internal-signer) token-action variants.
    pub(super) async fn token_resolve_identity_and_signer(
        &self,
        identity_id: &Identifier,
    ) -> Result<(Identity, IdentitySigner, IdentityPublicKey), PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;

        let identity = info
            .identity_manager
            .identity(identity_id)
            .map(|m| m.identity.clone())
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        let identity_index = info
            .identity_manager
            .identity_index(identity_id)
            .ok_or(PlatformWalletError::IdentityIndexNotSet(*identity_id))?;

        let signer = IdentitySigner::new(
            self.wallet_manager.clone(),
            self.wallet_id,
            self.sdk.network,
            identity_index,
        );

        let signing_key = identity
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

        Ok((identity, signer, signing_key))
    }

    /// Resolve only the AUTHENTICATION signing key — used by the
    /// `_with_external_signer` variants where the caller brings the
    /// `Signer` and we just need to pick which on-chain key id to sign
    /// with.
    pub(super) async fn token_resolve_signing_key(
        &self,
        identity_id: &Identifier,
    ) -> Result<IdentityPublicKey, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;

        let identity = info
            .identity_manager
            .identity(identity_id)
            .map(|m| m.identity.clone())
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        let signing_key = identity
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

        Ok(signing_key)
    }

    /// Fetch a data contract by id from Platform.
    pub(super) async fn token_fetch_data_contract(
        &self,
        contract_id: Identifier,
    ) -> Result<Arc<DataContract>, PlatformWalletError> {
        use dash_sdk::platform::Fetch;

        let contract = DataContract::fetch(&self.sdk, contract_id)
            .await
            .map_err(|e| PlatformWalletError::TokenError(format!("Fetch contract failed: {}", e)))?
            .ok_or_else(|| {
                PlatformWalletError::TokenError(format!(
                    "Data contract {} not found on Platform",
                    contract_id
                ))
            })?;
        Ok(Arc::new(contract))
    }
}
