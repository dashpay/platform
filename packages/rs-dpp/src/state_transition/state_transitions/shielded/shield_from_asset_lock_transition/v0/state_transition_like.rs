use platform_value::BinaryData;

use crate::prelude::UserFeeIncrease;
use crate::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
use crate::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use crate::state_transition::{
    StateTransition, StateTransitionSingleSigned,
};
use crate::version::FeatureVersion;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

impl From<ShieldFromAssetLockTransitionV0> for StateTransition {
    fn from(value: ShieldFromAssetLockTransitionV0) -> Self {
        let transition: ShieldFromAssetLockTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for ShieldFromAssetLockTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        StateTransitionType::ShieldFromAssetLock
    }

    /// Returns IDs of the modified data
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![]
    }

    /// Returns unique identifiers based on the cmx values from actions
    fn unique_identifiers(&self) -> Vec<String> {
        self.actions
            .iter()
            .map(|a| hex::encode(a.cmx))
            .collect()
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}

impl StateTransitionSingleSigned for ShieldFromAssetLockTransitionV0 {
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
