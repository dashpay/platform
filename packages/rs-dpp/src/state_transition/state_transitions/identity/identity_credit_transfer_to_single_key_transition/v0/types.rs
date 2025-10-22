use crate::state_transition::identity_credit_transfer_to_single_key_transition::fields::property_names::*;
use crate::state_transition::identity_credit_transfer_to_single_key_transition::fields::*;
use crate::state_transition::identity_credit_transfer_to_single_key_transition::v0::IdentityCreditTransferToSingleKeyTransitionV0;
use crate::state_transition::StateTransitionFieldTypes;

impl StateTransitionFieldTypes for IdentityCreditTransferToSingleKeyTransitionV0 {
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
