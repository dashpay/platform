use crate::consensus::basic::token::{
    InvalidTokenAmountError, InvalidTokenOrderFillRangeError, InvalidTokenOrderPriceError,
};
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::state_transition::batch_transition::token_place_ask_transition::v0::v0_methods::TokenPlaceAskTransitionV0Methods;
use crate::state_transition::batch_transition::TokenPlaceAskTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

/// The largest amount and price an order may carry: both are held as signed 8-byte values (an
/// order's fields and its escrow entry), and the order's value is computed from them in u128.
const MAX_ORDER_VALUE: u64 = i64::MAX as u64;

pub(super) trait TokenPlaceAskTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenPlaceAskTransitionStructureValidationV0 for TokenPlaceAskTransition {
    /// The amount and the price are between 1 and `i64::MAX`, and the minimum fill between 1 and
    /// the amount. Whether the whole order's value fits is checked in state validation, which
    /// knows the token's decimals.
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
