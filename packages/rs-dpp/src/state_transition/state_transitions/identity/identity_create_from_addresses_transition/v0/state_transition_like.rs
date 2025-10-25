use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use platform_value::BinaryData;

use crate::prelude::UserFeeIncrease;
use crate::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use crate::state_transition::{StateTransition, StateTransitionMultiSigned};

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
        vec![self.identity_id]
    }

    /// Get owner ID
    fn owner_id(&self) -> Identifier {
        self.identity_id
    }

    /// this is based on the asset lock
    fn unique_identifiers(&self) -> Vec<String> {
        vec![BASE64_STANDARD.encode(self.identity_id)]
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}

impl StateTransitionMultiSigned for IdentityCreateFromAddressesTransitionV0 {
    fn signatures(&self) -> &Vec<BinaryData> {
        &self.input_signatures
    }

    fn set_signatures(&mut self, signatures: Vec<BinaryData>) {
        self.input_signatures = signatures;
    }
}
