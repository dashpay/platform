use crate::address_funds::AddressWitness;
use crate::prelude::UserFeeIncrease;
use crate::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use crate::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use crate::state_transition::{StateTransition, StateTransitionWitnessSigned};
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::StateTransitionType::IdentityCreateFromAddresses;
use crate::version::FeatureVersion;

impl From<IdentityCreateFromAddressesTransitionV0> for StateTransition {
    fn from(value: IdentityCreateFromAddressesTransitionV0) -> Self {
        let transition: IdentityCreateFromAddressesTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for IdentityCreateFromAddressesTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        IdentityCreateFromAddresses
    }
    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![]
    }

    /// each input must be unique in the mempool
    fn unique_identifiers(&self) -> Vec<String> {
        self.inputs
            .iter()
            .map(|(key, (nonce, _))| key.base64_string_with_nonce(*nonce))
            .collect()
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}

impl StateTransitionWitnessSigned for IdentityCreateFromAddressesTransitionV0 {
    fn witnesses(&self) -> &Vec<AddressWitness> {
        &self.input_witnesses
    }

    fn set_witnesses(&mut self, input_witnesses: Vec<AddressWitness>) {
        self.input_witnesses = input_witnesses;
    }
}
