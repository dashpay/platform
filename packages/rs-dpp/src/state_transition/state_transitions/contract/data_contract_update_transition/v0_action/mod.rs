use crate::data_contract::DataContract;
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataContractUpdateTransitionActionV0 {
    pub data_contract: DataContract,
}

impl From<DataContractUpdateTransitionV0> for DataContractUpdateTransitionActionV0 {
    fn from(value: DataContractUpdateTransitionV0) -> Self {
        DataContractUpdateTransitionActionV0 {
            data_contract: value.data_contract,
        }
    }
}

impl From<&DataContractUpdateTransitionV0> for DataContractUpdateTransitionActionV0 {
    fn from(value: &DataContractUpdateTransitionV0) -> Self {
        DataContractUpdateTransitionActionV0 {
            data_contract: value.data_contract.clone(),
        }
    }
}
