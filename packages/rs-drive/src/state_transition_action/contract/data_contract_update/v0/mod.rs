/// transformer
pub mod transformer;

use dpp::data_contract::DataContract;
use dpp::prelude::IdentityContractNonce;

/// data contract update transition action v0
#[derive(Debug, Clone)]
pub struct DataContractUpdateTransitionActionV0 {
    /// data contract
    pub data_contract: DataContract,
    /// identity contract nonce
    pub identity_contract_nonce: IdentityContractNonce,
}
