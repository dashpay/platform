use crate::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use crate::data_contract::DataContract;
use serde::{Deserialize, Serialize};

pub const DATA_CONTRACT_UPDATE_TRANSITION_ACTION_VERSION: u32 = 0;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataContractUpdateTransitionAction {
    pub version: u32,
    pub data_contract: DataContract,
}

impl From<DataContractUpdateTransition> for DataContractUpdateTransitionAction {
    fn from(value: DataContractUpdateTransition) -> Self {
        DataContractUpdateTransitionAction {
            version: DATA_CONTRACT_UPDATE_TRANSITION_ACTION_VERSION,
            data_contract: value.data_contract,
        }
    }
}

impl From<&DataContractUpdateTransition> for DataContractUpdateTransitionAction {
    fn from(value: &DataContractUpdateTransition) -> Self {
        DataContractUpdateTransitionAction {
            version: DATA_CONTRACT_UPDATE_TRANSITION_ACTION_VERSION,
            data_contract: value.data_contract.clone(),
        }
    }
}
