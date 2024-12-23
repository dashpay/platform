use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_issuance_transition::TokenIssuanceTransitionV0;

impl TokenBaseTransitionAccessors for TokenIssuanceTransitionV0 {
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

pub trait TokenIssuanceTransitionV0Methods: TokenBaseTransitionAccessors {
    fn amount(&self) -> u64;

    fn set_amount(&mut self, amount: u64);
}

impl TokenIssuanceTransitionV0Methods for TokenIssuanceTransitionV0 {
    fn amount(&self) -> u64 {
        self.amount
    }

    fn set_amount(&mut self, amount: u64) {
        self.amount = amount;
    }
}
