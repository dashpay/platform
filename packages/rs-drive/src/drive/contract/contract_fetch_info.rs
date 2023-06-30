use costs::OperationCost;
use dpp::state_transition::fee::fee_result::FeeResult;
use crate::drive::flags::StorageFlags;

#[cfg(any(feature = "full", feature = "verify"))]
/// Contract and fetch information
#[derive(Default, PartialEq, Debug, Clone)]
pub struct ContractFetchInfo {
    /// The contract
    pub contract: Contract,
    /// The contract's potential storage flags
    pub storage_flags: Option<StorageFlags>,
    /// These are the operations that are used to fetch a contract
    /// This is only used on epoch change
    pub(crate) cost: OperationCost,
    /// The fee is updated every epoch based on operation costs
    /// Except if protocol version has changed in which case all the cache is cleared
    pub fee: Option<FeeResult>,
}
