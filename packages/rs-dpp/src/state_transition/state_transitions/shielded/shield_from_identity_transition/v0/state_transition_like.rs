use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use platform_value::BinaryData;

use crate::prelude::UserFeeIncrease;
use crate::state_transition::StateTransitionHasUserFeeIncrease;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionOwned, StateTransitionType},
};

use crate::state_transition::shield_from_identity_transition::v0::ShieldFromIdentityTransitionV0;
use crate::state_transition::shield_from_identity_transition::ShieldFromIdentityTransition;

use crate::state_transition::{StateTransition, StateTransitionSingleSigned};
use crate::version::FeatureVersion;

impl From<ShieldFromIdentityTransitionV0> for StateTransition {
    fn from(value: ShieldFromIdentityTransitionV0) -> Self {
        let transition: ShieldFromIdentityTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for ShieldFromIdentityTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    fn state_transition_type(&self) -> StateTransitionType {
        StateTransitionType::ShieldFromIdentity
    }

    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.identity_id]
    }

    /// Unique by identity and nonce, like every identity-signed transition.
    fn unique_identifiers(&self) -> Vec<String> {
        vec![format!(
            "{}-{:x}",
            BASE64_STANDARD.encode(self.identity_id),
            self.nonce
        )]
    }
}

impl StateTransitionHasUserFeeIncrease for ShieldFromIdentityTransitionV0 {
    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}

impl StateTransitionSingleSigned for ShieldFromIdentityTransitionV0 {
    fn signature(&self) -> &BinaryData {
        &self.signature
    }

    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = BinaryData::new(signature)
    }
}

impl StateTransitionOwned for ShieldFromIdentityTransitionV0 {
    fn owner_id(&self) -> Identifier {
        self.identity_id
    }
}
