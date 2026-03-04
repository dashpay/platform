use crate::prelude::UserFeeIncrease;
use crate::state_transition::unshield_transition::UnshieldTransition;
use crate::state_transition::{StateTransitionLike, StateTransitionType};
use crate::version::FeatureVersion;
use platform_value::Identifier;

impl StateTransitionLike for UnshieldTransition {
    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            UnshieldTransition::V0(transition) => transition.modified_data_ids(),
        }
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        match self {
            UnshieldTransition::V0(_) => 0,
        }
    }
    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        match self {
            UnshieldTransition::V0(transition) => transition.state_transition_type(),
        }
    }

    /// returns the fee multiplier
    fn user_fee_increase(&self) -> UserFeeIncrease {
        0
    }
    /// set a fee multiplier
    fn set_user_fee_increase(&mut self, _user_fee_increase: UserFeeIncrease) {
        // No-op: fee is cryptographically locked by the Orchard binding signature
    }

    fn unique_identifiers(&self) -> Vec<String> {
        match self {
            UnshieldTransition::V0(transition) => transition.unique_identifiers(),
        }
    }
}
