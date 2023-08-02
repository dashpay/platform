pub mod transformer;

use dpp::data_contract::DataContract;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct DataContractUpdateTransitionActionV0 {
    pub data_contract: DataContract,
}
