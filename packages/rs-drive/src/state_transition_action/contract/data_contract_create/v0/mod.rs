/// transformer
pub mod transformer;

use dpp::data_contract::DataContract;

/// data contract create transition action v0
#[derive(Debug, Clone)]
pub struct DataContractCreateTransitionActionV0 {
    /// data contract
    pub data_contract: DataContract,
}
