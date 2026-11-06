use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_place_ask_transition::TokenPlaceAskTransitionV0;

impl TokenBaseTransitionAccessors for TokenPlaceAskTransitionV0 {
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

pub trait TokenPlaceAskTransitionV0Methods: TokenBaseTransitionAccessors {
    fn amount(&self) -> TokenAmount;

    fn set_amount(&mut self, amount: TokenAmount);

    fn price(&self) -> Credits;

    fn set_price(&mut self, price: Credits);
}

impl TokenPlaceAskTransitionV0Methods for TokenPlaceAskTransitionV0 {
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
