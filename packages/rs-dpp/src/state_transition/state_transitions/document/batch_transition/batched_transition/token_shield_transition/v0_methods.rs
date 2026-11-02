use crate::balances::credits::TokenAmount;
use crate::shielded::SerializedAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_shield_transition::v0::v0_methods::TokenShieldTransitionV0Methods;
use crate::state_transition::batch_transition::TokenShieldTransition;

impl TokenBaseTransitionAccessors for TokenShieldTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenShieldTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenShieldTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenShieldTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenShieldTransitionV0Methods for TokenShieldTransition {
    fn amount(&self) -> TokenAmount {
        match self {
            TokenShieldTransition::V0(v0) => v0.amount(),
        }
    }

    fn set_amount(&mut self, amount: TokenAmount) {
        match self {
            TokenShieldTransition::V0(v0) => v0.set_amount(amount),
        }
    }

    fn actions(&self) -> &[SerializedAction] {
        match self {
            TokenShieldTransition::V0(v0) => v0.actions(),
        }
    }

    fn anchor(&self) -> &[u8; 32] {
        match self {
            TokenShieldTransition::V0(v0) => v0.anchor(),
        }
    }

    fn proof(&self) -> &[u8] {
        match self {
            TokenShieldTransition::V0(v0) => v0.proof(),
        }
    }

    fn binding_signature(&self) -> &[u8; 64] {
        match self {
            TokenShieldTransition::V0(v0) => v0.binding_signature(),
        }
    }
}
