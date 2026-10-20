use crate::prelude::UserFeeIncrease;
use crate::state_transition::shield_from_identity_transition::ShieldFromIdentityTransition;
use crate::state_transition::StateTransitionHasUserFeeIncrease;
use crate::state_transition::{
    StateTransitionLike, StateTransitionOwned, StateTransitionSingleSigned, StateTransitionType,
};
use crate::version::FeatureVersion;
use platform_value::{BinaryData, Identifier};

impl StateTransitionLike for ShieldFromIdentityTransition {
    fn modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            ShieldFromIdentityTransition::V0(transition) => transition.modified_data_ids(),
        }
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        match self {
            ShieldFromIdentityTransition::V0(_) => 0,
        }
    }

    fn state_transition_type(&self) -> StateTransitionType {
        match self {
            ShieldFromIdentityTransition::V0(transition) => transition.state_transition_type(),
        }
    }

    fn unique_identifiers(&self) -> Vec<String> {
        match self {
            ShieldFromIdentityTransition::V0(transition) => transition.unique_identifiers(),
        }
    }
}

impl StateTransitionHasUserFeeIncrease for ShieldFromIdentityTransition {
    fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            ShieldFromIdentityTransition::V0(transition) => transition.user_fee_increase(),
        }
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        match self {
            ShieldFromIdentityTransition::V0(transition) => {
                transition.set_user_fee_increase(user_fee_increase)
            }
        }
    }
}

impl StateTransitionSingleSigned for ShieldFromIdentityTransition {
    fn signature(&self) -> &BinaryData {
        match self {
            ShieldFromIdentityTransition::V0(transition) => transition.signature(),
        }
    }

    fn set_signature(&mut self, signature: BinaryData) {
        match self {
            ShieldFromIdentityTransition::V0(transition) => transition.set_signature(signature),
        }
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        match self {
            ShieldFromIdentityTransition::V0(transition) => {
                transition.set_signature_bytes(signature)
            }
        }
    }
}

impl StateTransitionOwned for ShieldFromIdentityTransition {
    fn owner_id(&self) -> Identifier {
        match self {
            ShieldFromIdentityTransition::V0(transition) => transition.owner_id(),
        }
    }
}
