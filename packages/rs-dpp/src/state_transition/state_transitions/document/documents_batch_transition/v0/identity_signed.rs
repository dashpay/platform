use crate::identity::SecurityLevel::HIGH;
use crate::identity::{KeyID, SecurityLevel};

use crate::state_transition::documents_batch_transition::DocumentsBatchTransitionV0;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for DocumentsBatchTransitionV0 {
    fn signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = key_id
    }

    fn security_level_requirement(&self) -> Vec<SecurityLevel> {
        // TODO: should use contract_based_security_level_requirement instead
        vec![HIGH]
    }
}
