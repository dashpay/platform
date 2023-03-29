use crate::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::data_contract::DataContract;
use serde::{Deserialize, Serialize};

pub const DATA_CONTRACT_CREATE_TRANSITION_ACTION_VERSION: u32 = 0;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataContractCreateTransitionAction {
    pub version: u32,
    pub data_contract: DataContract,
}

impl From<DataContractCreateTransition> for DataContractCreateTransitionAction {
    fn from(value: DataContractCreateTransition) -> Self {
        DataContractCreateTransitionAction {
            version: DATA_CONTRACT_CREATE_TRANSITION_ACTION_VERSION,
            data_contract: value.data_contract,
        }
    }
}

impl From<&DataContractCreateTransition> for DataContractCreateTransitionAction {
    fn from(value: &DataContractCreateTransition) -> Self {
        DataContractCreateTransitionAction {
            version: DATA_CONTRACT_CREATE_TRANSITION_ACTION_VERSION,
            data_contract: value.data_contract.clone(),
        }
    }
}
