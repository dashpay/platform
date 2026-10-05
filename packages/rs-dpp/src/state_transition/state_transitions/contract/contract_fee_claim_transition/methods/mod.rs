mod v0;
pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::data_contract::document_type::action_fees::ContractFeePot;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::{Identity, IdentityPublicKey, KeyID};
#[cfg(feature = "state-transition-signing")]
use crate::prelude::{Identifier, IdentityNonce, UserFeeIncrease};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::contract_fee_claim_transition::v0::ContractFeeClaimTransitionV0;
use crate::state_transition::contract_fee_claim_transition::ContractFeeClaimTransition;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::version::FeatureVersion;
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl ContractFeeClaimTransitionMethodsV0 for ContractFeeClaimTransition {
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_identity_with_signer<S: Signer<IdentityPublicKey>>(
        identity: &Identity,
        signing_key_id: &KeyID,
        data_contract_id: Identifier,
        pot: ContractFeePot,
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
                .contract_fee_claim_state_transition
                .default_current_version,
        ) {
            0 => Ok(ContractFeeClaimTransitionV0::try_from_identity_with_signer(
                identity,
                signing_key_id,
                data_contract_id,
                pot,
                identity_contract_nonce,
                user_fee_increase,
                signer,
                platform_version,
                version,
            )
            .await?),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown ContractFeeClaimTransition version for try_from_identity_with_signer {v}"
            ))),
        }
    }
}
