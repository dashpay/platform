use platform_value::Identifier;
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

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenIssuanceTransition::V0(v0) => v0.public_note(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenIssuanceTransition::V0(v0) => v0.public_note_owned(),
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenIssuanceTransition::V0(v0) => v0.set_public_note(public_note),
        }
    }

    fn issued_to_identity_id(&self) -> Option<Identifier> {
        match self {
            TokenIssuanceTransition::V0(v0) => v0.issued_to_identity_id(),
        }
    }

    fn set_issued_to_identity_id(&mut self, issued_to_identity_id: Option<Identifier>) {
        match self {
            TokenIssuanceTransition::V0(v0) => v0.set_issued_to_identity_id(issued_to_identity_id),
        }
    }
}
