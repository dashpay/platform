use crate::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
use crate::state_transition::{
    StateTransitionStructureValidation, StateTransitionWitnessValidation,
};
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for AddressCreditWithdrawalTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        match self {
            AddressCreditWithdrawalTransition::V0(v0) => v0.validate_structure(platform_version),
        }
    }
}

impl StateTransitionWitnessValidation for AddressCreditWithdrawalTransition {}
