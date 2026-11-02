use crate::balances::credits::TokenAmount;
use platform_value::Identifier;
use crate::shielded::SerializedAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_unshield_transition::v0::v0_methods::TokenUnshieldTransitionV0Methods;
use crate::state_transition::batch_transition::TokenUnshieldTransition;

impl TokenBaseTransitionAccessors for TokenUnshieldTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenUnshieldTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenUnshieldTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenUnshieldTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenUnshieldTransitionV0Methods for TokenUnshieldTransition {
    fn recipient_id(&self) -> Identifier {
        match self {
            TokenUnshieldTransition::V0(v0) => v0.recipient_id(),
        }
    }

    fn set_recipient_id(&mut self, recipient_id: Identifier) {
        match self {
            TokenUnshieldTransition::V0(v0) => v0.set_recipient_id(recipient_id),
        }
    }

    fn amount(&self) -> TokenAmount {
        match self {
            TokenUnshieldTransition::V0(v0) => v0.amount(),
        }
    }

    fn set_amount(&mut self, amount: TokenAmount) {
        match self {
            TokenUnshieldTransition::V0(v0) => v0.set_amount(amount),
        }
    }

    fn actions(&self) -> &[SerializedAction] {
        match self {
            TokenUnshieldTransition::V0(v0) => v0.actions(),
        }
    }

    fn anchor(&self) -> &[u8; 32] {
        match self {
            TokenUnshieldTransition::V0(v0) => v0.anchor(),
        }
    }

    fn proof(&self) -> &[u8] {
        match self {
            TokenUnshieldTransition::V0(v0) => v0.proof(),
        }
    }

    fn binding_signature(&self) -> &[u8; 64] {
        match self {
            TokenUnshieldTransition::V0(v0) => v0.binding_signature(),
        }
    }
}
