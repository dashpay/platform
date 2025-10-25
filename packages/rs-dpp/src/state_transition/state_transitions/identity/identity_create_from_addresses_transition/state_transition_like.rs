use crate::prelude::UserFeeIncrease;
use crate::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use crate::state_transition::{
    StateTransitionLike, StateTransitionMultiSigned, StateTransitionType,
};
use crate::version::FeatureVersion;
use platform_value::{BinaryData, Identifier};

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

    fn owner_id(&self) -> Identifier {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => transition.owner_id(),
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

impl StateTransitionMultiSigned for IdentityCreateFromAddressesTransition {
    fn signatures(&self) -> &Vec<BinaryData> {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => transition.signatures(),
        }
    }

    fn set_signatures(&mut self, signatures: Vec<BinaryData>) {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => {
                transition.set_signatures(signatures)
            }
        }
    }
}
