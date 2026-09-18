use crate::prelude::UserFeeIncrease;
use crate::state_transition::identity_key_limits_update_transition::IdentityKeyLimitsUpdateTransition;
use crate::state_transition::StateTransitionHasUserFeeIncrease;
use crate::state_transition::{
    StateTransitionLike, StateTransitionOwned, StateTransitionSingleSigned, StateTransitionType,
};
use crate::version::FeatureVersion;
use platform_value::{BinaryData, Identifier};

impl StateTransitionLike for IdentityKeyLimitsUpdateTransition {
    /// Returns the ID of the identity whose key is updated
    fn modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => transition.modified_data_ids(),
        }
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(_) => 0,
        }
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => transition.state_transition_type(),
        }
    }

    fn unique_identifiers(&self) -> Vec<String> {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => transition.unique_identifiers(),
        }
    }
}

impl StateTransitionHasUserFeeIncrease for IdentityKeyLimitsUpdateTransition {
    fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => transition.user_fee_increase(),
        }
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => {
                transition.set_user_fee_increase(user_fee_increase)
            }
        }
    }
}

impl StateTransitionSingleSigned for IdentityKeyLimitsUpdateTransition {
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => transition.signature(),
        }
    }

    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => {
                transition.set_signature(signature)
            }
        }
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => {
                transition.set_signature_bytes(signature)
            }
        }
    }
}

impl StateTransitionOwned for IdentityKeyLimitsUpdateTransition {
    fn owner_id(&self) -> Identifier {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => transition.owner_id(),
        }
    }
}
