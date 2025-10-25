use crate::prelude::UserFeeIncrease;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use crate::state_transition::batch_transition::{BatchTransition, BatchTransitionV1};
use crate::state_transition::StateTransitionType::Batch;
use crate::state_transition::{StateTransition, StateTransitionLike, StateTransitionSingleSigned, StateTransitionType};
use crate::version::FeatureVersion;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use platform_value::{BinaryData, Identifier};
use crate::state_transition::batch_transition::batched_transition::BatchedTransition;
use crate::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
use crate::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;

impl From<BatchTransitionV1> for StateTransition {
    fn from(value: BatchTransitionV1) -> Self {
        let document_batch_transition: BatchTransition = value.into();
        document_batch_transition.into()
    }
}

impl StateTransitionLike for BatchTransitionV1 {
    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        self.transitions
            .iter()
            .filter_map(|t| match t {
                BatchedTransition::Document(document_transition) => {
                    Some(document_transition.base().id())
                }
                BatchedTransition::Token(_) => None,
            })
            .collect()
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        1
    }
    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        Batch
    }

    /// Get owner ID
    fn owner_id(&self) -> Identifier {
        self.owner_id
    }

    /// We create a list of unique identifiers for the batch
    fn unique_identifiers(&self) -> Vec<String> {
        self.transitions
            .iter()
            .map(|transition| match transition {
                BatchedTransition::Document(document_transition) => {
                    format!(
                        "{}-{}-{:x}",
                        BASE64_STANDARD.encode(self.owner_id),
                        BASE64_STANDARD.encode(document_transition.data_contract_id()),
                        document_transition.identity_contract_nonce()
                    )
                }
                BatchedTransition::Token(token_transition) => {
                    format!(
                        "{}-{}-{:x}",
                        BASE64_STANDARD.encode(self.owner_id),
                        BASE64_STANDARD.encode(token_transition.data_contract_id()),
                        token_transition.identity_contract_nonce()
                    )
                }
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

impl StateTransitionSingleSigned for BatchTransitionV1 {
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
}
