use platform_value::Identifier;
use crate::fee::Credits;
use crate::prelude::Revision;
use crate::state_transition::batch_transition::batched_transition::token_order_adjust_price_transition::transition::TokenOrderAdjustPriceTransition;
use crate::state_transition::batch_transition::batched_transition::token_order_adjust_price_transition::v0::accessors::TokenOrderAdjustPriceTransitionV0Methods;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;

impl TokenBaseTransitionAccessors for TokenOrderAdjustPriceTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            Self::V0(v0) => &v0.base(),
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            Self::V0(v0) => v0.base_mut(),
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            Self::V0(v0) => v0.set_base(base),
        }
    }
}

impl TokenOrderAdjustPriceTransitionV0Methods for TokenOrderAdjustPriceTransition {
    fn order_id(&self) -> Identifier {
        match self {
            Self::V0(v0) => v0.order_id(),
        }
    }

    fn set_order_id(&mut self, id: Identifier) {
        match self {
            Self::V0(v0) => v0.set_order_id(id),
        }
    }

    fn order_revision(&self) -> Revision {
        match self {
            Self::V0(v0) => v0.order_revision(),
        }
    }

    fn set_order_revision(&mut self, revision: Revision) {
        match self {
            Self::V0(v0) => v0.set_order_revision(revision),
        }
    }

    fn token_price(&self) -> Credits {
        match self {
            Self::V0(v0) => v0.token_price(),
        }
    }

    fn set_token_price(&mut self, price: Credits) {
        match self {
            Self::V0(v0) => v0.set_token_price(price),
        }
    }
}
