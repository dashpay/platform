use crate::consensus::basic::token::ZeroTokenPriceError;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use crate::state_transition::batch_transition::batched_transition::token_order_adjust_price_transition::transition::TokenOrderAdjustPriceTransition;
use crate::state_transition::batch_transition::batched_transition::token_order_adjust_price_transition::v0::accessors::TokenOrderAdjustPriceTransitionV0Methods;

pub(super) trait TokenOrderAdjustPriceTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenOrderAdjustPriceTransitionStructureValidationV0 for TokenOrderAdjustPriceTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if self.token_price() == 0 {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ZeroTokenPriceError::new().into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::default())
    }
}
