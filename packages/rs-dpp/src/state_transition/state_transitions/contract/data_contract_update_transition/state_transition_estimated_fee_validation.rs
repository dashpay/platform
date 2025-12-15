use crate::consensus::state::identity::IdentityInsufficientBalanceError;
use crate::fee::Credits;
use crate::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use crate::state_transition::{
    StateTransitionEstimatedFeeValidation, StateTransitionIdentityEstimatedFeeValidation,
    StateTransitionOwned,
};
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for DataContractUpdateTransition {
    fn calculate_estimated_fee(&self, platform_version: &PlatformVersion) -> Credits {
        let base_fee = platform_version
            .fee_version
            .state_transition_min_fees
            .contract_update;

        let registration_cost = self
            .data_contract()
            .registration_cost(platform_version)
            .unwrap_or(0);

        base_fee.saturating_add(registration_cost)
    }
}

impl StateTransitionIdentityEstimatedFeeValidation for DataContractUpdateTransition {
    fn validate_estimated_fee(
        &self,
        identity_known_balance: Credits,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        let required_fee = self.calculate_estimated_fee(platform_version);

        if identity_known_balance < required_fee {
            return SimpleConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(
                    self.owner_id(),
                    identity_known_balance,
                    required_fee,
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}
