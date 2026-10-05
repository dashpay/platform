use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::shielded::SerializedAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_direct_purchase_to_pool_transition::v0::v0_methods::TokenDirectPurchaseToPoolTransitionV0Methods;
use crate::state_transition::batch_transition::TokenDirectPurchaseToPoolTransition;

impl TokenBaseTransitionAccessors for TokenDirectPurchaseToPoolTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenDirectPurchaseToPoolTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenDirectPurchaseToPoolTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenDirectPurchaseToPoolTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenDirectPurchaseToPoolTransitionV0Methods for TokenDirectPurchaseToPoolTransition {
    fn token_count(&self) -> TokenAmount {
        match self {
            TokenDirectPurchaseToPoolTransition::V0(v0) => v0.token_count(),
        }
    }
    fn total_agreed_price(&self) -> Credits {
        match self {
            TokenDirectPurchaseToPoolTransition::V0(v0) => v0.total_agreed_price(),
        }
    }
    fn actions(&self) -> &[SerializedAction] {
        match self {
            TokenDirectPurchaseToPoolTransition::V0(v0) => v0.actions(),
        }
    }
    fn anchor(&self) -> &[u8; 32] {
        match self {
            TokenDirectPurchaseToPoolTransition::V0(v0) => v0.anchor(),
        }
    }
    fn proof(&self) -> &[u8] {
        match self {
            TokenDirectPurchaseToPoolTransition::V0(v0) => v0.proof(),
        }
    }
    fn binding_signature(&self) -> &[u8; 64] {
        match self {
            TokenDirectPurchaseToPoolTransition::V0(v0) => v0.binding_signature(),
        }
    }
    fn set_token_count(&mut self, token_count: TokenAmount) {
        match self {
            TokenDirectPurchaseToPoolTransition::V0(v0) => v0.set_token_count(token_count),
        }
    }
    fn set_total_agreed_price(&mut self, total_agreed_price: Credits) {
        match self {
            TokenDirectPurchaseToPoolTransition::V0(v0) => {
                v0.set_total_agreed_price(total_agreed_price)
            }
        }
    }
}
