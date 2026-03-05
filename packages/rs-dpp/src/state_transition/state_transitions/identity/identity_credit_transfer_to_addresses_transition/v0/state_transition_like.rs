use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use platform_value::BinaryData;

use crate::prelude::UserFeeIncrease;
use crate::state_transition::StateTransitionHasUserFeeIncrease;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionOwned, StateTransitionType},
};

use crate::state_transition::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;
use crate::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;

use crate::state_transition::{StateTransition, StateTransitionSingleSigned};
use crate::version::FeatureVersion;

impl From<IdentityCreditTransferToAddressesTransitionV0> for StateTransition {
    fn from(value: IdentityCreditTransferToAddressesTransitionV0) -> Self {
        let identity_credit_transfer_to_addresses_transition: IdentityCreditTransferToAddressesTransition = value.into();
        identity_credit_transfer_to_addresses_transition.into()
    }
}

impl StateTransitionLike for IdentityCreditTransferToAddressesTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        StateTransitionType::IdentityCreditTransferToAddresses
    }

    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.identity_id]
    }

    /// We want things to be unique based on the nonce, so we don't add the transition type
    fn unique_identifiers(&self) -> Vec<String> {
        vec![format!(
            "{}-{:x}",
            BASE64_STANDARD.encode(self.identity_id),
            self.nonce
        )]
    }

}

impl StateTransitionHasUserFeeIncrease for IdentityCreditTransferToAddressesTransitionV0 {
    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}

impl StateTransitionSingleSigned for IdentityCreditTransferToAddressesTransitionV0 {
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

impl StateTransitionOwned for IdentityCreditTransferToAddressesTransitionV0 {
    /// Get owner ID
    fn owner_id(&self) -> Identifier {
        self.identity_id
    }
}
