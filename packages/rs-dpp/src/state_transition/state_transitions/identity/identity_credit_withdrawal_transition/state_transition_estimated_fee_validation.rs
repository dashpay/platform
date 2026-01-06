use crate::consensus::state::identity::IdentityInsufficientBalanceError;
use crate::fee::Credits;
use crate::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use crate::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use crate::state_transition::{
    StateTransitionEstimatedFeeValidation, StateTransitionIdentityEstimatedFeeValidation,
};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for IdentityCreditWithdrawalTransition {
    fn calculate_min_required_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        Ok(platform_version
            .fee_version
            .state_transition_min_fees
            .credit_withdrawal)
    }
}

impl StateTransitionIdentityEstimatedFeeValidation for IdentityCreditWithdrawalTransition {
    fn validate_estimated_fee(
        &self,
        identity_known_balance: Credits,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let required_fee = self.calculate_min_required_fee(platform_version)?;
        let required_total = self.amount().saturating_add(required_fee);

        if identity_known_balance < required_total {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(
                    self.identity_id(),
                    identity_known_balance,
                    required_total,
                )
                .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
