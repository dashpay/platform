use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::state_transition::batch_transition::batched_transition::Entropy;
use crate::state_transition::batch_transition::batched_transition::token_order_buy_limit_transition::v0::transition::TokenOrderBuyLimitTransitionV0;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;

impl TokenBaseTransitionAccessors for TokenOrderBuyLimitTransitionV0 {
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

pub trait TokenOrderBuyLimitTransitionV0Methods: TokenBaseTransitionAccessors {
    /// Entropy generated to create order ID
    fn order_id_entropy(&self) -> Entropy;

    /// How many tokens to buy
    fn token_amount(&self) -> TokenAmount;

    /// Set how many tokens to buy
    fn set_token_amount(&mut self, amount: TokenAmount);

    /// Max price to pay for token's amount
    fn token_max_price(&self) -> Credits;

    /// Set max price to pay for token's amount
    fn set_token_max_price(&mut self, max_price: Credits);
}

impl TokenOrderBuyLimitTransitionV0Methods for TokenOrderBuyLimitTransitionV0 {
    fn order_id_entropy(&self) -> Entropy {
        self.order_id_entropy
    }

    fn token_amount(&self) -> TokenAmount {
        self.token_amount
    }

    fn set_token_amount(&mut self, amount: TokenAmount) {
        self.token_amount = amount;
    }

    fn token_max_price(&self) -> Credits {
        self.token_max_price
    }

    fn set_token_max_price(&mut self, max_price: Credits) {
        self.token_max_price = max_price;
    }
}
