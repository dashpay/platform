use crate::consensus::state::identity::IdentityInsufficientBalanceError;
use crate::fee::Credits;
use crate::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use crate::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use crate::state_transition::{
    StateTransitionEstimatedFeeValidation, StateTransitionIdentityEstimatedFeeValidation,
};
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for IdentityCreditWithdrawalTransition {
    fn calculate_estimated_fee(&self, platform_version: &PlatformVersion) -> Credits {
        platform_version
            .fee_version
            .state_transition_min_fees
            .credit_withdrawal
    }
}

impl StateTransitionIdentityEstimatedFeeValidation for IdentityCreditWithdrawalTransition {
    fn validate_estimated_fee(
        &self,
        identity_known_balance: Credits,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        let required_fee = self.calculate_estimated_fee(platform_version);
        let required_total = self.amount().saturating_add(required_fee);

        if identity_known_balance < required_total {
            return SimpleConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(
                    self.identity_id(),
                    identity_known_balance,
                    self.amount(),
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}
