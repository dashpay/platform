use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use platform_value::Identifier;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_fill_ask_transition::TokenFillAskTransitionV0;

impl TokenBaseTransitionAccessors for TokenFillAskTransitionV0 {
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

pub trait TokenFillAskTransitionV0Methods: TokenBaseTransitionAccessors {
    fn order_id(&self) -> Identifier;

    fn set_order_id(&mut self, order_id: Identifier);

    fn min_amount(&self) -> TokenAmount;

    fn set_min_amount(&mut self, min_amount: TokenAmount);

    fn max_amount(&self) -> TokenAmount;

    fn set_max_amount(&mut self, max_amount: TokenAmount);

    fn max_total_price(&self) -> Credits;

    fn set_max_total_price(&mut self, max_total_price: Credits);
}

impl TokenFillAskTransitionV0Methods for TokenFillAskTransitionV0 {
    fn order_id(&self) -> Identifier {
        self.order_id
    }

    fn set_order_id(&mut self, order_id: Identifier) {
        self.order_id = order_id;
    }

    fn min_amount(&self) -> TokenAmount {
        self.min_amount
    }

    fn set_min_amount(&mut self, min_amount: TokenAmount) {
        self.min_amount = min_amount;
    }

    fn max_amount(&self) -> TokenAmount {
        self.max_amount
    }

    fn set_max_amount(&mut self, max_amount: TokenAmount) {
        self.max_amount = max_amount;
    }

    fn max_total_price(&self) -> Credits {
        self.max_total_price
    }

    fn set_max_total_price(&mut self, max_total_price: Credits) {
        self.max_total_price = max_total_price;
    }
}
