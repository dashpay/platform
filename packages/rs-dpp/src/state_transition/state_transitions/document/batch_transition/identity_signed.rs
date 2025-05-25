use crate::identity::{KeyID, Purpose, SecurityLevel};
use crate::state_transition::state_transitions::document::batch_transition::BatchTransition;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for BatchTransition {
    fn signature_public_key_id(&self) -> KeyID {
        match self {
            BatchTransition::V0(transition) => transition.signature_public_key_id(),
            BatchTransition::V1(transition) => transition.signature_public_key_id(),
        }
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        match self {
            BatchTransition::V0(transition) => transition.set_signature_public_key_id(key_id),
            BatchTransition::V1(transition) => transition.set_signature_public_key_id(key_id),
        }
    }

    fn security_level_requirement(&self, purpose: Purpose) -> Vec<SecurityLevel> {
        match self {
            BatchTransition::V0(transition) => transition.security_level_requirement(purpose),
            BatchTransition::V1(transition) => transition.security_level_requirement(purpose),
        }
    }

    fn purpose_requirement(&self) -> Vec<Purpose> {
        match self {
            BatchTransition::V0(transition) => transition.purpose_requirement(),
            BatchTransition::V1(transition) => transition.purpose_requirement(),
        }
    }
}
