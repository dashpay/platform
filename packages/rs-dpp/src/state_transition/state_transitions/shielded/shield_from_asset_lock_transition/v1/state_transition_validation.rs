use crate::state_transition::shield_from_asset_lock_transition::v0::state_transition_validation::validate_shield_from_asset_lock_structure;
use crate::state_transition::shield_from_asset_lock_transition::v1::ShieldFromAssetLockTransitionV1;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for ShieldFromAssetLockTransitionV1 {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        validate_shield_from_asset_lock_structure(
            &self.actions,
            self.value_balance,
            &self.proof,
            &self.anchor,
            platform_version,
        )
    }
}
