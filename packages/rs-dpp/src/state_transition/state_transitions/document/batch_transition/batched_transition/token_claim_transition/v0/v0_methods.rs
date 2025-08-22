use crate::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use crate::state_transition::batch_transition::batched_transition::token_claim_transition::TokenClaimTransitionV0;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;

impl TokenBaseTransitionAccessors for TokenClaimTransitionV0 {
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
pub trait TokenClaimTransitionV0Methods {
    /// Returns the `distribution_type` field of the `TokenClaimTransitionV0`.
    fn distribution_type(&self) -> TokenDistributionType;

    /// Returns the owned `distribution_type` field of the `TokenClaimTransitionV0`.
    fn distribution_type_owned(self) -> TokenDistributionType;

    /// Sets the `distribution_type` field in the `TokenClaimTransitionV0`.
    fn set_distribution_type(&mut self, distribution_type: TokenDistributionType);

    /// Returns the `public_note` field of the `TokenClaimTransitionV0`.
    fn public_note(&self) -> Option<&String>;

    /// Returns the owned `public_note` field of the `TokenClaimTransitionV0`.
    fn public_note_owned(self) -> Option<String>;

    /// Sets the value of the `public_note` field in the `TokenClaimTransitionV0`.
    fn set_public_note(&mut self, public_note: Option<String>);
}

impl TokenClaimTransitionV0Methods for TokenClaimTransitionV0 {
    fn distribution_type(&self) -> TokenDistributionType {
        self.distribution_type
    }

    fn distribution_type_owned(self) -> TokenDistributionType {
        self.distribution_type
    }

    fn set_distribution_type(&mut self, distribution_type: TokenDistributionType) {
        self.distribution_type = distribution_type;
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
