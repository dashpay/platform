use crate::state_transition::token_unshield_with_shielded_fee_transition::TokenUnshieldWithShieldedFeeTransition;
use crate::state_transition::{StateTransitionLike, StateTransitionType};
use crate::version::FeatureVersion;
use platform_value::Identifier;

impl StateTransitionLike for TokenUnshieldWithShieldedFeeTransition {
    fn modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(transition) => {
                transition.modified_data_ids()
            }
        }
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(_) => 0,
        }
    }

    fn state_transition_type(&self) -> StateTransitionType {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(transition) => {
                transition.state_transition_type()
            }
        }
    }

    fn unique_identifiers(&self) -> Vec<String> {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(transition) => {
                transition.unique_identifiers()
            }
        }
    }
}
