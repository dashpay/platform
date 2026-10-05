use crate::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use crate::shielded::SerializedAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_claim_to_pool_transition::TokenClaimToPoolTransitionV0;

impl TokenBaseTransitionAccessors for TokenClaimToPoolTransitionV0 {
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

pub trait TokenClaimToPoolTransitionV0Methods: TokenBaseTransitionAccessors {
    /// Which distribution is claimed.
    fn distribution_type(&self) -> TokenDistributionType;
    /// For a perpetual distribution, the cycle-aligned moment to claim up to. Ignored for pre-programmed distributions.
    fn claim_up_to(&self) -> Option<u64>;
    /// Orchard actions.
    fn actions(&self) -> &[SerializedAction];
    /// Sinsemilla root of the token pool's note commitment tree (Orchard anchor).
    fn anchor(&self) -> &[u8; 32];
    /// Halo 2 proof bytes.
    fn proof(&self) -> &[u8];
    /// RedPallas binding signature.
    fn binding_signature(&self) -> &[u8; 64];
    /// Optional public note. Only a group action proposer may set one.
    fn public_note(&self) -> Option<&String>;
    /// Takes the public note.
    fn public_note_owned(self) -> Option<String>;
    /// Sets `distribution_type`.
    fn set_distribution_type(&mut self, distribution_type: TokenDistributionType);
    /// Sets `claim_up_to`.
    fn set_claim_up_to(&mut self, claim_up_to: Option<u64>);
    /// Sets `public_note`.
    fn set_public_note(&mut self, public_note: Option<String>);
}

impl TokenClaimToPoolTransitionV0Methods for TokenClaimToPoolTransitionV0 {
    fn distribution_type(&self) -> TokenDistributionType {
        self.distribution_type
    }
    fn claim_up_to(&self) -> Option<u64> {
        self.claim_up_to
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
    fn public_note(&self) -> Option<&String> {
        self.public_note.as_ref()
    }
    fn public_note_owned(self) -> Option<String> {
        self.public_note
    }
    fn set_distribution_type(&mut self, distribution_type: TokenDistributionType) {
        self.distribution_type = distribution_type;
    }
    fn set_claim_up_to(&mut self, claim_up_to: Option<u64>) {
        self.claim_up_to = claim_up_to;
    }
    fn set_public_note(&mut self, public_note: Option<String>) {
        self.public_note = public_note;
    }
}
