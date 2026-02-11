use crate::consensus::basic::state_transition::{
    ShieldedEmptyProofError, ShieldedInvalidValueBalanceError, ShieldedNoActionsError,
};
use crate::consensus::basic::BasicError;
use crate::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for ShieldFromAssetLockTransitionV0 {
    fn validate_structure(
        &self,
        _platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        // Actions must not be empty
        if self.actions.is_empty() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedNoActionsError(ShieldedNoActionsError::new()).into(),
            );
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
        if self.proof.is_empty() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedEmptyProofError(ShieldedEmptyProofError::new()).into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}
