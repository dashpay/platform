use crate::state_transition::identity_top_up_from_shielded_pool_transition::v0::IdentityTopUpFromShieldedPoolTransitionV0;
use crate::state_transition::identity_top_up_from_shielded_pool_transition::IdentityTopUpFromShieldedPoolTransition;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;

impl From<IdentityTopUpFromShieldedPoolTransitionV0> for StateTransition {
    fn from(value: IdentityTopUpFromShieldedPoolTransitionV0) -> Self {
        let transition: IdentityTopUpFromShieldedPoolTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for IdentityTopUpFromShieldedPoolTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    fn state_transition_type(&self) -> StateTransitionType {
        StateTransitionType::IdentityTopUpFromShieldedPool
    }

    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.identity_id]
    }

    /// Unique by spent nullifier, exactly as `Unshield`: two transitions spending the
    /// same note can never both be valid.
    fn unique_identifiers(&self) -> Vec<String> {
        self.actions
            .iter()
            .map(|action| hex::encode(action.nullifier))
            .collect()
    }
}
