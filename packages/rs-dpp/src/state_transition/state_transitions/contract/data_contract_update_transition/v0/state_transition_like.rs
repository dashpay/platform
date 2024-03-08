use platform_value::BinaryData;

use crate::prelude::UserFeeMultiplier;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;

use crate::state_transition::StateTransitionType::DataContractUpdate;
use crate::version::FeatureVersion;

impl StateTransitionLike for DataContractUpdateTransitionV0 {
    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.data_contract.id()]
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }
    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        DataContractUpdate
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
        self.data_contract.owner_id()
    }

    fn unique_identifiers(&self) -> Vec<String> {
        vec![format!(
            "{}-{}-{:x}",
            base64::encode(self.data_contract.owner_id()),
            base64::encode(self.data_contract.id()),
            self.identity_contract_nonce
        )]
    }

    fn fee_multiplier(&self) -> UserFeeMultiplier {
        self.fee_multiplier
    }

    fn set_fee_multiplier(&mut self, fee_multiplier: UserFeeMultiplier) {
        self.fee_multiplier = fee_multiplier
    }
}
