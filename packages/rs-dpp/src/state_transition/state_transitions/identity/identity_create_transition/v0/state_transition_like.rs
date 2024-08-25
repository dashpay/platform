use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use platform_value::BinaryData;

use crate::prelude::UserFeeIncrease;
use crate::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use crate::state_transition::StateTransition;

use crate::state_transition::StateTransitionType::IdentityCreate;
use crate::version::FeatureVersion;

impl From<IdentityCreateTransitionV0> for StateTransition {
    fn from(value: IdentityCreateTransitionV0) -> Self {
        let transition: IdentityCreateTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for IdentityCreateTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        IdentityCreate
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

    /// this is based on the asset lock
    fn unique_identifiers(&self) -> Vec<String> {
        vec![BASE64_STANDARD.encode(self.identity_id)]
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}
