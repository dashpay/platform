use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::prelude::TimestampMillis;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_place_ask_transition::v0::v0_methods::TokenPlaceAskTransitionV0Methods;
use crate::state_transition::batch_transition::TokenPlaceAskTransition;

impl TokenBaseTransitionAccessors for TokenPlaceAskTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenPlaceAskTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenPlaceAskTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenPlaceAskTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenPlaceAskTransitionV0Methods for TokenPlaceAskTransition {
    fn amount(&self) -> TokenAmount {
        match self {
            TokenPlaceAskTransition::V0(v0) => v0.amount(),
        }
    }

    fn set_amount(&mut self, amount: TokenAmount) {
        match self {
            TokenPlaceAskTransition::V0(v0) => v0.set_amount(amount),
        }
    }

    fn price(&self) -> Credits {
        match self {
            TokenPlaceAskTransition::V0(v0) => v0.price(),
        }
    }

    fn set_price(&mut self, price: Credits) {
        match self {
            TokenPlaceAskTransition::V0(v0) => v0.set_price(price),
        }
    }

    fn decimals(&self) -> u8 {
        match self {
            TokenPlaceAskTransition::V0(v0) => v0.decimals(),
        }
    }

    fn set_decimals(&mut self, decimals: u8) {
        match self {
            TokenPlaceAskTransition::V0(v0) => v0.set_decimals(decimals),
        }
    }

    fn min_fill(&self) -> TokenAmount {
        match self {
            TokenPlaceAskTransition::V0(v0) => v0.min_fill(),
        }
    }

    fn set_min_fill(&mut self, min_fill: TokenAmount) {
        match self {
            TokenPlaceAskTransition::V0(v0) => v0.set_min_fill(min_fill),
        }
    }

    fn expires_at(&self) -> Option<TimestampMillis> {
        match self {
            TokenPlaceAskTransition::V0(v0) => v0.expires_at(),
        }
    }

    fn set_expires_at(&mut self, expires_at: Option<TimestampMillis>) {
        match self {
            TokenPlaceAskTransition::V0(v0) => v0.set_expires_at(expires_at),
        }
    }
}
