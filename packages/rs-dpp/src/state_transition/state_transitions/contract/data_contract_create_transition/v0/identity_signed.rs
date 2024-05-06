use crate::identity::identity_public_key::security_level::SecurityLevel::CRITICAL;
use crate::identity::identity_public_key::KeyID;
use crate::identity::identity_public_key::security_level::SecurityLevel;
use crate::state_transition::state_transitions::contract::data_contract_create_transition::DataContractCreateTransitionV0;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for DataContractCreateTransitionV0 {
    fn signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: crate::identity::identity_public_key::KeyID) {
        self.signature_public_key_id = key_id
    }

    fn security_level_requirement(&self) -> Vec<SecurityLevel> {
        vec![CRITICAL]
    }
}
