use crate::prelude::UserFeeIncrease;
use crate::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use crate::state_transition::StateTransitionHasUserFeeIncrease;
use crate::state_transition::{
    StateTransitionLike, StateTransitionSingleSigned, StateTransitionType,
};
use crate::version::FeatureVersion;
use platform_value::{BinaryData, Identifier};

impl StateTransitionLike for ShieldFromAssetLockTransition {
    /// Returns IDs of the modified data
    fn modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            ShieldFromAssetLockTransition::V0(transition) => transition.modified_data_ids(),
        }
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        match self {
            ShieldFromAssetLockTransition::V0(_) => 0,
        }
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        match self {
            ShieldFromAssetLockTransition::V0(transition) => transition.state_transition_type(),
        }
    }

    fn unique_identifiers(&self) -> Vec<String> {
        match self {
            ShieldFromAssetLockTransition::V0(transition) => transition.unique_identifiers(),
        }
    }
}

impl StateTransitionHasUserFeeIncrease for ShieldFromAssetLockTransition {
    fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            ShieldFromAssetLockTransition::V0(transition) => transition.user_fee_increase(),
        }
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        match self {
            ShieldFromAssetLockTransition::V0(transition) => {
                transition.set_user_fee_increase(user_fee_increase)
            }
        }
    }
}

impl StateTransitionSingleSigned for ShieldFromAssetLockTransition {
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        match self {
            ShieldFromAssetLockTransition::V0(transition) => transition.signature(),
        }
    }

    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        match self {
            ShieldFromAssetLockTransition::V0(transition) => transition.set_signature(signature),
        }
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        match self {
            ShieldFromAssetLockTransition::V0(transition) => {
                transition.set_signature_bytes(signature)
            }
        }
    }
}
