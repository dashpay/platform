use crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use crate::state_transition::{
    StateTransitionStructureValidation, StateTransitionWitnessValidation,
};
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for AddressFundsTransferTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        match self {
            AddressFundsTransferTransition::V0(v0) => v0.validate_structure(platform_version),
        }
    }
}

impl StateTransitionWitnessValidation for AddressFundsTransferTransition {}
