use crate::drive::flags::StorageFlags;
use costs::OperationCost;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;

#[cfg(any(feature = "full", feature = "verify"))]
///DataContract and fetch information
#[derive(PartialEq, Debug, Clone)]
pub struct DataContractFetchInfo {
    /// The contract
    pub contract: DataContract,
    /// The contract's potential storage flags
    pub storage_flags: Option<StorageFlags>,
    /// These are the operations that are used to fetch a contract
    /// This is only used on epoch change
    pub(crate) cost: OperationCost,
    /// The fee is updated every epoch based on operation costs
    /// Except if protocol version has changed in which case all the cache is cleared
    pub fee: Option<FeeResult>,
}
