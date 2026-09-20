use crate::consensus::state::identity::IdentityInsufficientBalanceError;
use crate::fee::Credits;
use crate::state_transition::contract_user_moderation_transition::ContractUserModerationTransition;
use crate::state_transition::{
    StateTransitionEstimatedFeeValidation, StateTransitionIdentityEstimatedFeeValidation,
    StateTransitionOwned,
};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for ContractUserModerationTransition {
    /// One contract-scoped write, so the contract update floor is reused rather than a new slot
    /// added to the shipped fee schedules. It is a floor, not the price: a ban or a suspension
    /// stores its reason, up to a kilobyte, and like a document's content that storage is
    /// checked against the balance when the fees of the event are validated, from the dry run
    /// of the write.
    fn calculate_min_required_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        Ok(platform_version
            .fee_version
            .state_transition_min_fees
            .contract_update)
    }
}

impl StateTransitionIdentityEstimatedFeeValidation for ContractUserModerationTransition {
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
