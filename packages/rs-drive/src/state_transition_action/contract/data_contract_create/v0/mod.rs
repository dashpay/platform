/// transformer
pub mod transformer;

use dpp::data_contract::DataContract;
use dpp::prelude::IdentityNonce;

/// data contract create transition action v0
#[derive(Debug, Clone)]
pub struct DataContractCreateTransitionActionV0 {
    /// data contract
    pub data_contract: DataContract,
    /// identity nonce
    pub identity_nonce: IdentityNonce,
}
