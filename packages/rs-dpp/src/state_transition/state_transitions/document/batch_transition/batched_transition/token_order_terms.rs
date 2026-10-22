//! The terms of a new token order (protocol version 15), checked alike for a listing
//! (`TokenPlaceAsk`) and an offer (`TokenPlaceBid`).

use crate::balances::credits::TokenAmount;
use crate::consensus::basic::token::{
    InvalidTokenAmountError, InvalidTokenOrderExpiryError, InvalidTokenOrderFillRangeError,
    InvalidTokenOrderPriceError,
};
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::fee::Credits;
use crate::prelude::TimestampMillis;
use crate::validation::SimpleConsensusValidationResult;

/// The largest amount, price and expiry an order may carry: its document stores each as a signed
/// 8-byte value, and its value is computed from the amount and the price in u128.
pub(crate) const MAX_TOKEN_ORDER_TERM: u64 = i64::MAX as u64;

/// The amount and the price are between 1 and `i64::MAX`, the minimum fill between 1 and the
/// amount, and an expiry at most `i64::MAX`. Whether the whole order's value fits and whether it
/// expires after the block are checked with the batch's advanced structure, which knows the
/// token's decimals and the block.
pub(crate) fn validate_token_order_terms(
    amount: TokenAmount,
    price: Credits,
    min_fill: TokenAmount,
    expires_at: Option<TimestampMillis>,
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
    if let Some(expires_at) = expires_at {
        if expires_at > MAX_TOKEN_ORDER_TERM {
            return SimpleConsensusValidationResult::new_with_error(
                InvalidTokenOrderExpiryError::new(expires_at, MAX_TOKEN_ORDER_TERM).into(),
            );
        }
    }
    SimpleConsensusValidationResult::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn errors(
        amount: TokenAmount,
        price: Credits,
        min_fill: TokenAmount,
        expires_at: Option<TimestampMillis>,
    ) -> Vec<ConsensusError> {
        validate_token_order_terms(amount, price, min_fill, expires_at).errors
    }

    #[test]
    fn should_accept_an_expiry_up_to_i64_max() {
        assert!(errors(1_000, 1, 1, None).is_empty());
        assert!(errors(1_000, 1, 1, Some(0)).is_empty());
        assert!(errors(1_000, 1, 1, Some(MAX_TOKEN_ORDER_TERM)).is_empty());
    }

    #[test]
    fn should_refuse_an_expiry_above_i64_max() {
        for expires_at in [MAX_TOKEN_ORDER_TERM + 1, u64::MAX] {
            assert!(matches!(
                errors(1_000, 1, 1, Some(expires_at)).as_slice(),
                [ConsensusError::BasicError(BasicError::InvalidTokenOrderExpiryError(error))]
                    if error.expires_at() == expires_at
                        && error.max_expires_at() == MAX_TOKEN_ORDER_TERM
            ));
        }
    }
}
