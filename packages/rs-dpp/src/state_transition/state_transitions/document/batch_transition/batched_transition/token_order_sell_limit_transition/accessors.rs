use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::state_transition::batch_transition::batched_transition::token_order_sell_limit_transition::transition::TokenOrderSellLimitTransition;
use crate::state_transition::batch_transition::batched_transition::token_order_sell_limit_transition::v0::accessors::TokenOrderSellLimitTransitionV0Methods;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;

impl TokenBaseTransitionAccessors for TokenOrderSellLimitTransition {
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

impl TokenOrderSellLimitTransitionV0Methods for TokenOrderSellLimitTransition {
    fn token_amount(&self) -> TokenAmount {
        match self {
            Self::V0(v0) => v0.token_amount(),
        }
    }

    fn set_token_amount(&mut self, amount: TokenAmount) {
        match self {
            Self::V0(v0) => v0.set_token_amount(amount),
        }
    }

    fn token_min_price(&self) -> Credits {
        match self {
            Self::V0(v0) => v0.token_min_price(),
        }
    }

    fn set_token_min_price(&mut self, min_price: Credits) {
        match self {
            Self::V0(v0) => v0.set_token_min_price(min_price),
        }
    }
}
