use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::state_transition::batch_transition::batched_transition::Entropy;
use crate::state_transition::batch_transition::batched_transition::token_order_buy_limit_transition::transition::TokenOrderBuyLimitTransition;
use crate::state_transition::batch_transition::batched_transition::token_order_buy_limit_transition::v0::accessors::TokenOrderBuyLimitTransitionV0Methods;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;

impl TokenBaseTransitionAccessors for TokenOrderBuyLimitTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            Self::V0(v0) => v0.base(),
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

impl TokenOrderBuyLimitTransitionV0Methods for TokenOrderBuyLimitTransition {
    fn order_id_entropy(&self) -> Entropy {
        match self {
            Self::V0(v0) => v0.order_id_entropy(),
        }
    }

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

    fn token_max_price(&self) -> Credits {
        match self {
            Self::V0(v0) => v0.token_max_price(),
        }
    }

    fn set_token_max_price(&mut self, max_price: Credits) {
        match self {
            Self::V0(v0) => v0.set_token_max_price(max_price),
        }
    }
}
