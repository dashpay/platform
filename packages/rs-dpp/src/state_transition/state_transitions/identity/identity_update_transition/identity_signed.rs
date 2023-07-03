use crate::identity::KeyID;
use crate::state_transition::identity_update_transition::IdentityUpdateTransition;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for IdentityUpdateTransition {
    fn get_signature_public_key_id(&self) -> Option<KeyID> {
        match self {
            IdentityUpdateTransition::V0(transition) => {
                transition.get_signature_public_key_id()
            }
        }
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.set_signature_public_key_id(key_id),
        }
    }
}