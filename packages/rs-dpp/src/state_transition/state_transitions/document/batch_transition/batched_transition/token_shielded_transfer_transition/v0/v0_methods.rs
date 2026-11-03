use crate::shielded::SerializedAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_shielded_transfer_transition::TokenShieldedTransferTransitionV0;

impl TokenBaseTransitionAccessors for TokenShieldedTransferTransitionV0 {
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

pub trait TokenShieldedTransferTransitionV0Methods: TokenBaseTransitionAccessors {
    /// The Orchard actions.
    fn actions(&self) -> &[SerializedAction];

    /// The Orchard anchor the bundle was built against.
    fn anchor(&self) -> &[u8; 32];

    /// The Halo 2 proof bytes.
    fn proof(&self) -> &[u8];

    /// The RedPallas binding signature.
    fn binding_signature(&self) -> &[u8; 64];
}

impl TokenShieldedTransferTransitionV0Methods for TokenShieldedTransferTransitionV0 {
    fn actions(&self) -> &[SerializedAction] {
        &self.actions
    }

    fn anchor(&self) -> &[u8; 32] {
        &self.anchor
    }

    fn proof(&self) -> &[u8] {
        &self.proof
    }

    fn binding_signature(&self) -> &[u8; 64] {
        &self.binding_signature
    }
}
