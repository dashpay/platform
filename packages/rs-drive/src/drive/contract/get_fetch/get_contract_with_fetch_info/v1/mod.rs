use crate::drive::contract::DataContractFetchInfo;

use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::{CalculatedCostOperation, PreCalculatedFeeResult};
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::sync::Arc;

impl Drive {
    /// Retrieves the specified contract.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A contract ID as a 32-byte array. The contract ID is used to
    ///   fetch the corresponding contract.
    /// * `add_to_cache_if_pulled` - A boolean indicating whether to add the fetched contract to the
    ///   cache if it was pulled from storage.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for fetching the contract.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Arc<Contract>>, Error>` - If successful, returns an `Option` containing a
    ///   reference to the fetched `Contract`. If an error occurs during the contract fetching,
    ///   returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract fetching fails.
    #[inline(always)]
    pub(super) fn get_contract_with_fetch_info_v1(
        &self,
        contract_id: [u8; 32],
        add_to_cache_if_pulled: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContractFetchInfo>>, Error> {
        self.get_contract_with_fetch_info_and_add_to_operations_v1(
            contract_id,
            None,
            add_to_cache_if_pulled,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    /// Retrieves the specified contract along with its fetch info and calculates the fee if an epoch is provided.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A contract ID as a 32-byte array. The contract ID is used to
    ///   fetch the corresponding contract and its fetch info.
    /// * `epoch` - An optional reference to an `Epoch` object. If provided, the function calculates
    ///   the fee for the contract operations.
    /// * `add_to_cache_if_pulled` - A boolean indicating whether to add the fetched contract to the
    ///   cache if it was pulled from storage.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///
    ///   for fetching the contract.
    ///
    /// # Returns
    ///
    /// * `Result<(Option<FeeResult>, Option<Arc<DataContractFetchInfo>>), Error>` - If successful,
    ///   returns a tuple containing an `Option` with the `FeeResult` (if an epoch was provided) and
    ///   an `Option` containing an `Arc` to the fetched `ContractFetchInfo`. If an error occurs
    ///   during the contract fetching or fee calculation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract fetching or fee calculation fails.
    #[inline(always)]
    pub(super) fn get_contract_with_fetch_info_and_fee_v1(
        &self,
        contract_id: [u8; 32],
        epoch: Option<&Epoch>,
        add_to_cache_if_pulled: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<FeeResult>, Option<Arc<DataContractFetchInfo>>), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = Vec::new();

        let contract_fetch_info = self.get_contract_with_fetch_info_and_add_to_operations_v1(
            contract_id,
            epoch,
            add_to_cache_if_pulled,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let fee_result = epoch.map_or(Ok(None), |epoch| {
            self.calculate_fee(None, Some(drive_operations), epoch, platform_version)
                .map(Some)
        })?;
        Ok((fee_result, contract_fetch_info))
    }

    /// Returns the contract with fetch info and operations with the given ID.
    ///
    /// Generation 0 with two changes: a cached copy is served only when it was parsed under
    /// this reader's protocol version (`DataContractCache::get_parsed_under`), and a cache hit
    /// is billed its cost priced under this reader's protocol version, never a fee stored on
    /// the entry.
    ///
    /// `add_to_cache_if_pulled` is whether this call may store a contract it pulled from state
    /// in the cache.
    #[inline(always)]
    pub(super) fn get_contract_with_fetch_info_and_add_to_operations_v1(
        &self,
        contract_id: [u8; 32],
        epoch: Option<&Epoch>,
        add_to_cache_if_pulled: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContractFetchInfo>>, Error> {
        let is_transactional = transaction.is_some();

        // A read with no transaction is a committed-state read. Its snapshot is taken before
        // anything is looked up so that, should a block commit while this call runs, whatever
        // it read is not published to the global cache (see `CommittedGeneration`).
        let observed_generation =
            (!is_transactional).then(|| self.cache.data_contracts.committed_generation());

        let cache_contract = |fetch_info: Arc<DataContractFetchInfo>| match observed_generation {
            Some(observed) => self
                .cache
                .data_contracts
                .insert_committed(fetch_info, observed),
            None => self.cache.data_contracts.insert_block(fetch_info),
        };

        // Only a copy parsed under this reader's protocol version is served: the same stored
        // contract may parse differently under another, and a query at the previous version
        // can have cached its parse. Anything else is a miss, read from state below as a node
        // with an empty cache reads it, and cached if this call may cache.
        match self.cache.data_contracts.get_parsed_under(
            contract_id,
            is_transactional,
            platform_version,
        ) {
            None => {
                let maybe_contract_fetch_info = self.fetch_contract_and_add_operations(
                    contract_id,
                    epoch,
                    transaction,
                    drive_operations,
                    platform_version,
                )?;

                if add_to_cache_if_pulled {
                    // Store a contract in cache if present
                    if let Some(contract_fetch_info) = &maybe_contract_fetch_info {
                        cache_contract(Arc::clone(contract_fetch_info));
                    };
                }
                Ok(maybe_contract_fetch_info)
            }
            Some(contract_fetch_info) => {
                // A hit is billed what a cold read of the entry bills: its cost priced under
                // this reader's protocol version, as the fetch prices it. The fee an entry may
                // carry is not used, since a reader of another protocol version, at generation
                // 0, can have priced it under that version's schedule.
                if let Some(epoch) = epoch {
                    let op = vec![CalculatedCostOperation(contract_fetch_info.cost.clone())];
                    let fee = self.calculate_fee(None, Some(op), epoch, platform_version)?;
                    drive_operations.push(PreCalculatedFeeResult(fee));
                }
                Ok(Some(contract_fetch_info))
            }
        }
    }
}
