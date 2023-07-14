mod transformer;

use crate::data_contract::DataContract;
use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataContractCreateTransitionActionV0 {
    pub data_contract: DataContract,
}

