use platform_value::Identifier;
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

    /// Returns the `public_note` field of the `TokenIssuanceTransitionV0`.
    fn public_note(&self) -> Option<&String>;

    /// Returns the owned `public_note` field of the `TokenIssuanceTransitionV0`.
    fn public_note_owned(self) -> Option<String>;

    /// Sets the value of the `public_note` field in the `TokenIssuanceTransitionV0`.
    fn set_public_note(&mut self, public_note: Option<String>);

    /// Returns the `issued_to_identity_id` field of the `TokenIssuanceTransitionV0`.
    fn issued_to_identity_id(&self) -> Option<Identifier>;

    /// Sets the value of the `issued_to_identity_id` field in the `TokenIssuanceTransitionV0`.
    fn set_issued_to_identity_id(&mut self, issued_to_identity_id: Option<Identifier>);
}

impl TokenIssuanceTransitionV0Methods for TokenIssuanceTransitionV0 {
    fn amount(&self) -> u64 {
        self.amount
    }

    fn set_amount(&mut self, amount: u64) {
        self.amount = amount;
    }

    fn public_note(&self) -> Option<&String> {
        self.public_note.as_ref()
    }

    fn public_note_owned(self) -> Option<String> {
        self.public_note
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        self.public_note = public_note;
    }

    fn issued_to_identity_id(&self) -> Option<Identifier> {
        self.issued_to_identity_id
    }
    fn set_issued_to_identity_id(&mut self, issued_to_identity_id: Option<Identifier>) {
        self.issued_to_identity_id = issued_to_identity_id;
    }
}
