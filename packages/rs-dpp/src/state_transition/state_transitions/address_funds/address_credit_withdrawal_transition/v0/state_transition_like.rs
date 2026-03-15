use crate::address_funds::AddressWitness;
use crate::prelude::UserFeeIncrease;
use crate::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
use crate::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
use crate::state_transition::StateTransitionHasUserFeeIncrease;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::StateTransitionType::AddressCreditWithdrawal;
use crate::state_transition::{StateTransition, StateTransitionWitnessSigned};
use crate::version::FeatureVersion;

impl From<AddressCreditWithdrawalTransitionV0> for StateTransition {
    fn from(value: AddressCreditWithdrawalTransitionV0) -> Self {
        let address_credit_withdrawal_transition: AddressCreditWithdrawalTransition = value.into();
        address_credit_withdrawal_transition.into()
    }
}

impl StateTransitionLike for AddressCreditWithdrawalTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        AddressCreditWithdrawal
    }

    /// Returns IDs of the modified data - empty for withdrawals
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

impl StateTransitionHasUserFeeIncrease for AddressCreditWithdrawalTransitionV0 {
    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}

impl StateTransitionWitnessSigned for AddressCreditWithdrawalTransitionV0 {
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

    fn default_v0() -> AddressCreditWithdrawalTransitionV0 {
        AddressCreditWithdrawalTransitionV0::default()
    }

    #[test]
    fn state_transition_protocol_version_is_zero() {
        let t = default_v0();
        assert_eq!(t.state_transition_protocol_version(), 0);
    }

    #[test]
    fn state_transition_type_is_address_credit_withdrawal() {
        let t = default_v0();
        assert_eq!(t.state_transition_type(), AddressCreditWithdrawal);
    }

    #[test]
    fn modified_data_ids_is_empty() {
        let t = default_v0();
        assert!(t.modified_data_ids().is_empty());
    }

    #[test]
    fn unique_identifiers_maps_inputs() {
        let mut t = default_v0();
        let addr = PlatformAddress::P2pkh([1u8; 20]);
        t.inputs.insert(addr, (5, 100));
        let ids = t.unique_identifiers();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], addr.base64_string_with_nonce(5));
    }

    #[test]
    fn unique_identifiers_empty_when_no_inputs() {
        let t = default_v0();
        assert!(t.unique_identifiers().is_empty());
    }

    #[test]
    fn user_fee_increase_getter_setter() {
        let mut t = default_v0();
        assert_eq!(t.user_fee_increase(), 0);
        t.set_user_fee_increase(99);
        assert_eq!(t.user_fee_increase(), 99);
    }

    #[test]
    fn witness_signed_getters_setters() {
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
            StateTransitionType::AddressCreditWithdrawal
        );
    }
}
