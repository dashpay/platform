use crate::balances::credits::{TokenAmount, TOKEN_MAX_AMOUNT};
use crate::consensus::basic::token::{InvalidTokenAmountError, ZeroTokenAmountError};
use crate::consensus::ConsensusError;

pub(super) trait ValidateTokenAmountV0 {
    fn validate_token_amount_v0(&self, amount: TokenAmount) -> Option<ConsensusError> {
        if amount == 0 {
            return Some(ZeroTokenAmountError::new().into());
        }

        if amount > TOKEN_MAX_AMOUNT {
            return Some(InvalidTokenAmountError::new(TOKEN_MAX_AMOUNT, amount).into());
        };

        None
    }
}
