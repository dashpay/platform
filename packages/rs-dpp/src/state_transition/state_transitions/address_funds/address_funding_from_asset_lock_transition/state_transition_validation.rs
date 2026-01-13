use crate::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use crate::state_transition::{
    StateTransitionStructureValidation, StateTransitionWitnessValidation,
};
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for AddressFundingFromAssetLockTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        match self {
            AddressFundingFromAssetLockTransition::V0(v0) => {
                v0.validate_structure(platform_version)
            }
        }
    }
}

impl StateTransitionWitnessValidation for AddressFundingFromAssetLockTransition {}
