use crate::address_funds::AddressWitness;
use crate::prelude::UserFeeIncrease;
use crate::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
use crate::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
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

    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}

impl StateTransitionWitnessSigned for AddressCreditWithdrawalTransitionV0 {
    fn witnesses(&self) -> &Vec<AddressWitness> {
        &self.input_witnesses
    }

    fn set_witnesses(&mut self, witnesses: Vec<AddressWitness>) {
        self.input_witnesses = witnesses;
    }
}
