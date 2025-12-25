use platform_value::BinaryData;

use crate::address_funds::AddressWitness;
use crate::prelude::UserFeeIncrease;
use crate::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
use crate::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use crate::state_transition::{
    StateTransition, StateTransitionSingleSigned, StateTransitionWitnessSigned,
};
use crate::version::FeatureVersion;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

impl From<AddressFundingFromAssetLockTransitionV0> for StateTransition {
    fn from(value: AddressFundingFromAssetLockTransitionV0) -> Self {
        let transition: AddressFundingFromAssetLockTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for AddressFundingFromAssetLockTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        StateTransitionType::AddressFundingFromAssetLock
    }

    /// Returns IDs of the modified data - the output addresses
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![]
    }

    /// this is based on the asset lock proof
    fn unique_identifiers(&self) -> Vec<String> {
        vec![]
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}

impl StateTransitionSingleSigned for AddressFundingFromAssetLockTransitionV0 {
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        &self.signature
    }
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = BinaryData::new(signature)
    }
}

impl StateTransitionWitnessSigned for AddressFundingFromAssetLockTransitionV0 {
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
