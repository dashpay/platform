use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use platform_value::Identifier;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_fill_bid_transition::v0::v0_methods::TokenFillBidTransitionV0Methods;
use crate::state_transition::batch_transition::TokenFillBidTransition;

impl TokenBaseTransitionAccessors for TokenFillBidTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenFillBidTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenFillBidTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenFillBidTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenFillBidTransitionV0Methods for TokenFillBidTransition {
    fn order_id(&self) -> Identifier {
        match self {
            TokenFillBidTransition::V0(v0) => v0.order_id(),
        }
    }

    fn set_order_id(&mut self, order_id: Identifier) {
        match self {
            TokenFillBidTransition::V0(v0) => v0.set_order_id(order_id),
        }
    }

    fn min_amount(&self) -> TokenAmount {
        match self {
            TokenFillBidTransition::V0(v0) => v0.min_amount(),
        }
    }

    fn set_min_amount(&mut self, min_amount: TokenAmount) {
        match self {
            TokenFillBidTransition::V0(v0) => v0.set_min_amount(min_amount),
        }
    }

    fn max_amount(&self) -> TokenAmount {
        match self {
            TokenFillBidTransition::V0(v0) => v0.max_amount(),
        }
    }

    fn set_max_amount(&mut self, max_amount: TokenAmount) {
        match self {
            TokenFillBidTransition::V0(v0) => v0.set_max_amount(max_amount),
        }
    }

    fn min_total_received(&self) -> Credits {
        match self {
            TokenFillBidTransition::V0(v0) => v0.min_total_received(),
        }
    }

    fn set_min_total_received(&mut self, min_total_received: Credits) {
        match self {
            TokenFillBidTransition::V0(v0) => v0.set_min_total_received(min_total_received),
        }
    }
}
