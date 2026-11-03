use crate::state_transition::token_shielded_transfer_with_shielded_fee_transition::TokenShieldedTransferWithShieldedFeeTransition;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for TokenShieldedTransferWithShieldedFeeTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        match self {
            TokenShieldedTransferWithShieldedFeeTransition::V0(v0) => {
                v0.validate_structure(platform_version)
            }
        }
    }
}
