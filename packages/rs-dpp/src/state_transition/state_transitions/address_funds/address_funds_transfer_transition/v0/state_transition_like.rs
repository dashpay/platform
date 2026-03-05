use crate::address_funds::AddressWitness;
use crate::prelude::UserFeeIncrease;
use crate::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
use crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use crate::state_transition::StateTransitionHasUserFeeIncrease;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::StateTransitionType::AddressFundsTransfer;
use crate::state_transition::{StateTransition, StateTransitionWitnessSigned};
use crate::version::FeatureVersion;

impl From<AddressFundsTransferTransitionV0> for StateTransition {
    fn from(value: AddressFundsTransferTransitionV0) -> Self {
        let utxo_transfer_transition: AddressFundsTransferTransition = value.into();
        utxo_transfer_transition.into()
    }
}

impl StateTransitionLike for AddressFundsTransferTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        AddressFundsTransfer
    }

    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![]
    }

    /// State transitions with the same inputs should not be allowed to overlap
    fn unique_identifiers(&self) -> Vec<String> {
        self.inputs
            .iter()
            .map(|(key, (nonce, _))| key.base64_string_with_nonce(*nonce))
            .collect()
    }

}

impl StateTransitionHasUserFeeIncrease for AddressFundsTransferTransitionV0 {
    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}

impl StateTransitionWitnessSigned for AddressFundsTransferTransitionV0 {
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

    fn set_witnesses(&mut self, witnesses: Vec<AddressWitness>) {
        self.input_witnesses = witnesses;
    }
}
