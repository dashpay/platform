use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
use crate::state_transition::data_contract_update_transition::property_names::{SIGNATURE, SIGNATURE_PUBLIC_KEY_ID};
use crate::state_transition::StateTransitionConvert;

impl StateTransitionConvert for DataContractUpdateTransitionV0 {
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
