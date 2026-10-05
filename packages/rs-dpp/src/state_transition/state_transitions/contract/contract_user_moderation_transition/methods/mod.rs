mod v0;
pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::{Identity, IdentityPublicKey, KeyID};
#[cfg(feature = "state-transition-signing")]
use crate::prelude::{Identifier, IdentityNonce, UserFeeIncrease};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::contract_user_moderation_transition::v0::{
    ContractUserModerationAction, ContractUserModerationTransitionV0,
};
use crate::state_transition::contract_user_moderation_transition::ContractUserModerationTransition;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::version::FeatureVersion;
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl ContractUserModerationTransitionMethodsV0 for ContractUserModerationTransition {
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_identity_with_signer<S: Signer<IdentityPublicKey>>(
        identity: &Identity,
        signing_key_id: &KeyID,
        data_contract_id: Identifier,
        action: ContractUserModerationAction,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .contract_user_moderation_state_transition
                .default_current_version,
        ) {
            0 => Ok(
                ContractUserModerationTransitionV0::try_from_identity_with_signer(
                    identity,
                    signing_key_id,
                    data_contract_id,
                    action,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    version,
                )
                .await?,
            ),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown ContractUserModerationTransition version for try_from_identity_with_signer {v}"
            ))),
        }
    }
}
