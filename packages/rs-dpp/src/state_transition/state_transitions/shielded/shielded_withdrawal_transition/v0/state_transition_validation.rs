use crate::consensus::basic::state_transition::{
    ShieldedInvalidValueBalanceError, UnshieldAmountZeroError, UnshieldValueBalanceBelowAmountError,
};
use crate::consensus::basic::BasicError;
use crate::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
use crate::state_transition::state_transitions::shielded::common_validation::{
    validate_actions_not_empty, validate_anchor_not_zero, validate_proof_not_empty,
};
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for ShieldedWithdrawalTransitionV0 {
    fn validate_structure(
        &self,
        _platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        // Actions must not be empty
        if let Some(err) = validate_actions_not_empty(&self.actions) {
            return err;
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
