use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::state_transition::batch_transition::batched_transition::token_order_sell_limit_transition::v0::transition::TokenOrderSellLimitTransitionV0;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;

impl TokenBaseTransitionAccessors for TokenOrderSellLimitTransitionV0 {
    fn base(&self) -> &TokenBaseTransition {
        &self.base
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        &mut self.base
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        self.base = base;
    }
}

pub trait TokenOrderSellLimitTransitionV0Methods: TokenBaseTransitionAccessors {
    /// How many tokens to sell
    fn token_amount(&self) -> TokenAmount;

    /// Set how many tokens to sell
    fn set_token_amount(&mut self, amount: TokenAmount);

    /// Min price to pay for token's amount
    fn token_min_price(&self) -> Credits;

    /// Set min price to be paid for specified token's amount
    fn set_token_min_price(&mut self, min_price: Credits);
}

impl TokenOrderSellLimitTransitionV0Methods for TokenOrderSellLimitTransitionV0 {
    fn token_amount(&self) -> TokenAmount {
        self.token_amount
    }

    fn set_token_amount(&mut self, amount: TokenAmount) {
        self.token_amount = amount;
    }

    fn token_min_price(&self) -> Credits {
        self.token_min_price
    }

    fn set_token_min_price(&mut self, min_price: Credits) {
        self.token_min_price = min_price;
    }
}
