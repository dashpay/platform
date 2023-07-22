use crate::drive::flags::StorageFlags;
use costs::OperationCost;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;
use dpp::tests::fixtures::get_dpns_data_contract_fixture;

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

#[cfg(test)]
impl DataContractFetchInfo {
    /// This should ONLY be used for tests
    pub fn dpns_contract_fixture(protocol_version : u32) -> Self {
        let dpns = get_dpns_data_contract_fixture(None, protocol_version);
        DataContractFetchInfo {
            contract: dpns.data_contract_owned(),
            storage_flags: None,
            cost: OperationCost::with_seek_count(1), //Just so there's a cost
            fee: Some(FeeResult::new_from_processing_fee(30000))
        }
    }
}
