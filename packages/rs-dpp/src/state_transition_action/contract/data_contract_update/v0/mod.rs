mod transformer;

use crate::data_contract::DataContract;
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataContractUpdateTransitionActionV0 {
    pub data_contract: DataContract,
}

