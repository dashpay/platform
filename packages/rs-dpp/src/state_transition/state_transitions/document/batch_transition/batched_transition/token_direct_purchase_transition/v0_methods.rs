use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_direct_purchase_transition::v0::v0_methods::TokenDirectPurchaseTransitionV0Methods;
use crate::state_transition::batch_transition::TokenDirectPurchaseTransition;

impl TokenBaseTransitionAccessors for TokenDirectPurchaseTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenDirectPurchaseTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenDirectPurchaseTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenDirectPurchaseTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenDirectPurchaseTransitionV0Methods for TokenDirectPurchaseTransition {
    fn token_count(&self) -> TokenAmount {
        match self {
            TokenDirectPurchaseTransition::V0(v0) => v0.token_count(),
        }
    }

    fn set_token_count(&mut self, token_count: TokenAmount) {
        match self {
            TokenDirectPurchaseTransition::V0(v0) => v0.set_token_count(token_count),
        }
    }

    fn total_agreed_price(&self) -> Credits {
        match self {
            TokenDirectPurchaseTransition::V0(v0) => v0.total_agreed_price(),
        }
    }

    fn set_total_agreed_price(&mut self, credits: Credits) {
        match self {
            TokenDirectPurchaseTransition::V0(v0) => v0.set_total_agreed_price(credits),
        }
    }
}
