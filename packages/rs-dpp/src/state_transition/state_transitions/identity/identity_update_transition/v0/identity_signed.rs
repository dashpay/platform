use crate::data_contract::DataContract;
use crate::identity::SecurityLevel::MASTER;
use crate::identity::{KeyID, SecurityLevel};
use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::{StateTransitionFieldTypes, StateTransitionIdentitySigned};

impl StateTransitionIdentitySigned for IdentityUpdateTransitionV0 {
    fn signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = key_id
    }

    fn security_level_requirement(&self) -> Vec<SecurityLevel> {
        vec![MASTER]
    }
}
