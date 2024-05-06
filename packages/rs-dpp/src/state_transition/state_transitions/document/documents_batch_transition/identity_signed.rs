use crate::identity::identity_public_key::security_level::SecurityLevel;
use crate::identity::identity_public_key::KeyID;
use crate::state_transition::state_transitions::document::documents_batch_transition::DocumentsBatchTransition;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for DocumentsBatchTransition {
    fn signature_public_key_id(&self) -> KeyID {
        match self {
            DocumentsBatchTransition::V0(transition) => transition.signature_public_key_id(),
        }
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        match self {
            DocumentsBatchTransition::V0(transition) => {
                transition.set_signature_public_key_id(key_id)
            }
        }
    }

    fn security_level_requirement(&self) -> Vec<SecurityLevel> {
        match self {
            DocumentsBatchTransition::V0(transition) => transition.security_level_requirement(),
        }
    }
}
