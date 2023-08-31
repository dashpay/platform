/// transformer
pub mod transformer;

use dpp::data_contract::DataContract;

/// data contract update transition action v0
#[derive(Debug, Clone)]
pub struct DataContractUpdateTransitionActionV0 {
    /// data contract
    pub data_contract: DataContract,
}
