use crate::prelude::UserFeeIncrease;
use crate::state_transition::unshield_transition::v0::UnshieldTransitionV0;
use crate::state_transition::unshield_transition::UnshieldTransition;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType::Unshield;
use crate::version::FeatureVersion;

impl From<UnshieldTransitionV0> for StateTransition {
    fn from(value: UnshieldTransitionV0) -> Self {
        let transition: UnshieldTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for UnshieldTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        Unshield
    }

    /// Returns IDs of modified data (none for shielded transitions)
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![]
    }

    /// For ZK-only transitions, uniqueness comes from nullifiers in the actions.
    /// Each nullifier can only be used once, making them natural unique identifiers.
    fn unique_identifiers(&self) -> Vec<String> {
        self.actions
            .iter()
            .map(|action| hex::encode(action.nullifier))
            .collect()
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}
