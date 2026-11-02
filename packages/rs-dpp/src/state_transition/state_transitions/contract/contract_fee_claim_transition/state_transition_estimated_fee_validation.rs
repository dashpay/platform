use crate::consensus::state::identity::IdentityInsufficientBalanceError;
use crate::fee::Credits;
use crate::state_transition::contract_fee_claim_transition::ContractFeeClaimTransition;
use crate::state_transition::{
    StateTransitionEstimatedFeeValidation, StateTransitionIdentityEstimatedFeeValidation,
    StateTransitionOwned,
};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for ContractFeeClaimTransition {
    /// A claim credits balances as a credit transfer does, so that floor is reused rather than
    /// a new slot added to the shipped fee schedules.
    fn calculate_min_required_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        Ok(platform_version
            .fee_version
            .state_transition_min_fees
            .credit_transfer)
    }
}

impl StateTransitionIdentityEstimatedFeeValidation for ContractFeeClaimTransition {
    fn validate_estimated_fee(
        &self,
        identity_known_balance: Credits,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let required_fee = self.calculate_min_required_fee(platform_version)?;

        if identity_known_balance < required_fee {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(
                    self.owner_id(),
                    identity_known_balance,
                    required_fee,
                )
                .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
