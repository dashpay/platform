use crate::address_funds::AddressWitness;
use crate::prelude::UserFeeIncrease;
use crate::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use crate::state_transition::StateTransitionHasUserFeeIncrease;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionOwned, StateTransitionType},
};

use crate::state_transition::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;

use crate::state_transition::StateTransitionType::IdentityTopUpFromAddresses;
use crate::state_transition::{StateTransition, StateTransitionWitnessSigned};
use crate::version::FeatureVersion;

impl From<IdentityTopUpFromAddressesTransitionV0> for StateTransition {
    fn from(value: IdentityTopUpFromAddressesTransitionV0) -> Self {
        let transition: IdentityTopUpFromAddressesTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for IdentityTopUpFromAddressesTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        IdentityTopUpFromAddresses
    }

    /// Returns ID of the topUpd contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.identity_id]
    }

    /// State transitions with the same inputs should not be allowed to overlap
    fn unique_identifiers(&self) -> Vec<String> {
        self.inputs
            .iter()
            .map(|(key, (nonce, _))| key.base64_string_with_nonce(*nonce))
            .collect()
    }
}

impl StateTransitionHasUserFeeIncrease for IdentityTopUpFromAddressesTransitionV0 {
    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}

impl StateTransitionOwned for IdentityTopUpFromAddressesTransitionV0 {
    /// Get owner ID
    fn owner_id(&self) -> Identifier {
        self.identity_id
    }
}

impl StateTransitionWitnessSigned for IdentityTopUpFromAddressesTransitionV0 {
    fn inputs(
        &self,
    ) -> &std::collections::BTreeMap<
        crate::address_funds::PlatformAddress,
        (crate::prelude::AddressNonce, crate::fee::Credits),
    > {
        &self.inputs
    }

    fn inputs_mut(
        &mut self,
    ) -> &mut std::collections::BTreeMap<
        crate::address_funds::PlatformAddress,
        (crate::prelude::AddressNonce, crate::fee::Credits),
    > {
        &mut self.inputs
    }

    fn set_inputs(
        &mut self,
        inputs: std::collections::BTreeMap<
            crate::address_funds::PlatformAddress,
            (crate::prelude::AddressNonce, crate::fee::Credits),
        >,
    ) {
        self.inputs = inputs;
    }

    fn witnesses(&self) -> &Vec<AddressWitness> {
        &self.input_witnesses
    }

    fn set_witnesses(&mut self, input_witnesses: Vec<AddressWitness>) {
        self.input_witnesses = input_witnesses;
    }
}
