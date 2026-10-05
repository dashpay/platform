use crate::address_funds::{AddressWitness, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::{AddressNonce, UserFeeIncrease};
use crate::state_transition::shield_transition::v1::ShieldTransitionV1;
use crate::state_transition::shield_transition::ShieldTransition;
use crate::state_transition::StateTransitionHasUserFeeIncrease;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::StateTransitionType::Shield;
use crate::state_transition::{StateTransition, StateTransitionWitnessSigned};
use crate::version::FeatureVersion;
use std::collections::BTreeMap;

impl From<ShieldTransitionV1> for StateTransition {
    fn from(value: ShieldTransitionV1) -> Self {
        let shield_transition: ShieldTransition = value.into();
        shield_transition.into()
    }
}

impl StateTransitionLike for ShieldTransitionV1 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        1
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        Shield
    }

    /// Returns IDs of modified data (none for shielded transitions)
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

impl StateTransitionHasUserFeeIncrease for ShieldTransitionV1 {
    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}

impl StateTransitionWitnessSigned for ShieldTransitionV1 {
    fn inputs(&self) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        &self.inputs
    }

    fn inputs_mut(&mut self) -> &mut BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        &mut self.inputs
    }

    fn set_inputs(&mut self, inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>) {
        self.inputs = inputs;
    }

    fn witnesses(&self) -> &Vec<AddressWitness> {
        &self.input_witnesses
    }

    fn set_witnesses(&mut self, witnesses: Vec<AddressWitness>) {
        self.input_witnesses = witnesses;
    }
}
