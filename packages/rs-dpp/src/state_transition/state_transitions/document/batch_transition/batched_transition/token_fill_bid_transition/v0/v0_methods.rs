use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use platform_value::Identifier;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_fill_bid_transition::TokenFillBidTransitionV0;

impl TokenBaseTransitionAccessors for TokenFillBidTransitionV0 {
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

pub trait TokenFillBidTransitionV0Methods: TokenBaseTransitionAccessors {
    fn order_id(&self) -> Identifier;

    fn set_order_id(&mut self, order_id: Identifier);

    fn min_amount(&self) -> TokenAmount;

    fn set_min_amount(&mut self, min_amount: TokenAmount);

    fn max_amount(&self) -> TokenAmount;

    fn set_max_amount(&mut self, max_amount: TokenAmount);

    fn min_total_received(&self) -> Credits;

    fn set_min_total_received(&mut self, min_total_received: Credits);
}

impl TokenFillBidTransitionV0Methods for TokenFillBidTransitionV0 {
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

    fn min_total_received(&self) -> Credits {
        self.min_total_received
    }

    fn set_min_total_received(&mut self, min_total_received: Credits) {
        self.min_total_received = min_total_received;
    }
}
