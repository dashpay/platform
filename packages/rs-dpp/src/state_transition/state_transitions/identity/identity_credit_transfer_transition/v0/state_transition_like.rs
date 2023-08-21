use platform_value::BinaryData;

use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
use crate::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;

use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType::IdentityCreditTransfer;
use crate::version::FeatureVersion;

impl From<IdentityCreditTransferTransitionV0> for StateTransition {
    fn from(value: IdentityCreditTransferTransitionV0) -> Self {
        let identity_credit_transfer_transition: IdentityCreditTransferTransition = value.into();
        identity_credit_transfer_transition.into()
    }
}

impl StateTransitionLike for IdentityCreditTransferTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        IdentityCreditTransfer
    }
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        &self.signature
    }
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature
    }
    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.identity_id, self.recipient_id]
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = BinaryData::new(signature)
    }

    /// Get owner ID
    fn owner_id(&self) -> Identifier {
        self.identity_id
    }
}
