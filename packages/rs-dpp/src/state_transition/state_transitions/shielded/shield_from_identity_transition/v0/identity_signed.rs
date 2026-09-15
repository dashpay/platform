use crate::identity::SecurityLevel::CRITICAL;
use crate::identity::{KeyID, Purpose, SecurityLevel};
use crate::state_transition::shield_from_identity_transition::v0::ShieldFromIdentityTransitionV0;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for ShieldFromIdentityTransitionV0 {
    fn signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = key_id
    }

    fn security_level_requirement(&self, _purpose: Purpose) -> Vec<SecurityLevel> {
        vec![CRITICAL]
    }

    fn purpose_requirement(&self) -> Vec<Purpose> {
        vec![Purpose::TRANSFER]
    }
}
