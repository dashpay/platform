//! The terms of a new token order (protocol version 15), checked alike for a listing
//! (`TokenPlaceAsk`) and an offer (`TokenPlaceBid`).

use crate::balances::credits::TokenAmount;
use crate::consensus::basic::token::{
    InvalidTokenAmountError, InvalidTokenOrderFillRangeError, InvalidTokenOrderPriceError,
    InvalidTokenOrderValueError,
};
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::fee::Credits;
use crate::tokens::token_order::TokenOrder;
use crate::validation::SimpleConsensusValidationResult;

/// The largest amount and price an order may carry: its document stores each as a signed
/// 8-byte value, and its value is computed from the amount and the price in u128.
pub(crate) const MAX_TOKEN_ORDER_TERM: u64 = i64::MAX as u64;

/// The amount and the price are between 1 and `i64::MAX`, the minimum fill between 1 and the
/// amount, and what the order is worth at the decimals it signs (`TokenOrder::order_value`) at
/// most `i64::MAX` credits. Whether the signed decimals are the token's is checked with the
/// batch's advanced structure, which knows the token.
pub(crate) fn validate_token_order_terms(
    amount: TokenAmount,
    price: Credits,
    decimals: u8,
    min_fill: TokenAmount,
) -> SimpleConsensusValidationResult {
    if amount == 0 || amount > MAX_TOKEN_ORDER_TERM {
        return SimpleConsensusValidationResult::new_with_error(ConsensusError::BasicError(
            BasicError::InvalidTokenAmountError(InvalidTokenAmountError::new(
                MAX_TOKEN_ORDER_TERM,
                amount,
            )),
        ));
    }
    if price == 0 || price > MAX_TOKEN_ORDER_TERM {
        return SimpleConsensusValidationResult::new_with_error(
            InvalidTokenOrderPriceError::new(price, MAX_TOKEN_ORDER_TERM).into(),
        );
    }
    if min_fill == 0 || min_fill > amount {
        return SimpleConsensusValidationResult::new_with_error(
            InvalidTokenOrderFillRangeError::new(min_fill, amount).into(),
        );
    }
    if TokenOrder::order_value(amount, price, decimals).is_none() {
        return SimpleConsensusValidationResult::new_with_error(
            InvalidTokenOrderValueError::new(amount, price, decimals).into(),
        );
    }
    SimpleConsensusValidationResult::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An order is worth its amount at its price at the decimals it signs, at most `i64::MAX`
    /// credits: 1 base unit at `i64::MAX` credits per whole token is worth exactly that at 0
    /// decimals, 2 are worth more; at 8 decimals 2 are worth far less.
    #[test]
    fn should_refuse_an_order_worth_more_than_an_order_holds() {
        let at = |amount, decimals| {
            validate_token_order_terms(amount, MAX_TOKEN_ORDER_TERM, decimals, 1).errors
        };
        assert!(at(1, 0).is_empty());
        assert!(at(2, 8).is_empty());
        assert!(matches!(
            at(2, 0).as_slice(),
            [ConsensusError::BasicError(BasicError::InvalidTokenOrderValueError(error))]
                if error.amount() == 2
                    && error.price() == MAX_TOKEN_ORDER_TERM
                    && error.decimals() == 0
        ));
    }
}
