pub mod transformer;

use dpp::data_contract::DataContract;

#[derive(Debug, Clone)]
pub struct DataContractCreateTransitionActionV0 {
    pub data_contract: DataContract,
}
