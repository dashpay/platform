/// transformer
pub mod transformer;

use dpp::data_contract::DataContract;
use dpp::prelude::{FeeMultiplier, IdentityNonce};

/// data contract create transition action v0
#[derive(Debug, Clone)]
pub struct DataContractCreateTransitionActionV0 {
    /// data contract
    pub data_contract: DataContract,
    /// identity nonce
    pub identity_nonce: IdentityNonce,
    /// fee multiplier
    pub fee_multiplier: FeeMultiplier,
}
