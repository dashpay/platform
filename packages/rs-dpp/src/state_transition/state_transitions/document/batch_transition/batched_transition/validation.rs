use crate::balances::credits::{TokenAmount, MAX_CREDITS, TOKEN_MAX_AMOUNT};
use crate::consensus::basic::token::{
    InvalidTokenAmountError, InvalidTokenNoteTooBigError, InvalidTokenPriceError,
    ZeroTokenAmountError, ZeroTokenPriceError,
};
use crate::fee::Credits;
use crate::tokens::MAX_TOKEN_NOTE_LEN;
use crate::validation::SimpleConsensusValidationResult;

pub(super) fn validate_token_amount_v0(amount: TokenAmount) -> SimpleConsensusValidationResult {
    let mut result = SimpleConsensusValidationResult::default();

    if amount == 0 {
        result.add_error(ZeroTokenAmountError::new());
    }

    if amount > TOKEN_MAX_AMOUNT {
        result.add_error(InvalidTokenAmountError::new(TOKEN_MAX_AMOUNT, amount));
    };

    result
}

pub(super) fn validate_token_price_v0(price: Credits) -> SimpleConsensusValidationResult {
    let mut result = SimpleConsensusValidationResult::default();

    if price == 0 {
        result.add_error(ZeroTokenPriceError::new());
    };

    if price > MAX_CREDITS {
        result.add_error(InvalidTokenPriceError::new(MAX_CREDITS, price));
    }

    result
}

pub(super) fn validate_public_note(public_note: &str) -> SimpleConsensusValidationResult {
    let mut result = SimpleConsensusValidationResult::default();

    if public_note.len() > MAX_TOKEN_NOTE_LEN {
        result.add_error(InvalidTokenNoteTooBigError::new(
            MAX_TOKEN_NOTE_LEN as u32,
            "public_note",
            public_note.len() as u32,
        ));
    }

    result
}
