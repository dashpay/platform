use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_place_bid_transition::TokenPlaceBidTransitionV0;

impl TokenBaseTransitionAccessors for TokenPlaceBidTransitionV0 {
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

pub trait TokenPlaceBidTransitionV0Methods: TokenBaseTransitionAccessors {
    fn amount(&self) -> TokenAmount;

    fn set_amount(&mut self, amount: TokenAmount);

    fn price(&self) -> Credits;

    fn set_price(&mut self, price: Credits);
}

impl TokenPlaceBidTransitionV0Methods for TokenPlaceBidTransitionV0 {
    fn amount(&self) -> TokenAmount {
        self.amount
    }

    fn set_amount(&mut self, amount: TokenAmount) {
        self.amount = amount;
    }

    fn price(&self) -> Credits {
        self.price
    }

    fn set_price(&mut self, price: Credits) {
        self.price = price;
    }
}
