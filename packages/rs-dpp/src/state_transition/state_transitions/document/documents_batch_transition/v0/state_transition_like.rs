use crate::prelude::UserFeeIncrease;
use crate::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransitionV0Methods;
use crate::state_transition::documents_batch_transition::{
    DocumentsBatchTransition, DocumentsBatchTransitionV0,
};
use crate::state_transition::StateTransitionType::DocumentsBatch;
use crate::state_transition::{StateTransition, StateTransitionLike, StateTransitionType};
use crate::version::FeatureVersion;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
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
        self.transitions.iter().map(|t| t.base().id()).collect()
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
    fn owner_id(&self) -> Identifier {
        self.owner_id
    }

    /// We create a list of unique identifiers for the batch
    fn unique_identifiers(&self) -> Vec<String> {
        self.transitions
            .iter()
            .map(|transition| {
                format!(
                    "{}-{}-{:x}",
                    BASE64_STANDARD.encode(self.owner_id),
                    BASE64_STANDARD.encode(transition.data_contract_id()),
                    transition.identity_contract_nonce()
                )
            })
            .collect()
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}
