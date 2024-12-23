use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_burn_transition::TokenBurnTransitionV0;

impl TokenBaseTransitionAccessors for TokenBurnTransitionV0 {
    fn base(&self) -> &TokenBaseTransition {
        &self.base
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        &mut self.base
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        self.base = base;
    }
}

pub trait TokenBurnTransitionV0Methods: TokenBaseTransitionAccessors {
    fn burn_amount(&self) -> u64;

    fn set_burn_amount(&mut self, amount: u64);
}

impl TokenBurnTransitionV0Methods for TokenBurnTransitionV0 {
    fn burn_amount(&self) -> u64 {
        self.burn_amount
    }

    fn set_burn_amount(&mut self, amount: u64) {
        self.burn_amount = amount;
    }
}
