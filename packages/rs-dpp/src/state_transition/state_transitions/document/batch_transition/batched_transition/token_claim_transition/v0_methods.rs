use crate::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_claim_transition::v0::v0_methods::TokenClaimTransitionV0Methods;
use crate::state_transition::batch_transition::TokenClaimTransition;

impl TokenBaseTransitionAccessors for TokenClaimTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenClaimTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenClaimTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenClaimTransition::V0(v0) => v0.base = base,
        }
    }
}
impl TokenClaimTransitionV0Methods for TokenClaimTransition {
    fn distribution_type(&self) -> TokenDistributionType {
        match self {
            TokenClaimTransition::V0(v0) => v0.distribution_type(),
        }
    }

    fn distribution_type_owned(self) -> TokenDistributionType {
        match self {
            TokenClaimTransition::V0(v0) => v0.distribution_type_owned(),
        }
    }

    fn set_distribution_type(&mut self, distribution_type: TokenDistributionType) {
        match self {
            TokenClaimTransition::V0(v0) => v0.set_distribution_type(distribution_type),
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenClaimTransition::V0(v0) => v0.public_note(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenClaimTransition::V0(v0) => v0.public_note_owned(),
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenClaimTransition::V0(v0) => v0.set_public_note(public_note),
        }
    }
}
