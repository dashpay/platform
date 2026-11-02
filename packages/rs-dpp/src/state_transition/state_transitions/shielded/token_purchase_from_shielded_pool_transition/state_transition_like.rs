use crate::state_transition::token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransition;
use crate::state_transition::{StateTransitionLike, StateTransitionType};
use crate::version::FeatureVersion;
use platform_value::Identifier;

impl StateTransitionLike for TokenPurchaseFromShieldedPoolTransition {
    fn modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(transition) => {
                transition.modified_data_ids()
            }
        }
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(_) => 0,
        }
    }

    fn state_transition_type(&self) -> StateTransitionType {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(transition) => {
                transition.state_transition_type()
            }
        }
    }

    fn unique_identifiers(&self) -> Vec<String> {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(transition) => {
                transition.unique_identifiers()
            }
        }
    }
}
