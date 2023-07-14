use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
use crate::state_transition_action::contract::data_contract_create::v0::DataContractCreateTransitionActionV0;
use serde::{Deserialize, Serialize};

impl From<DataContractCreateTransitionV0> for DataContractCreateTransitionActionV0 {
    fn from(value: DataContractCreateTransitionV0) -> Self {
        DataContractCreateTransitionActionV0 {
            data_contract: value.data_contract,
        }
    }
}

impl From<&DataContractCreateTransitionV0> for DataContractCreateTransitionActionV0 {
    fn from(value: &DataContractCreateTransitionV0) -> Self {
        DataContractCreateTransitionActionV0 {
            data_contract: value.data_contract.clone(),
        }
    }
}
