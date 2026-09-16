use crate::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use crate::shielded::SerializedAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_claim_to_pool_transition::v0::v0_methods::TokenClaimToPoolTransitionV0Methods;
use crate::state_transition::batch_transition::TokenClaimToPoolTransition;

impl TokenBaseTransitionAccessors for TokenClaimToPoolTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenClaimToPoolTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenClaimToPoolTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenClaimToPoolTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenClaimToPoolTransitionV0Methods for TokenClaimToPoolTransition {
    fn distribution_type(&self) -> TokenDistributionType {
        match self {
            TokenClaimToPoolTransition::V0(v0) => v0.distribution_type(),
        }
    }
    fn claim_up_to(&self) -> Option<u64> {
        match self {
            TokenClaimToPoolTransition::V0(v0) => v0.claim_up_to(),
        }
    }
    fn actions(&self) -> &[SerializedAction] {
        match self {
            TokenClaimToPoolTransition::V0(v0) => v0.actions(),
        }
    }
    fn anchor(&self) -> &[u8; 32] {
        match self {
            TokenClaimToPoolTransition::V0(v0) => v0.anchor(),
        }
    }
    fn proof(&self) -> &[u8] {
        match self {
            TokenClaimToPoolTransition::V0(v0) => v0.proof(),
        }
    }
    fn binding_signature(&self) -> &[u8; 64] {
        match self {
            TokenClaimToPoolTransition::V0(v0) => v0.binding_signature(),
        }
    }
    fn public_note(&self) -> Option<&String> {
        match self {
            TokenClaimToPoolTransition::V0(v0) => v0.public_note(),
        }
    }
    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenClaimToPoolTransition::V0(v0) => v0.public_note_owned(),
        }
    }
    fn set_distribution_type(&mut self, distribution_type: TokenDistributionType) {
        match self {
            TokenClaimToPoolTransition::V0(v0) => v0.set_distribution_type(distribution_type),
        }
    }
    fn set_claim_up_to(&mut self, claim_up_to: Option<u64>) {
        match self {
            TokenClaimToPoolTransition::V0(v0) => v0.set_claim_up_to(claim_up_to),
        }
    }
    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenClaimToPoolTransition::V0(v0) => v0.set_public_note(public_note),
        }
    }
}
