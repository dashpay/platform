use crate::identity::identity_public_key::SecurityLevel::CRITICAL;
use crate::identity::identity_public_key::{KeyID, Purpose, SecurityLevel};
use crate::state_transition::state_transitions::contract::data_contract_update_transition::DataContractUpdateTransitionV0;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for DataContractUpdateTransitionV0 {
    fn signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: crate::identity::identity_public_key::KeyID) {
        self.signature_public_key_id = key_id
    }

    fn security_level_requirement(&self, _purpose: Purpose) -> Vec<SecurityLevel> {
        vec![CRITICAL]
    }
}
