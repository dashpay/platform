use crate::consensus::basic::token::InvalidTokenOrderFillRangeError;
use crate::state_transition::batch_transition::token_fill_ask_transition::v0::v0_methods::TokenFillAskTransitionV0Methods;
use crate::state_transition::batch_transition::TokenFillAskTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait TokenFillAskTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenFillAskTransitionStructureValidationV0 for TokenFillAskTransition {
    /// The fill's range is not empty: 1 <= `min_amount` <= `max_amount` <= `i64::MAX`.
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if self.min_amount() == 0
            || self.min_amount() > self.max_amount()
            || self.max_amount() > i64::MAX as u64
        {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidTokenOrderFillRangeError::new(self.min_amount(), self.max_amount()).into(),
            ));
        }
        Ok(SimpleConsensusValidationResult::default())
    }
}
