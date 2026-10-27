use crate::balances::credits::TokenAmount;
use crate::shielded::SerializedAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_unshield_transition::TokenUnshieldTransitionV0;
use platform_value::Identifier;

impl TokenBaseTransitionAccessors for TokenUnshieldTransitionV0 {
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

pub trait TokenUnshieldTransitionV0Methods: TokenBaseTransitionAccessors {
    /// Tokens leaving the shielded pool.
    fn amount(&self) -> TokenAmount;

    /// Sets the amount leaving the shielded pool.
    fn set_amount(&mut self, amount: TokenAmount);

    /// The identity credited with the unshielded amount.
    fn recipient_id(&self) -> Identifier;

    /// Sets the identity credited with the unshielded amount.
    fn set_recipient_id(&mut self, recipient_id: Identifier);

    /// The Orchard actions.
    fn actions(&self) -> &[SerializedAction];

    /// The Orchard anchor the bundle was built against.
    fn anchor(&self) -> &[u8; 32];

    /// The Halo 2 proof bytes.
    fn proof(&self) -> &[u8];

    /// The RedPallas binding signature.
    fn binding_signature(&self) -> &[u8; 64];
}

impl TokenUnshieldTransitionV0Methods for TokenUnshieldTransitionV0 {
    fn amount(&self) -> TokenAmount {
        self.amount
    }

    fn set_amount(&mut self, amount: TokenAmount) {
        self.amount = amount;
    }

    fn recipient_id(&self) -> Identifier {
        self.recipient_id
    }

    fn set_recipient_id(&mut self, recipient_id: Identifier) {
        self.recipient_id = recipient_id;
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
