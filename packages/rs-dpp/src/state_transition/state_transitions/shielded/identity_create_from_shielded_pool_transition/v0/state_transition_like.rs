use crate::state_transition::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
use crate::state_transition::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType::IdentityCreateFromShieldedPool;
use crate::version::FeatureVersion;

impl From<IdentityCreateFromShieldedPoolTransitionV0> for StateTransition {
    fn from(value: IdentityCreateFromShieldedPoolTransitionV0) -> Self {
        let transition: IdentityCreateFromShieldedPoolTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for IdentityCreateFromShieldedPoolTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        IdentityCreateFromShieldedPool
    }

    /// Returns the id of the newly created identity (the only modified data).
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.identity_id]
    }

    /// For ZK pool-spend transitions, uniqueness comes from the nullifiers in the actions.
    /// Each nullifier can only be used once, making them natural unique identifiers.
    fn unique_identifiers(&self) -> Vec<String> {
        self.actions
            .iter()
            .map(|action| hex::encode(action.nullifier))
            .collect()
    }
}
