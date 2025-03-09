use crate::state_transition::state_transitions::identity::identity_credit_withdrawal_transition::fields::*;
use crate::state_transition::state_transitions::identity::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
use crate::state_transition::StateTransitionFieldTypes;

impl StateTransitionFieldTypes for IdentityCreditWithdrawalTransitionV1 {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, OUTPUT_SCRIPT]
    }
}
