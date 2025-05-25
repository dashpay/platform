use crate::identity::identity_public_key::security_level::SecurityLevel::{CRITICAL, HIGH, MEDIUM};
use crate::identity::{
    identity_public_key::{Purpose, SecurityLevel},
    KeyID,
};
use crate::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use crate::state_transition::batch_transition::resolvers::v0::BatchTransitionResolversV0;
use crate::state_transition::state_transitions::document::batch_transition::BatchTransitionV1;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for BatchTransitionV1 {
    fn signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = key_id
    }

    fn security_level_requirement(&self, purpose: Purpose) -> Vec<SecurityLevel> {
        if purpose == Purpose::TRANSFER {
            vec![CRITICAL]
        } else {
            // These are the available key levels that must sign the state transition
            // However the fact that it is signed by one of these does not guarantee that it
            // meets the security level requirement, as that is dictated from within the data
            // contract
            vec![CRITICAL, HIGH, MEDIUM]
        }
    }

    fn purpose_requirement(&self) -> Vec<Purpose> {
        if self.transitions_len() == 1 {
            if let Some(first) = self.first_transition() {
                if first.as_transition_token_claim().is_some()
                    || first.as_transition_token_transfer().is_some()
                {
                    return vec![Purpose::AUTHENTICATION, Purpose::TRANSFER];
                }
            }
        }

        vec![Purpose::AUTHENTICATION]
    }
}
