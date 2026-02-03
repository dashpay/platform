use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use platform_value::BinaryData;

use crate::prelude::UserFeeIncrease;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionOwned, StateTransitionType},
};

use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV1;
use crate::state_transition::StateTransitionSingleSigned;
use crate::state_transition::StateTransitionType::DataContractUpdate;
use crate::version::FeatureVersion;

impl StateTransitionLike for DataContractUpdateTransitionV1 {
    /// Returns ID of the updated contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.id]
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        1
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        DataContractUpdate
    }

    fn unique_identifiers(&self) -> Vec<String> {
        vec![format!(
            "{}-{}-{:x}",
            BASE64_STANDARD.encode(self.owner_id),
            BASE64_STANDARD.encode(self.id),
            self.identity_contract_nonce
        )]
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}

impl StateTransitionSingleSigned for DataContractUpdateTransitionV1 {
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

impl StateTransitionOwned for DataContractUpdateTransitionV1 {
    /// Get owner ID
    fn owner_id(&self) -> Identifier {
        self.owner_id
    }
}
