use crate::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
use crate::data_contract::DataContract;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataContractCreateTransitionActionV0 {
    pub data_contract: DataContract,
}

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
