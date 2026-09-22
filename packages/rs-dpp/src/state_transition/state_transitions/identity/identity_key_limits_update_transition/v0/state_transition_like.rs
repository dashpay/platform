use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use platform_value::BinaryData;

use crate::prelude::UserFeeIncrease;
use crate::state_transition::StateTransitionHasUserFeeIncrease;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionOwned, StateTransitionType},
};

use crate::state_transition::identity_key_limits_update_transition::v0::IdentityKeyLimitsUpdateTransitionV0;
use crate::state_transition::identity_key_limits_update_transition::IdentityKeyLimitsUpdateTransition;

use crate::state_transition::StateTransitionType::IdentityKeyLimitsUpdate;
use crate::state_transition::{StateTransition, StateTransitionSingleSigned};
use crate::version::FeatureVersion;

impl From<IdentityKeyLimitsUpdateTransitionV0> for StateTransition {
    fn from(value: IdentityKeyLimitsUpdateTransitionV0) -> Self {
        let transition: IdentityKeyLimitsUpdateTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for IdentityKeyLimitsUpdateTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        IdentityKeyLimitsUpdate
    }

    /// Returns the ID of the identity whose key is updated
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

impl StateTransitionHasUserFeeIncrease for IdentityKeyLimitsUpdateTransitionV0 {
    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}

impl StateTransitionSingleSigned for IdentityKeyLimitsUpdateTransitionV0 {
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

impl StateTransitionOwned for IdentityKeyLimitsUpdateTransitionV0 {
    /// Get owner ID
    fn owner_id(&self) -> Identifier {
        self.identity_id
    }
}
