use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_burn_transition::TokenBurnTransitionV0;

impl TokenBaseTransitionAccessors for TokenBurnTransitionV0 {
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

pub trait TokenBurnTransitionV0Methods: TokenBaseTransitionAccessors {
    fn burn_amount(&self) -> u64;

    fn set_burn_amount(&mut self, amount: u64);

    /// Returns the `public_note` field of the `TokenBurnTransitionV0`.
    fn public_note(&self) -> Option<&String>;

    /// Returns the owned `public_note` field of the `TokenBurnTransitionV0`.
    fn public_note_owned(self) -> Option<String>;

    /// Sets the `public_note` field in the `TokenBurnTransitionV0`.
    fn set_public_note(&mut self, public_note: Option<String>);
}

impl TokenBurnTransitionV0Methods for TokenBurnTransitionV0 {
    fn burn_amount(&self) -> u64 {
        self.burn_amount
    }

    fn set_burn_amount(&mut self, amount: u64) {
        self.burn_amount = amount;
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
}
