use crate::consensus::basic::state_transition::{
    ShieldedEmptyProofError, ShieldedInvalidValueBalanceError, ShieldedNoActionsError,
    ShieldedZeroAnchorError, UnshieldAmountZeroError, UnshieldValueBalanceBelowAmountError,
};
use crate::consensus::basic::BasicError;
use crate::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for ShieldedWithdrawalTransitionV0 {
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

        // Amount must be > 0
        if self.amount == 0 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::UnshieldAmountZeroError(UnshieldAmountZeroError::new()).into(),
            );
        }

        // value_balance must be positive (credits flowing out of pool = amount + fee)
        if self.value_balance <= 0 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "shielded withdrawal value_balance must be positive".to_string(),
                    ),
                )
                .into(),
            );
        }

        // value_balance must be >= amount (value_balance = amount + fee)
        if (self.value_balance as u64) < self.amount {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::UnshieldValueBalanceBelowAmountError(
                    UnshieldValueBalanceBelowAmountError::new(self.value_balance, self.amount),
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
