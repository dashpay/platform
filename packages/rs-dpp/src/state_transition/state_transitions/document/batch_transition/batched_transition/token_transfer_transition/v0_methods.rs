use platform_value::Identifier;
use crate::state_transition::batch_transition::batched_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::TokenTransferTransition;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;

impl TokenBaseTransitionAccessors for TokenTransferTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenTransferTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenTransferTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenTransferTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenTransferTransitionV0Methods for TokenTransferTransition {
    fn amount(&self) -> u64 {
        match self {
            TokenTransferTransition::V0(v0) => v0.amount,
        }
    }

    fn set_amount(&mut self, amount: u64) {
        match self {
            TokenTransferTransition::V0(v0) => v0.amount = amount,
        }
    }

    fn recipient_owner_id(&self) -> Identifier {
        match self {
            TokenTransferTransition::V0(v0) => v0.recipient_owner_id,
        }
    }

    fn recipient_owner_id_ref(&self) -> &Identifier {
        match self {
            TokenTransferTransition::V0(v0) => &v0.recipient_owner_id,
        }
    }

    fn set_recipient_owner_id(&mut self, recipient_owner_id: Identifier) {
        match self {
            TokenTransferTransition::V0(v0) => v0.recipient_owner_id = recipient_owner_id,
        }
    }
}
