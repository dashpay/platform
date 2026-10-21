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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::state_transition::batch_transition::token_fill_ask_transition::TokenFillAskTransitionV0;

    fn fill(min_amount: u64, max_amount: u64) -> TokenFillAskTransition {
        TokenFillAskTransition::V0(TokenFillAskTransitionV0 {
            base: Default::default(),
            min_amount,
            max_amount,
            ..Default::default()
        })
    }

    #[test]
    fn should_accept_a_fill_range_that_is_not_empty() {
        for (min, max) in [(1, 1), (1, i64::MAX as u64)] {
            assert!(fill(min, max)
                .validate_structure_v0()
                .expect("expected the structure validation to run")
                .is_valid());
        }
    }

    #[test]
    fn should_refuse_an_empty_fill_range() {
        for (min, max) in [(0, 1), (2, 1), (1, i64::MAX as u64 + 1)] {
            let result = fill(min, max)
                .validate_structure_v0()
                .expect("expected the structure validation to run");
            assert!(matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidTokenOrderFillRangeError(_)
                )]
            ));
        }
    }
}
