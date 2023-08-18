use platform_value::BinaryData;

use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
use crate::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;

use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType::IdentityCreditWithdrawal;
use crate::version::FeatureVersion;

impl From<IdentityCreditWithdrawalTransitionV0> for StateTransition {
    fn from(value: IdentityCreditWithdrawalTransitionV0) -> Self {
        let identity_credit_withdrawal_transition: IdentityCreditWithdrawalTransition =
            value.into();
        identity_credit_withdrawal_transition.into()
    }
}

impl StateTransitionLike for IdentityCreditWithdrawalTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        IdentityCreditWithdrawal
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
        vec![self.identity_id]
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = BinaryData::new(signature)
    }

    /// Get owner ID
    fn owner_id(&self) -> Identifier {
        self.identity_id
    }
}
