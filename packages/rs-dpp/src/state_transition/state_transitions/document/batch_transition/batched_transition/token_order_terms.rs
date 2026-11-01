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

/// The amount and the price are between 1 and `i64::MAX`. What the order needs of the token, its
/// minimum fill and its decimals, is checked with the batch's advanced structure, which knows the
/// token ([`validate_token_order_terms_for_token`]).
pub(crate) fn validate_token_order_terms(
    amount: TokenAmount,
    price: Credits,
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
    SimpleConsensusValidationResult::default()
}

/// What an order of `amount` at `price` needs of its token, given the token's minimum fill and
/// decimals as the token has them when the order is placed: an amount of at least the minimum
/// fill (`InvalidTokenOrderFillRangeError`, the minimum and the amount), so no order opens
/// holding less than a trade, and a worth at the token's decimals (`TokenOrder::order_value`) of
/// at most `i64::MAX` credits (`InvalidTokenOrderValueError`).
pub fn validate_token_order_terms_for_token(
    amount: TokenAmount,
    price: Credits,
    min_fill: TokenAmount,
    decimals: u8,
) -> SimpleConsensusValidationResult {
    if amount < min_fill {
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

    /// An order is worth its amount at its price at the token's decimals, at most `i64::MAX`
    /// credits: 1 base unit at `i64::MAX` credits per whole token is worth exactly that at 0
    /// decimals, 2 are worth more; at 8 decimals 2 are worth far less, and from 39 decimals
    /// any order is worth one credit.
    #[test]
    fn should_refuse_an_order_worth_more_than_an_order_holds() {
        let at = |amount, decimals| {
            validate_token_order_terms_for_token(amount, MAX_TOKEN_ORDER_TERM, 1, decimals).errors
        };
        assert!(at(1, 0).is_empty());
        assert!(at(2, 8).is_empty());
        assert!(at(MAX_TOKEN_ORDER_TERM, 39).is_empty());
        assert!(at(MAX_TOKEN_ORDER_TERM, 255).is_empty());
        assert!(matches!(
            at(2, 0).as_slice(),
            [ConsensusError::BasicError(BasicError::InvalidTokenOrderValueError(error))]
                if error.amount() == 2
                    && error.price() == MAX_TOKEN_ORDER_TERM
                    && error.decimals() == 0
        ));
    }

    /// An order of less than the token's minimum fill is refused; one of exactly it, or more,
    /// is not.
    #[test]
    fn should_refuse_an_order_of_less_than_the_minimum_fill() {
        let at = |amount| validate_token_order_terms_for_token(amount, 1, 100, 2).errors;
        assert!(at(100).is_empty());
        assert!(at(1_000).is_empty());
        assert!(matches!(
            at(99).as_slice(),
            [ConsensusError::BasicError(BasicError::InvalidTokenOrderFillRangeError(error))]
                if error.min() == 100 && error.max() == 99
        ));
    }
}
