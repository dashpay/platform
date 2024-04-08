use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use grovedb::TransactionArg;
use std::sync::Arc;

impl Drive {
    /// Returns the contract fetch info with the given ID if it's in cache.
    #[inline(always)]
    pub(super) fn get_cached_contract_with_fetch_info_v0(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Option<Arc<DataContractFetchInfo>> {
        self.cache
            .data_contracts
            .get(contract_id, transaction.is_some())
            .map(|fetch_info| Arc::clone(&fetch_info))
    }
}
