use crate::identity::SecurityLevel::CRITICAL;
use crate::identity::{KeyID, SecurityLevel};
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for DataContractUpdateTransitionV0 {
    fn signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: crate::identity::KeyID) {
        self.signature_public_key_id = key_id
    }

    fn security_level_requirement(&self) -> Vec<SecurityLevel> {
        vec![CRITICAL]
    }
}
