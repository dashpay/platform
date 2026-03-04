use crate::consensus::basic::state_transition::ShieldedInvalidValueBalanceError;
use crate::consensus::basic::BasicError;
use crate::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
use crate::state_transition::state_transitions::shielded::common_validation::{
    validate_actions_count, validate_anchor_not_zero, validate_proof_not_empty,
};
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for ShieldFromAssetLockTransitionV0 {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        // Actions count must be in [1, max]
        if let Some(err) = validate_actions_count(
            &self.actions,
            platform_version
                .system_limits
                .max_shielded_transition_actions,
        ) {
            return err;
        }

        // value_balance must be > 0 (credits flowing into pool)
        if self.value_balance == 0 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "shield_from_asset_lock value_balance must be greater than 0".to_string(),
                    ),
                )
                .into(),
            );
        }

        // value_balance must fit in i64 (Orchard protocol uses i64 internally)
        if self.value_balance > i64::MAX as u64 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "shield_from_asset_lock value_balance exceeds i64::MAX".to_string(),
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
