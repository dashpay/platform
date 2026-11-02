use crate::balances::credits::TokenAmount;
use crate::shielded::SerializedAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_shield_transition::TokenShieldTransitionV0;

impl TokenBaseTransitionAccessors for TokenShieldTransitionV0 {
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

pub trait TokenShieldTransitionV0Methods: TokenBaseTransitionAccessors {
    /// Tokens entering the shielded pool.
    fn amount(&self) -> TokenAmount;

    /// Sets the amount entering the shielded pool.
    fn set_amount(&mut self, amount: TokenAmount);

    /// The Orchard actions.
    fn actions(&self) -> &[SerializedAction];

    /// The Orchard anchor the bundle was built against.
    fn anchor(&self) -> &[u8; 32];

    /// The Halo 2 proof bytes.
    fn proof(&self) -> &[u8];

    /// The RedPallas binding signature.
    fn binding_signature(&self) -> &[u8; 64];
}

impl TokenShieldTransitionV0Methods for TokenShieldTransitionV0 {
    fn amount(&self) -> TokenAmount {
        self.amount
    }

    fn set_amount(&mut self, amount: TokenAmount) {
        self.amount = amount;
    }

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
