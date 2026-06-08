use crate::state_transition::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
use crate::state_transition::{StateTransitionLike, StateTransitionType};
use crate::version::FeatureVersion;
use platform_value::Identifier;

impl StateTransitionLike for IdentityCreateFromShieldedPoolTransition {
    /// Returns the id of the newly created identity (the only modified data).
    fn modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(transition) => {
                transition.modified_data_ids()
            }
        }
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(_) => 0,
        }
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(transition) => {
                transition.state_transition_type()
            }
        }
    }

    fn unique_identifiers(&self) -> Vec<String> {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(transition) => {
                transition.unique_identifiers()
            }
        }
    }
}
