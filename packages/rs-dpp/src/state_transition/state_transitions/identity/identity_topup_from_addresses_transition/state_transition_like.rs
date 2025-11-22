use crate::prelude::UserFeeIncrease;
use crate::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use crate::state_transition::{StateTransitionLike, StateTransitionOwned, StateTransitionType};
use crate::version::FeatureVersion;
use platform_value::Identifier;

impl StateTransitionLike for IdentityTopUpFromAddressesTransition {
    /// Returns ID of the topupd contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            IdentityTopUpFromAddressesTransition::V0(transition) => transition.modified_data_ids(),
        }
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        match self {
            IdentityTopUpFromAddressesTransition::V0(_) => 0,
        }
    }
    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        match self {
            IdentityTopUpFromAddressesTransition::V0(transition) => {
                transition.state_transition_type()
            }
        }
    }

    /// returns the fee multiplier
    fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            IdentityTopUpFromAddressesTransition::V0(transition) => transition.user_fee_increase(),
        }
    }
    /// set a fee multiplier
    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        match self {
            IdentityTopUpFromAddressesTransition::V0(transition) => {
                transition.set_user_fee_increase(user_fee_increase)
            }
        }
    }

    fn unique_identifiers(&self) -> Vec<String> {
        match self {
            IdentityTopUpFromAddressesTransition::V0(transition) => transition.unique_identifiers(),
        }
    }
}

impl StateTransitionOwned for IdentityTopUpFromAddressesTransition {
    fn owner_id(&self) -> Identifier {
        match self {
            IdentityTopUpFromAddressesTransition::V0(transition) => transition.owner_id(),
        }
    }
}
