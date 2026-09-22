#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::{Identity, IdentityPublicKey, KeyID};
#[cfg(feature = "state-transition-signing")]
use crate::prelude::{Identifier, IdentityNonce, UserFeeIncrease};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::contract_user_moderation_transition::v0::ContractUserModerationAction;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::version::FeatureVersion;
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

pub trait ContractUserModerationTransitionMethodsV0 {
    /// Builds and signs the transition for `identity`, the moderator. `signing_key_id` must
    /// name a CRITICAL authentication key without contract bounds that `signer` holds the
    /// private key for. `identity_contract_nonce` is the signer's next nonce for the contract.
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
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
    ) -> Result<StateTransition, ProtocolError>;

    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::ContractUserModeration
    }
}
