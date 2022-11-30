use crate::drive::contract::ContractFetchInfo;
use crate::drive::TransactionPointerAddress;
use grovedb::{Transaction, TransactionArg};
use moka::sync::Cache;
use std::collections::HashMap;
use std::sync::Arc;

/// Drive cache struct
pub struct DriveCache {
    /// Cached contracts
    pub cached_contracts: DataContractCache,
    /// Genesis time in ms
    pub genesis_time_ms: Option<u64>,
}

/// Data Contract cache that handle both non transactional and transactional data
pub struct DataContractCache {
    global_cache: Cache<[u8; 32], Arc<ContractFetchInfo>>,
    transactional_cache: DataContractTransactionalCache,
}

impl DataContractCache {
    /// Create a new Data Contract cache instance
    pub fn new(global_cache_max_capacity: u64, transactional_cache_max_capacity: u64) -> Self {
        Self {
            global_cache: Cache::new(global_cache_max_capacity),
            transactional_cache: DataContractTransactionalCache::new(
                transactional_cache_max_capacity,
            ),
        }
    }

    /// Inserts Data Contract to transactional cache if present
    /// otherwise to goes to global cache
    pub fn insert(&mut self, fetch_info: Arc<ContractFetchInfo>, transaction: TransactionArg) {
        if let Some(tx) = transaction {
            self.transactional_cache.insert(tx, fetch_info);
        } else {
            self.global_cache
                .insert(fetch_info.contract.id().to_buffer(), fetch_info);
        }
    }

    /// Tries to get a data contract from transaction cache if present
    /// if transactional cache doesn't have the contract or transaction is not present
    /// then it tries get the contract from global cache
    pub fn get(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Option<Arc<ContractFetchInfo>> {
        transaction
            .and_then(|tx| self.transactional_cache.get(tx, contract_id))
            .or_else(|| self.global_cache.get(&contract_id))
    }

    /// Merge transactional cache to global cache if present
    pub fn merge_transactional_cache(&self, transaction: &Transaction) {
        if let Some(cache) = self.transactional_cache.get_cache(transaction) {
            for (contract_id, fetch_info) in cache {
                self.global_cache.insert(*contract_id, fetch_info);
            }
        }
    }

    /// Clear cache for specific transaction
    pub fn clear_transactional_cache(&mut self, transaction: &Transaction) {
        self.transactional_cache.clear(transaction);
    }

    /// Clear all transactional cache
    pub fn clear_all_transactional_cache(&mut self) {
        self.transactional_cache.clear_all();
    }
}

/// Transactional Cache contains data contracts cache per transaction
/// and provide convenient methods to insert and get data contracts from the cache
pub struct DataContractTransactionalCache {
    cache_map: HashMap<TransactionPointerAddress, Cache<[u8; 32], Arc<ContractFetchInfo>>>,
    max_capacity: u64,
}

impl DataContractTransactionalCache {
    /// Creates new transactional cache
    pub fn new(max_capacity: u64) -> Self {
        Self {
            cache_map: HashMap::new(),
            max_capacity,
        }
    }

    /// Insert a data contract with fetch info to cache
    pub fn insert(&mut self, transaction: &Transaction, fetch_info: Arc<ContractFetchInfo>) {
        let transaction_pointer_address = self.retrieve_transaction_pointer_address(transaction);

        let cache = self
            .cache_map
            .entry(transaction_pointer_address)
            .or_insert_with(|| Cache::new(self.max_capacity));

        cache.insert(fetch_info.contract.id.to_buffer(), fetch_info);
    }

    /// Returns a data contract from cache if present
    pub fn get(
        &self,
        transaction: &Transaction,
        data_contract_id: [u8; 32],
    ) -> Option<Arc<ContractFetchInfo>> {
        self.get_cache(transaction)
            .and_then(|cache| cache.get(&data_contract_id))
    }

    /// Clear cache for specific transaction
    fn clear(&mut self, transaction: &Transaction) {
        let transaction_pointer_address = self.retrieve_transaction_pointer_address(transaction);

        self.cache_map.remove(&transaction_pointer_address);
    }

    /// Clear all transactional cache
    fn clear_all(&mut self) {
        self.cache_map.clear();
    }

    /// Returns cache for transaction or error if not present
    fn get_cache(
        &self,
        transaction: &Transaction,
    ) -> Option<&Cache<[u8; 32], Arc<ContractFetchInfo>>> {
        let transaction_pointer_address = self.retrieve_transaction_pointer_address(transaction);

        self.cache_map.get(&transaction_pointer_address)
    }

