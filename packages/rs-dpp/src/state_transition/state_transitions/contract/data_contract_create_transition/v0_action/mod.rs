use crate::data_contract::DataContract;
use serde::{Deserialize, Serialize};
use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;

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
