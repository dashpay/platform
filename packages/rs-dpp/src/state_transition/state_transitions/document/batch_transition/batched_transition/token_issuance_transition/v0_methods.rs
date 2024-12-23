use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_issuance_transition::TokenIssuanceTransition;
use crate::state_transition::batch_transition::token_issuance_transition::v0::v0_methods::TokenIssuanceTransitionV0Methods;

impl TokenBaseTransitionAccessors for TokenIssuanceTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenIssuanceTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenIssuanceTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenIssuanceTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenIssuanceTransitionV0Methods for TokenIssuanceTransition {
    fn amount(&self) -> u64 {
        match self {
            TokenIssuanceTransition::V0(v0) => v0.amount(),
        }
    }

    fn set_amount(&mut self, amount: u64) {
        match self {
            TokenIssuanceTransition::V0(v0) => v0.set_amount(amount),
        }
    }
}
