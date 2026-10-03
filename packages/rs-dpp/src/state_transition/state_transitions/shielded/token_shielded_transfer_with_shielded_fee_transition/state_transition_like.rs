use crate::state_transition::token_shielded_transfer_with_shielded_fee_transition::TokenShieldedTransferWithShieldedFeeTransition;
use crate::state_transition::{StateTransitionLike, StateTransitionType};
use crate::version::FeatureVersion;
use platform_value::Identifier;

impl StateTransitionLike for TokenShieldedTransferWithShieldedFeeTransition {
    fn modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            TokenShieldedTransferWithShieldedFeeTransition::V0(transition) => {
                transition.modified_data_ids()
            }
        }
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        match self {
            TokenShieldedTransferWithShieldedFeeTransition::V0(_) => 0,
        }
    }

    fn state_transition_type(&self) -> StateTransitionType {
        match self {
            TokenShieldedTransferWithShieldedFeeTransition::V0(transition) => {
                transition.state_transition_type()
            }
        }
    }

    fn unique_identifiers(&self) -> Vec<String> {
        match self {
            TokenShieldedTransferWithShieldedFeeTransition::V0(transition) => {
                transition.unique_identifiers()
            }
        }
    }
}
