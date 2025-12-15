use crate::consensus::state::identity::IdentityInsufficientBalanceError;
use crate::fee::Credits;
use crate::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use crate::state_transition::identity_update_transition::IdentityUpdateTransition;
use crate::state_transition::{
    StateTransitionEstimatedFeeValidation, StateTransitionIdentityEstimatedFeeValidation,
};
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for IdentityUpdateTransition {
    fn calculate_estimated_fee(&self, platform_version: &PlatformVersion) -> Credits {
        platform_version
            .fee_version
            .state_transition_min_fees
            .identity_update
    }
}

impl StateTransitionIdentityEstimatedFeeValidation for IdentityUpdateTransition {
    fn validate_estimated_fee(
        &self,
        identity_known_balance: Credits,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        let required_fee = self.calculate_estimated_fee(platform_version);

        if identity_known_balance < required_fee {
            return SimpleConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(
                    self.identity_id(),
                    identity_known_balance,
                    required_fee,
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}
