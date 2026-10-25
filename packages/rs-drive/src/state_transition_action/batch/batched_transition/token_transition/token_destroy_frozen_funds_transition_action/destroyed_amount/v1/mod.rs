use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::{TokenDestroyFrozenFundsTransitionAction, TokenDestroyFrozenFundsTransitionActionAccessorsV0};
use dpp::balances::credits::TokenAmount;

impl TokenDestroyFrozenFundsTransitionAction {
    /// Generation 1 (protocol version 15): the frozen balance and what its listings hold in
    /// escrow, summed with checked arithmetic.
    pub(super) fn destroyed_amount_v1(&self) -> Result<TokenAmount, Error> {
        self.total_destroyed()
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "a frozen holding is more tokens than a u64 holds",
            )))
    }
}
