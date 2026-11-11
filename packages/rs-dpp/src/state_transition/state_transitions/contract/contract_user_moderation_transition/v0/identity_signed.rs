use crate::identity::SecurityLevel::CRITICAL;
use crate::identity::{KeyID, Purpose, SecurityLevel};
use crate::state_transition::contract_user_moderation_transition::v0::ContractUserModerationTransitionV0;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for ContractUserModerationTransitionV0 {
    fn signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = key_id
    }

    /// Moderation is signed like a contract update: a CRITICAL key only.
    fn security_level_requirement(&self, _purpose: Purpose) -> Vec<SecurityLevel> {
        vec![CRITICAL]
    }
}
