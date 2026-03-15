use platform_value::BinaryData;

use crate::address_funds::AddressWitness;
use crate::prelude::UserFeeIncrease;
use crate::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
use crate::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use crate::state_transition::StateTransitionHasUserFeeIncrease;
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
}

impl StateTransitionHasUserFeeIncrease for AddressFundingFromAssetLockTransitionV0 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_funds::PlatformAddress;
    use std::collections::BTreeMap;

    fn default_v0() -> AddressFundingFromAssetLockTransitionV0 {
        AddressFundingFromAssetLockTransitionV0::default()
    }

    #[test]
    fn state_transition_protocol_version_is_zero() {
        let t = default_v0();
        assert_eq!(t.state_transition_protocol_version(), 0);
    }

    #[test]
    fn state_transition_type_is_address_funding_from_asset_lock() {
        let t = default_v0();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::AddressFundingFromAssetLock
        );
    }

    #[test]
    fn modified_data_ids_is_empty() {
        let t = default_v0();
        assert!(t.modified_data_ids().is_empty());
    }

    #[test]
    fn unique_identifiers_is_empty() {
        let t = default_v0();
        assert!(t.unique_identifiers().is_empty());
    }

    #[test]
    fn user_fee_increase_getter_setter() {
        let mut t = default_v0();
        assert_eq!(t.user_fee_increase(), 0);
        t.set_user_fee_increase(42);
        assert_eq!(t.user_fee_increase(), 42);
    }

    #[test]
    fn signature_getter_setter() {
        let mut t = default_v0();
        assert!(t.signature().is_empty());
        let sig = BinaryData::new(vec![1, 2, 3]);
        t.set_signature(sig.clone());
        assert_eq!(t.signature(), &sig);
    }

    #[test]
    fn set_signature_bytes() {
        let mut t = default_v0();
        t.set_signature_bytes(vec![4, 5, 6]);
        assert_eq!(t.signature().as_slice(), &[4, 5, 6]);
    }

    #[test]
    fn witness_signed_inputs_and_witnesses() {
        let mut t = default_v0();
        assert!(t.inputs().is_empty());
        assert!(t.witnesses().is_empty());

        let mut new_inputs = BTreeMap::new();
        new_inputs.insert(PlatformAddress::P2pkh([1u8; 20]), (0u32, 100u64));
        t.set_inputs(new_inputs);
        assert_eq!(t.inputs().len(), 1);

        let witnesses = vec![AddressWitness::P2pkh {
            signature: vec![0u8; 65].into(),
        }];
        t.set_witnesses(witnesses);
        assert_eq!(t.witnesses().len(), 1);

        t.inputs_mut().clear();
        assert!(t.inputs().is_empty());
    }

    #[test]
    fn from_v0_into_state_transition() {
        let t = default_v0();
        let st: StateTransition = t.into();
        assert_eq!(
            st.state_transition_type(),
            StateTransitionType::AddressFundingFromAssetLock
        );
    }
}
