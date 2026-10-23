use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_place_bid_transition::v0::v0_methods::TokenPlaceBidTransitionV0Methods;
use crate::state_transition::batch_transition::TokenPlaceBidTransition;

impl TokenBaseTransitionAccessors for TokenPlaceBidTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenPlaceBidTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenPlaceBidTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenPlaceBidTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenPlaceBidTransitionV0Methods for TokenPlaceBidTransition {
    fn amount(&self) -> TokenAmount {
        match self {
            TokenPlaceBidTransition::V0(v0) => v0.amount(),
        }
    }

    fn set_amount(&mut self, amount: TokenAmount) {
        match self {
            TokenPlaceBidTransition::V0(v0) => v0.set_amount(amount),
        }
    }

    fn price(&self) -> Credits {
        match self {
            TokenPlaceBidTransition::V0(v0) => v0.price(),
        }
    }

    fn set_price(&mut self, price: Credits) {
        match self {
            TokenPlaceBidTransition::V0(v0) => v0.set_price(price),
        }
    }

    fn decimals(&self) -> u8 {
        match self {
            TokenPlaceBidTransition::V0(v0) => v0.decimals(),
        }
    }

    fn set_decimals(&mut self, decimals: u8) {
        match self {
            TokenPlaceBidTransition::V0(v0) => v0.set_decimals(decimals),
        }
    }

    fn min_fill(&self) -> TokenAmount {
        match self {
            TokenPlaceBidTransition::V0(v0) => v0.min_fill(),
        }
    }

    fn set_min_fill(&mut self, min_fill: TokenAmount) {
        match self {
            TokenPlaceBidTransition::V0(v0) => v0.set_min_fill(min_fill),
        }
    }
}
