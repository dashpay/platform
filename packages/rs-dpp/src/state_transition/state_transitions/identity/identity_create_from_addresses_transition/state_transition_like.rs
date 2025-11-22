use crate::address_funds::AddressWitness;
use crate::prelude::UserFeeIncrease;
use crate::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use crate::state_transition::{
    StateTransitionLike, StateTransitionOwned, StateTransitionType, StateTransitionWitnessSigned,
};
use crate::version::FeatureVersion;
use platform_value::Identifier;

impl StateTransitionLike for IdentityCreateFromAddressesTransition {
    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => transition.modified_data_ids(),
        }
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        match self {
            IdentityCreateFromAddressesTransition::V0(_) => 0,
        }
    }
    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => {
                transition.state_transition_type()
            }
        }
    }

    /// returns the fee multiplier
    fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => transition.user_fee_increase(),
        }
    }
    /// set a fee multiplier
    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => {
                transition.set_user_fee_increase(user_fee_increase)
            }
        }
    }

    fn unique_identifiers(&self) -> Vec<String> {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => {
                transition.unique_identifiers()
            }
        }
    }
}

impl StateTransitionWitnessSigned for IdentityCreateFromAddressesTransition {
    fn witnesses(&self) -> &Vec<AddressWitness> {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => transition.witnesses(),
        }
    }

    fn set_witnesses(&mut self, witnesses: Vec<AddressWitness>) {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => {
                transition.set_witnesses(witnesses)
            }
        }
    }
}

impl StateTransitionOwned for IdentityCreateFromAddressesTransition {
    fn owner_id(&self) -> Identifier {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => transition.owner_id(),
        }
    }
}
