use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use crate::state_transition::batch_transition::batched_transition::token_order_adjust_price_transition::transition::TokenOrderAdjustPriceTransition;
use crate::state_transition::batch_transition::batched_transition::token_order_adjust_price_transition::v0::accessors::TokenOrderAdjustPriceTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::validation::validate_token_price_v0;

pub(super) trait TokenOrderAdjustPriceTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenOrderAdjustPriceTransitionStructureValidationV0 for TokenOrderAdjustPriceTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let result = validate_token_price_v0(self.token_price());

        Ok(result)
    }
}
