use crate::state_transition::documents_batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::documents_batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::documents_batch_transition::token_burn_transition::TokenBurnTransition;
use crate::state_transition::documents_batch_transition::token_burn_transition::v0::v0_methods::TokenBurnTransitionV0Methods;

impl TokenBaseTransitionAccessors for TokenBurnTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenBurnTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenBurnTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenBurnTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenBurnTransitionV0Methods for TokenBurnTransition {
    fn burn_amount(&self) -> u64 {
        match self {
            TokenBurnTransition::V0(v0) => v0.burn_amount(),
        }
    }

    fn set_burn_amount(&mut self, burn_amount: u64) {
        match self {
            TokenBurnTransition::V0(v0) => v0.set_burn_amount(burn_amount),
        }
    }
}
