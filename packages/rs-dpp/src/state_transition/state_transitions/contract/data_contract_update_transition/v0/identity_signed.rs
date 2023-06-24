use platform_value::Identifier;
use crate::identity::KeyID;
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
use crate::state_transition::StateTransitionIdentitySignedV0;

impl StateTransitionIdentitySignedV0 for DataContractUpdateTransitionV0 {
    /// Get owner ID
    fn get_owner_id(&self) -> &Identifier {
        &self.data_contract.owner_id
    }

    fn get_signature_public_key_id(&self) -> Option<KeyID> {
        Some(self.signature_public_key_id)
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = key_id
    }
}