use crate::tokens::token_order_side::TokenOrderSide;
use platform_value::Identifier;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_cancel_order_transition::v0::v0_methods::TokenCancelOrderTransitionV0Methods;
use crate::state_transition::batch_transition::TokenCancelOrderTransition;

impl TokenBaseTransitionAccessors for TokenCancelOrderTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenCancelOrderTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenCancelOrderTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenCancelOrderTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenCancelOrderTransitionV0Methods for TokenCancelOrderTransition {
    fn order_id(&self) -> Identifier {
        match self {
            TokenCancelOrderTransition::V0(v0) => v0.order_id(),
        }
    }

    fn set_order_id(&mut self, order_id: Identifier) {
        match self {
            TokenCancelOrderTransition::V0(v0) => v0.set_order_id(order_id),
        }
    }

    fn side(&self) -> TokenOrderSide {
        match self {
            TokenCancelOrderTransition::V0(v0) => v0.side(),
        }
    }

    fn set_side(&mut self, side: TokenOrderSide) {
        match self {
            TokenCancelOrderTransition::V0(v0) => v0.set_side(side),
        }
    }
}
