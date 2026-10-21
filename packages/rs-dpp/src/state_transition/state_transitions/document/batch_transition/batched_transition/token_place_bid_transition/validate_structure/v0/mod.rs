use crate::consensus::basic::token::{
    InvalidTokenAmountError, InvalidTokenOrderFillRangeError, InvalidTokenOrderPriceError,
};
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::state_transition::batch_transition::token_place_bid_transition::v0::v0_methods::TokenPlaceBidTransitionV0Methods;
use crate::state_transition::batch_transition::TokenPlaceBidTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

/// The largest amount and price an order may carry: both are held as signed 8-byte values (an
/// order's fields and its escrow entry), and the order's value is computed from them in u128.
const MAX_ORDER_VALUE: u64 = i64::MAX as u64;

pub(super) trait TokenPlaceBidTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenPlaceBidTransitionStructureValidationV0 for TokenPlaceBidTransition {
    /// The amount and the price are between 1 and `i64::MAX`, and the minimum fill between 1 and
    /// the amount. Whether the whole order's value fits is checked with the batch's advanced
    /// structure, which knows the token's decimals.
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if self.amount() == 0 || self.amount() > MAX_ORDER_VALUE {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::InvalidTokenAmountError(
                    InvalidTokenAmountError::new(MAX_ORDER_VALUE, self.amount()),
                )),
            ));
        }
        if self.price() == 0 || self.price() > MAX_ORDER_VALUE {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidTokenOrderPriceError::new(self.price(), MAX_ORDER_VALUE).into(),
            ));
        }
        if self.min_fill() == 0 || self.min_fill() > self.amount() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidTokenOrderFillRangeError::new(self.min_fill(), self.amount()).into(),
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
    use crate::state_transition::batch_transition::token_place_bid_transition::TokenPlaceBidTransitionV0;

    fn order(amount: u64, price: u64, min_fill: u64) -> TokenPlaceBidTransition {
        TokenPlaceBidTransition::V0(TokenPlaceBidTransitionV0 {
            base: Default::default(),
            amount,
            price,
            min_fill,
            expires_at: None,
        })
    }

    fn errors(transition: TokenPlaceBidTransition) -> Vec<ConsensusError> {
        transition
            .validate_structure_v0()
            .expect("expected the structure validation to run")
            .errors
    }

    #[test]
    fn should_accept_an_order_within_its_bounds() {
        assert!(errors(order(1_000, 1, 1_000)).is_empty());
        assert!(errors(order(MAX_ORDER_VALUE, MAX_ORDER_VALUE, 1)).is_empty());
    }

    #[test]
    fn should_refuse_an_amount_of_0_or_above_i64_max() {
        for amount in [0, MAX_ORDER_VALUE + 1] {
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
        for price in [0, MAX_ORDER_VALUE + 1] {
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
