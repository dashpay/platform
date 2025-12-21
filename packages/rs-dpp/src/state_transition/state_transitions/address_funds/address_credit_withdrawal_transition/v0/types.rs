use crate::state_transition::address_credit_withdrawal_transition::fields::*;
use crate::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
use crate::state_transition::StateTransitionFieldTypes;

impl StateTransitionFieldTypes for AddressCreditWithdrawalTransitionV0 {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![OUTPUT_SCRIPT]
    }
}
