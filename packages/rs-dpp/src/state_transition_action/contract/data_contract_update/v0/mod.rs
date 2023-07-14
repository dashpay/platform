mod transformer;

use crate::data_contract::DataContract;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataContractUpdateTransitionActionV0 {
    pub data_contract: DataContract,
}

