use crate::identity::SecurityLevel::{CRITICAL, HIGH};
use crate::identity::{KeyID, SecurityLevel};
use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for DataContractCreateTransitionV0 {
    fn signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: crate::identity::KeyID) {
        self.signature_public_key_id = key_id
    }

    fn security_level_requirement(&self) -> Vec<SecurityLevel> {
        vec![CRITICAL, HIGH]
    }
}
