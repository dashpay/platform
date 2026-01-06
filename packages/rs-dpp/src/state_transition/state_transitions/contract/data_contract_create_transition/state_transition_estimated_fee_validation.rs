use crate::consensus::state::identity::IdentityInsufficientBalanceError;
use crate::fee::Credits;
use crate::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::state_transition::{
    StateTransitionEstimatedFeeValidation, StateTransitionIdentityEstimatedFeeValidation,
    StateTransitionOwned,
};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for DataContractCreateTransition {
    fn calculate_min_required_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        let base_fee = platform_version
            .fee_version
            .state_transition_min_fees
            .contract_create;

        let registration_cost = self.data_contract().registration_cost(platform_version)?;

        Ok(base_fee.saturating_add(registration_cost))
    }
}

impl StateTransitionIdentityEstimatedFeeValidation for DataContractCreateTransition {
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
