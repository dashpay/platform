use crate::consensus::basic::state_transition::ShieldedInvalidValueBalanceError;
use crate::consensus::basic::BasicError;
use crate::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
use crate::state_transition::state_transitions::shielded::common_validation::{
    validate_actions_not_empty, validate_anchor_not_zero, validate_proof_not_empty,
};
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for ShieldFromAssetLockTransitionV0 {
    fn validate_structure(
        &self,
        _platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        // Actions must not be empty
        if let Some(err) = validate_actions_not_empty(&self.actions) {
            return err;
        }

        // value_balance must be negative (credits flowing into pool)
        if self.value_balance >= 0 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "shield_from_asset_lock value_balance must be negative".to_string(),
                    ),
                )
                .into(),
            );
        }

        // Proof must not be empty
        if let Some(err) = validate_proof_not_empty(&self.proof) {
            return err;
        }

        // Anchor must not be all zeros
        if let Some(err) = validate_anchor_not_zero(&self.anchor) {
            return err;
        }

        SimpleConsensusValidationResult::new()
    }
}
