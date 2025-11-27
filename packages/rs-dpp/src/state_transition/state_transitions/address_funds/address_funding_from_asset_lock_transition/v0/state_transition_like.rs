use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use platform_value::BinaryData;

use crate::prelude::UserFeeIncrease;
use crate::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionOwned, StateTransitionType},
};

use crate::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
use crate::state_transition::{StateTransition, StateTransitionSingleSigned};

use crate::state_transition::StateTransitionType::IdentityCreate;
use crate::version::FeatureVersion;

impl From<AddressFundingFromAssetLockTransitionV0> for StateTransition {
    fn from(value: AddressFundingFromAssetLockTransitionV0) -> Self {
        let transition: AddressFundingFromAssetLockTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for AddressFundingFromAssetLockTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        IdentityCreate
    }

    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.identity_id]
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

impl StateTransitionSingleSigned for AddressFundingFromAssetLockTransitionV0 {
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

impl StateTransitionOwned for AddressFundingFromAssetLockTransitionV0 {
    /// Get owner ID
    fn owner_id(&self) -> Identifier {
        self.identity_id
    }
}
