use crate::consensus::basic::state_transition::{
    ShieldedEmptyProofError, ShieldedInvalidValueBalanceError, ShieldedNoActionsError,
    ShieldedZeroAnchorError,
};
use crate::consensus::basic::BasicError;
use crate::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for ShieldedTransferTransitionV0 {
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

        // value_balance must be >= 0 (fee extracted from pool, 0 means no fee)
        if self.value_balance < 0 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "shielded transfer value_balance must be non-negative (fee only)"
                            .to_string(),
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

        // Anchor must not be all zeros
        if self.anchor == [0u8; 32] {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedZeroAnchorError(ShieldedZeroAnchorError::new()).into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}
