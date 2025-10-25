use crate::state_transition::identity_credit_transfer_to_address_transition::fields::*;
use crate::state_transition::identity_credit_transfer_to_address_transition::v0::IdentityCreditTransferToAddressTransitionV0;
use crate::state_transition::StateTransitionFieldTypes;

impl StateTransitionFieldTypes for IdentityCreditTransferToAddressTransitionV0 {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }
}
