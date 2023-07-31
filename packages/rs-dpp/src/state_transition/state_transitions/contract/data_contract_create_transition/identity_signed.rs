use crate::data_contract::DataContract;
use crate::identity::SecurityLevel::CRITICAL;
use crate::identity::{KeyID, SecurityLevel};
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for DataContractCreateTransition {
    fn signature_public_key_id(&self) -> KeyID {
        match self {
            DataContractCreateTransition::V0(transition) => transition.signature_public_key_id(),
        }
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        match self {
            DataContractCreateTransition::V0(transition) => {
                transition.set_signature_public_key_id(key_id)
            }
        }
    }

    fn security_level_requirement(&self) -> Vec<SecurityLevel> {
        match self {
            DataContractCreateTransition::V0(transition) => transition.security_level_requirement(),
        }
    }
}
