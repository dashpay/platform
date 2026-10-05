use crate::shielded::SerializedAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_shielded_transfer_transition::v0::v0_methods::TokenShieldedTransferTransitionV0Methods;
use crate::state_transition::batch_transition::TokenShieldedTransferTransition;

impl TokenBaseTransitionAccessors for TokenShieldedTransferTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenShieldedTransferTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenShieldedTransferTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenShieldedTransferTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenShieldedTransferTransitionV0Methods for TokenShieldedTransferTransition {
    fn actions(&self) -> &[SerializedAction] {
        match self {
            TokenShieldedTransferTransition::V0(v0) => v0.actions(),
        }
    }

    fn anchor(&self) -> &[u8; 32] {
        match self {
            TokenShieldedTransferTransition::V0(v0) => v0.anchor(),
        }
    }

    fn proof(&self) -> &[u8] {
        match self {
            TokenShieldedTransferTransition::V0(v0) => v0.proof(),
        }
    }

    fn binding_signature(&self) -> &[u8; 64] {
        match self {
            TokenShieldedTransferTransition::V0(v0) => v0.binding_signature(),
        }
    }
}
