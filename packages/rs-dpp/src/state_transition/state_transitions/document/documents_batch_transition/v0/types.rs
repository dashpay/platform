use crate::state_transition::documents_batch_transition::fields::property_names::*;
use crate::state_transition::documents_batch_transition::fields::*;
use crate::state_transition::documents_batch_transition::DocumentsBatchTransitionV0;
use crate::state_transition::StateTransitionFieldTypes;

impl StateTransitionFieldTypes for DocumentsBatchTransitionV0 {
    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![OWNER_ID]
    }

    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }
}
