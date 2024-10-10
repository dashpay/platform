use crate::identity::identity_public_key::SecurityLevel::MASTER;
use crate::identity::identity_public_key::{KeyID, Purpose, SecurityLevel};
use crate::state_transition::state_transitions::identity::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for IdentityUpdateTransitionV0 {
    fn signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = key_id
    }

    fn security_level_requirement(&self, _purpose: Purpose) -> Vec<SecurityLevel> {
        vec![MASTER]
    }
}
