use crate::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
use crate::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType::ShieldedTransfer;
use crate::version::FeatureVersion;

impl From<ShieldedTransferTransitionV0> for StateTransition {
    fn from(value: ShieldedTransferTransitionV0) -> Self {
        let transition: ShieldedTransferTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for ShieldedTransferTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        ShieldedTransfer
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

}
