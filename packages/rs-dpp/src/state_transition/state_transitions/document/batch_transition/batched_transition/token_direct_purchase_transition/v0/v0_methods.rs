use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_direct_purchase_transition::TokenDirectPurchaseTransitionV0;

impl TokenBaseTransitionAccessors for TokenDirectPurchaseTransitionV0 {
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

pub trait TokenDirectPurchaseTransitionV0Methods: TokenBaseTransitionAccessors {
    fn token_count(&self) -> TokenAmount;

    fn set_token_count(&mut self, token_count: TokenAmount);

    fn total_agreed_price(&self) -> Credits;

    fn set_total_agreed_price(&mut self, credits: Credits);
}

impl TokenDirectPurchaseTransitionV0Methods for TokenDirectPurchaseTransitionV0 {
    fn token_count(&self) -> TokenAmount {
        self.token_count
    }

    fn set_token_count(&mut self, token_count: TokenAmount) {
        self.token_count = token_count;
    }

    fn total_agreed_price(&self) -> Credits {
        self.total_agreed_price
    }

    fn set_total_agreed_price(&mut self, credits: Credits) {
        self.total_agreed_price = credits;
    }
}
