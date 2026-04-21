//! Mutate an identity's public-key set.

use dpp::identity::accessors::IdentityGettersV0;

use dpp::identity::accessors::IdentitySettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::Identity;
use dpp::identity::IdentityPublicKey;
use dpp::identity::KeyType;
use dpp::identity::Purpose;
use dpp::identity::SecurityLevel;
use dpp::prelude::Identifier;

use dpp::identity::signer::Signer;

use dash_sdk::platform::transition::put_settings::PutSettings;

use crate::error::PlatformWalletError;

use super::*;

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
        settings: Option<PutSettings>,
    ) -> Result<(), PlatformWalletError> {
        use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
        use dpp::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
        use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
        use dpp::state_transition::proof_result::StateTransitionProofResult;

        let (mut identity, identity_index) = {
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
            .get_identity_nonce(identity.id(), true, settings)
            .await?;

        let user_fee_increase = settings
            .and_then(|s| s.user_fee_increase)
            .unwrap_or_default();

        // Build the update transition.
        let state_transition = IdentityUpdateTransition::try_from_identity_with_signer(
            &identity,
            &master_key_id,
            add_public_keys,
            disable_public_keys,
            identity_nonce,
            user_fee_increase,
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
            .broadcast_and_wait::<StateTransitionProofResult>(&self.sdk, settings)
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
        settings: Option<PutSettings>,
    ) -> Result<dpp::state_transition::proof_result::StateTransitionProofResult, dash_sdk::Error>
    {
        use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
        use dpp::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
        use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;

        // Get identity nonce from Platform.
        let identity_nonce = self
            .sdk
            .get_identity_nonce(identity.id(), true, settings)
            .await?;

        let user_fee_increase = settings
            .and_then(|s| s.user_fee_increase)
            .unwrap_or_default();

        // Build the update transition.
        let state_transition = IdentityUpdateTransition::try_from_identity_with_signer(
            identity,
            master_key_id,
            add_public_keys,
            disable_public_keys,
            identity_nonce,
            user_fee_increase,
            signer,
            self.sdk.version(),
            None,
        )
        .map_err(|e| dash_sdk::Error::Protocol(e))?;

        // Broadcast and wait for confirmation.
        let result = state_transition
            .broadcast_and_wait(&self.sdk, settings)
            .await?;

        Ok(result)
    }
}
