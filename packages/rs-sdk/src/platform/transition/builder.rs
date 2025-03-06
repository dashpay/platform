use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::Identifier;
use crate::{Error, Sdk};
use dpp::balances::credits::TokenAmount;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;

/// Trait for building state transitions
#[async_trait::async_trait]
#[async_trait::async_trait]
pub trait StateTransitionBuilder {
    /// Returns the settings for the state transition
    ///
    /// # Returns
    ///
    /// * `Option<PutSettings>` - The settings, if any
    fn settings(&self) -> Option<PutSettings>;

    /// Signs the state transition
    ///
    /// # Arguments
    ///
    /// * `sdk` - The SDK instance
    /// * `identity_public_key` - The public key of the identity
    /// * `signer` - The signer instance
    /// * `platform_version` - The platform version
    ///
    /// # Returns
    ///
    /// * `Result<StateTransition, Error>` - The signed state transition or an error
    async fn sign(
        &self,
        sdk: &Sdk,
        identity_public_key: &IdentityPublicKey,
        signer: &impl Signer,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, Error>;

    /// Broadcasts the state transition
    ///
    /// # Arguments
    ///
    /// * `sdk` - The SDK instance
    /// * `identity_public_key` - The public key of the identity
    /// * `signer` - The signer instance
    /// * `platform_version` - The platform version
    ///
    /// # Returns
    ///
    /// * `Result<StateTransition, Error>` - The broadcasted state transition or an error
    async fn broadcast(
        &self,
        sdk: &Sdk,
        identity_public_key: &IdentityPublicKey,
        signer: &impl Signer,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, Error> {
        let state_transition = self
            .sign(sdk, identity_public_key, signer, platform_version)
            .await?;

        state_transition.broadcast(sdk, self.settings()).await?;

        Ok(state_transition)
    }

    /// Broadcasts the state transition and waits for the result
    ///
    /// # Arguments
    ///
    /// * `sdk` - The SDK instance
    /// * `identity_public_key` - The public key of the identity
    /// * `signer` - The signer instance
    /// * `platform_version` - The platform version
    ///
    /// # Returns
    ///
    /// * `Result<(Identifier, TokenAmount), Error>` - The result of the broadcasted state transition or an error
    async fn broadcast_and_wait_for_result(
        &self,
        sdk: &Sdk,
        identity_public_key: &IdentityPublicKey,
        signer: &impl Signer,
        platform_version: &PlatformVersion,
    ) -> Result<(Identifier, TokenAmount), Error> {
        let state_transition = self
            .broadcast(sdk, identity_public_key, signer, platform_version)
            .await?;

        state_transition
            .wait_for_response(sdk, self.settings())
            .await
    }
}