    /// Get transaction pointer address from transaction reference
    fn retrieve_transaction_pointer_address(
        &self,
        transaction: &Transaction,
    ) -> TransactionPointerAddress {
        let transaction_raw_pointer = transaction as *const Transaction;

        transaction_raw_pointer as TransactionPointerAddress
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::common::helpers::setup::setup_drive;

    mod get {
        use super::*;

        #[test]
        fn test_get_from_global_cache_when_transaction_is_not_specified() {
            let drive = setup_drive(None);
            let transaction = drive.grove.start_transaction();

            let mut data_contract_cache = DataContractCache::new(10, 10);

            // Create global contract
            let fetch_info_global = Arc::new(ContractFetchInfo::default());

            let contract_id = fetch_info_global.contract.id().to_buffer();

            data_contract_cache
                .global_cache
                .insert(contract_id, Arc::clone(&fetch_info_global));

            // Create transactional contract with a new version
            let mut fetch_info_transactional = ContractFetchInfo::default();

            fetch_info_transactional.contract.increment_version();

            let fetch_info_transactional = Arc::new(fetch_info_transactional);

            data_contract_cache
                .transactional_cache
                .insert(&transaction, Arc::clone(&fetch_info_transactional));

            let fetch_info_from_cache = data_contract_cache
                .get(contract_id, None)
                .expect("should be present");

            assert_eq!(fetch_info_from_cache, fetch_info_global)
        }

        #[test]
        fn test_get_from_global_cache_when_transactional_cache_does_not_have_contract() {
            let drive = setup_drive(None);
            let transaction = drive.grove.start_transaction();

            let data_contract_cache = DataContractCache::new(10, 10);

            let fetch_info_global = Arc::new(ContractFetchInfo::default());

            let contract_id = fetch_info_global.contract.id().to_buffer();

            data_contract_cache
                .global_cache
                .insert(contract_id, Arc::clone(&fetch_info_global));

            let fetch_info_from_cache = data_contract_cache
                .get(contract_id, Some(&transaction))
                .expect("should be present");

            assert_eq!(fetch_info_from_cache, fetch_info_global)
        }

        #[test]
        fn test_get_from_transactional_cache() {
            let drive = setup_drive(None);
            let transaction = drive.grove.start_transaction();

            let mut data_contract_cache = DataContractCache::new(10, 10);

            let fetch_info_transactional = Arc::new(ContractFetchInfo::default());

            let contract_id = fetch_info_transactional.contract.id().to_buffer();

            data_contract_cache
                .transactional_cache
                .insert(&transaction, Arc::clone(&fetch_info_transactional));

            let fetch_info_from_cache = data_contract_cache
                .get(contract_id, Some(&transaction))
                .expect("should be present");

            assert_eq!(fetch_info_from_cache, fetch_info_transactional)
        }

        #[test]
        fn test_get_from_correct_transactional_cache() {
            let drive = setup_drive(None);
            let transaction1 = drive.grove.start_transaction();
            let transaction2 = drive.grove.start_transaction();

            let mut data_contract_cache = DataContractCache::new(10, 10);

            // Create transactional contract 1
            let fetch_info_transactional1 = Arc::new(ContractFetchInfo::default());

            let contract_id = fetch_info_transactional1.contract.id().to_buffer();

            data_contract_cache
                .transactional_cache
                .insert(&transaction1, Arc::clone(&fetch_info_transactional1));

            // Create transactional contract 2
            let mut fetch_info_transactional2 = ContractFetchInfo::default();

            fetch_info_transactional2.contract.increment_version();

            let fetch_info_transactional2 = Arc::new(fetch_info_transactional2);

            data_contract_cache
                .transactional_cache
                .insert(&transaction2, Arc::clone(&fetch_info_transactional2));

            // Get a contract for contract 1

            let fetch_info_from_transaction1 = data_contract_cache
                .get(contract_id, Some(&transaction1))
                .expect("should be present");

            assert_eq!(fetch_info_from_transaction1, fetch_info_transactional1);

            // Get a contract for contract 2

            let fetch_info_from_transaction2 = data_contract_cache
                .get(contract_id, Some(&transaction2))
                .expect("should be present");

            assert_eq!(fetch_info_from_transaction2, fetch_info_transactional2);

            // Get a contract without transaction

            let fetch_info_from_global_cache = data_contract_cache.get(contract_id, None);

            assert!(fetch_info_from_global_cache.is_none());
        }
    }
}
