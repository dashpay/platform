use crate::tokens::token_order_side::TokenOrderSide;
use platform_value::Identifier;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_cancel_order_transition::TokenCancelOrderTransitionV0;

impl TokenBaseTransitionAccessors for TokenCancelOrderTransitionV0 {
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

pub trait TokenCancelOrderTransitionV0Methods: TokenBaseTransitionAccessors {
    fn order_id(&self) -> Identifier;

    fn set_order_id(&mut self, order_id: Identifier);

    fn side(&self) -> TokenOrderSide;

    fn set_side(&mut self, side: TokenOrderSide);
}

impl TokenCancelOrderTransitionV0Methods for TokenCancelOrderTransitionV0 {
    fn order_id(&self) -> Identifier {
        self.order_id
    }

    fn set_order_id(&mut self, order_id: Identifier) {
        self.order_id = order_id;
    }

    fn side(&self) -> TokenOrderSide {
        self.side
    }

    fn set_side(&mut self, side: TokenOrderSide) {
        self.side = side;
    }
}
