pub mod transformer;

use dpp::data_contract::DataContract;

#[derive(Debug, Clone)]
pub struct DataContractUpdateTransitionActionV0 {
    pub data_contract: DataContract,
}
