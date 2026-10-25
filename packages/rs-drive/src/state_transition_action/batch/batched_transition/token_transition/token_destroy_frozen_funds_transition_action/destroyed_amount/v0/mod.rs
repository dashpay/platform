use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::{TokenDestroyFrozenFundsTransitionAction, TokenDestroyFrozenFundsTransitionActionAccessorsV0};
use dpp::balances::credits::TokenAmount;

impl TokenDestroyFrozenFundsTransitionAction {
    /// Generation 0 (protocol version 14 and earlier): the frozen balance alone.
    pub(super) fn destroyed_amount_v0(&self) -> TokenAmount {
        self.amount()
    }
}
