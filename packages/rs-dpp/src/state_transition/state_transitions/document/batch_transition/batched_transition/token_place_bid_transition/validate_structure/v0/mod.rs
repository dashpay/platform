use crate::state_transition::batch_transition::batched_transition::token_order_terms::validate_token_order_terms;
use crate::state_transition::batch_transition::token_place_bid_transition::v0::v0_methods::TokenPlaceBidTransitionV0Methods;
use crate::state_transition::batch_transition::TokenPlaceBidTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait TokenPlaceBidTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenPlaceBidTransitionStructureValidationV0 for TokenPlaceBidTransition {
    /// The offer's terms, checked as a listing's are: see `validate_token_order_terms`.
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        Ok(validate_token_order_terms(
            self.amount(),
            self.price(),
            self.decimals(),
            self.min_fill(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::state_transition::batch_transition::batched_transition::token_order_terms::MAX_TOKEN_ORDER_TERM;
    use crate::state_transition::batch_transition::token_place_bid_transition::TokenPlaceBidTransitionV0;

    fn order(amount: u64, price: u64, min_fill: u64) -> TokenPlaceBidTransition {
        TokenPlaceBidTransition::V0(TokenPlaceBidTransitionV0 {
            base: Default::default(),
            amount,
            price,
            decimals: 8,
            min_fill,
        })
    }

    fn errors(transition: TokenPlaceBidTransition) -> Vec<ConsensusError> {
        transition
            .validate_structure_v0()
            .expect("expected the structure validation to run")
            .errors
    }

    /// An offer for a token with 39 or more decimals is worth one credit at most, so terms at
    /// their bounds pass the structure validation there too.
    #[test]
    fn should_accept_an_offer_for_a_token_with_39_or_more_decimals() {
        for decimals in [39, 255] {
            let TokenPlaceBidTransition::V0(mut v0) =
                order(MAX_TOKEN_ORDER_TERM, MAX_TOKEN_ORDER_TERM, 1);
            v0.decimals = decimals;
            assert!(
                errors(TokenPlaceBidTransition::V0(v0)).is_empty(),
                "decimals {decimals}"
            );
        }
    }

    #[test]
    fn should_accept_an_order_within_its_bounds() {
        assert!(errors(order(1_000, 1, 1_000)).is_empty());
        // Each term at its bound, the order worth what an order holds at the test's decimals.
        assert!(errors(order(MAX_TOKEN_ORDER_TERM, 1, 1)).is_empty());
        assert!(errors(order(1, MAX_TOKEN_ORDER_TERM, 1)).is_empty());
    }

    #[test]
    fn should_refuse_an_amount_of_0_or_above_i64_max() {
        for amount in [0, MAX_TOKEN_ORDER_TERM + 1] {
            assert!(matches!(
                errors(order(amount, 1, 1)).as_slice(),
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
                errors(order(1, price, 1)).as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidTokenOrderPriceError(_)
                )]
            ));
        }
    }

    #[test]
    fn should_refuse_a_minimum_fill_of_0_or_above_the_amount() {
        for min_fill in [0, 11] {
            assert!(matches!(
                errors(order(10, 1, min_fill)).as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidTokenOrderFillRangeError(_)
                )]
            ));
        }
    }
}
