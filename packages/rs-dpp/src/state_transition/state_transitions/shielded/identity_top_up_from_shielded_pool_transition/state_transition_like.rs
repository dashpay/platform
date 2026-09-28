use crate::state_transition::identity_top_up_from_shielded_pool_transition::IdentityTopUpFromShieldedPoolTransition;
use crate::state_transition::{StateTransitionLike, StateTransitionType};
use crate::version::FeatureVersion;
use platform_value::Identifier;

impl StateTransitionLike for IdentityTopUpFromShieldedPoolTransition {
    fn modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            IdentityTopUpFromShieldedPoolTransition::V0(transition) => {
                transition.modified_data_ids()
            }
        }
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        match self {
            IdentityTopUpFromShieldedPoolTransition::V0(_) => 0,
        }
    }

    fn state_transition_type(&self) -> StateTransitionType {
        match self {
            IdentityTopUpFromShieldedPoolTransition::V0(transition) => {
                transition.state_transition_type()
            }
        }
    }

    fn unique_identifiers(&self) -> Vec<String> {
        match self {
            IdentityTopUpFromShieldedPoolTransition::V0(transition) => {
                transition.unique_identifiers()
            }
        }
    }
}
