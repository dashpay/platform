use crate::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransitionV0Methods;
use crate::state_transition::documents_batch_transition::{
    DocumentsBatchTransition, DocumentsBatchTransitionV0,
};
use crate::state_transition::StateTransitionType::DocumentsBatch;
use crate::state_transition::{StateTransition, StateTransitionLike, StateTransitionType};
use crate::version::FeatureVersion;
use platform_value::{BinaryData, Identifier};

impl From<DocumentsBatchTransitionV0> for StateTransition {
    fn from(value: DocumentsBatchTransitionV0) -> Self {
        let document_batch_transition: DocumentsBatchTransition = value.into();
        document_batch_transition.into()
    }
}

impl StateTransitionLike for DocumentsBatchTransitionV0 {
    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.transitions.iter().map(|t| t.base().id()).collect()]
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
    fn owner_id(&self) -> &Identifier {
        &self.owner_id
    }
}
