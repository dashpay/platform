use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV1;
use crate::state_transition::state_transitions::common_fields::property_names::{
    SIGNATURE, SIGNATURE_PUBLIC_KEY_ID,
};
use crate::state_transition::StateTransitionFieldTypes;

impl StateTransitionFieldTypes for DataContractCreateTransitionV1 {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }
}
