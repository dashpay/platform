use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::Identifier;
use crate::{Error, Sdk};
use dpp::balances::credits::TokenAmount;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;

pub trait StateTransitionBuilder {
    fn settings(&self) -> Option<PutSettings>;

    async fn sign(
        &self,
        sdk: &Sdk,
        identity_public_key: &IdentityPublicKey,
        signer: &impl Signer,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, Error>;

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
