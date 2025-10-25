use crate::prelude::UserFeeIncrease;
use crate::state_transition::batch_transition::BatchTransition;
use crate::state_transition::{
    StateTransitionLike, StateTransitionSingleSigned, StateTransitionType,
};
use crate::version::FeatureVersion;
use platform_value::{BinaryData, Identifier};

impl StateTransitionLike for BatchTransition {
    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            BatchTransition::V0(transition) => transition.modified_data_ids(),
            BatchTransition::V1(transition) => transition.modified_data_ids(),
        }
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        match self {
            BatchTransition::V0(_) => 0,
            BatchTransition::V1(_) => 1,
        }
    }
    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        match self {
            BatchTransition::V0(transition) => transition.state_transition_type(),
            BatchTransition::V1(transition) => transition.state_transition_type(),
        }
    }

    /// returns the fee multiplier
    fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            BatchTransition::V0(transition) => transition.user_fee_increase(),
            BatchTransition::V1(transition) => transition.user_fee_increase(),
        }
    }
    /// set a fee multiplier
    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        match self {
            BatchTransition::V0(transition) => transition.set_user_fee_increase(user_fee_increase),
            BatchTransition::V1(transition) => transition.set_user_fee_increase(user_fee_increase),
        }
    }

    fn owner_id(&self) -> Identifier {
        match self {
            BatchTransition::V0(transition) => transition.owner_id(),
            BatchTransition::V1(transition) => transition.owner_id(),
        }
    }

    fn unique_identifiers(&self) -> Vec<String> {
        match self {
            BatchTransition::V0(transition) => transition.unique_identifiers(),
            BatchTransition::V1(transition) => transition.unique_identifiers(),
        }
    }
}

impl StateTransitionSingleSigned for BatchTransition {
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        match self {
            BatchTransition::V0(transition) => transition.signature(),
            BatchTransition::V1(transition) => transition.signature(),
        }
    }
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        match self {
            BatchTransition::V0(transition) => transition.set_signature(signature),
            BatchTransition::V1(transition) => transition.set_signature(signature),
        }
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        match self {
            BatchTransition::V0(transition) => transition.set_signature_bytes(signature),
            BatchTransition::V1(transition) => transition.set_signature_bytes(signature),
        }
    }
}
