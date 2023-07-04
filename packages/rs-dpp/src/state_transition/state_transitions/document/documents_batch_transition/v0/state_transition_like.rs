use platform_value::{BinaryData, Identifier};
use crate::state_transition::documents_batch_transition::DocumentsBatchTransitionV0;
use crate::state_transition::{StateTransitionLike, StateTransitionType};
use crate::state_transition::StateTransitionType::DocumentsBatch;
use crate::version::FeatureVersion;

impl StateTransitionLike for DocumentsBatchTransitionV0 {
    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.transitions.iter().map(|t| t.base().id).collect()]
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }
    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        DocumentsBatch
    }
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        &self.signature
    }
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = BinaryData::new(signature)
    }

    /// Get owner ID
    fn get_owner_id(&self) -> &Identifier {
        &self.owner_id
    }
}
