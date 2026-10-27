use crate::state_transition::batch_transition::batched_transition::token_order_terms::validate_token_order_terms;
use crate::state_transition::batch_transition::token_place_ask_transition::v0::v0_methods::TokenPlaceAskTransitionV0Methods;
use crate::state_transition::batch_transition::TokenPlaceAskTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait TokenPlaceAskTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenPlaceAskTransitionStructureValidationV0 for TokenPlaceAskTransition {
    /// The listing's terms, checked as an offer's are: see `validate_token_order_terms`.
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        Ok(validate_token_order_terms(self.amount(), self.price()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::state_transition::batch_transition::batched_transition::token_order_terms::MAX_TOKEN_ORDER_TERM;
    use crate::state_transition::batch_transition::token_place_ask_transition::TokenPlaceAskTransitionV0;

    fn order(amount: u64, price: u64) -> TokenPlaceAskTransition {
        TokenPlaceAskTransition::V0(TokenPlaceAskTransitionV0 {
            base: Default::default(),
            amount,
            price,
        })
    }

    fn errors(transition: TokenPlaceAskTransition) -> Vec<ConsensusError> {
        transition
            .validate_structure_v0()
            .expect("expected the structure validation to run")
            .errors
    }

    #[test]
    fn should_accept_an_order_within_its_bounds() {
        assert!(errors(order(1_000, 1)).is_empty());
        // Each term at its bound: what the order is worth at the token's decimals is checked
        // with the batch's advanced structure.
        assert!(errors(order(MAX_TOKEN_ORDER_TERM, 1)).is_empty());
        assert!(errors(order(1, MAX_TOKEN_ORDER_TERM)).is_empty());
    }

    #[test]
    fn should_refuse_an_amount_of_0_or_above_i64_max() {
        for amount in [0, MAX_TOKEN_ORDER_TERM + 1] {
            assert!(matches!(
                errors(order(amount, 1)).as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidTokenAmountError(_)
                )]
            ));
        }
    }

    #[test]
    fn should_refuse_a_price_of_0_or_above_i64_max() {
        for price in [0, MAX_TOKEN_ORDER_TERM + 1] {
            assert!(matches!(
                errors(order(1, price)).as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidTokenOrderPriceError(_)
                )]
            ));
        }
    }
}
