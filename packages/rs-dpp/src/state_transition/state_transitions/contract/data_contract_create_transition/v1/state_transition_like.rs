use platform_value::BinaryData;

use crate::prelude::UserFeeIncrease;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionOwned, StateTransitionType},
};

use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV1;
use crate::state_transition::StateTransitionSingleSigned;
use crate::state_transition::StateTransitionType::DataContractCreate;
use crate::version::FeatureVersion;

impl StateTransitionLike for DataContractCreateTransitionV1 {
    /// Returns ID of the created contract (derived from owner_id + identity_nonce)
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.data_contract_id()]
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        1
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        DataContractCreate
    }

    fn unique_identifiers(&self) -> Vec<String> {
        vec![format!("dcc-{}-{}", self.owner_id, self.data_contract_id())]
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}

impl StateTransitionSingleSigned for DataContractCreateTransitionV1 {
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

impl StateTransitionOwned for DataContractCreateTransitionV1 {
    /// Get owner ID
    fn owner_id(&self) -> Identifier {
        self.owner_id
    }
}
