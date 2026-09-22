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
    pub(super) fn get_contract_with_fetch_info_v0(
        &self,
        contract_id: [u8; 32],
        add_to_cache_if_pulled: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContractFetchInfo>>, Error> {
        self.get_contract_with_fetch_info_and_add_to_operations_v0(
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
    pub(super) fn get_contract_with_fetch_info_and_fee_v0(
        &self,
        contract_id: [u8; 32],
        epoch: Option<&Epoch>,
        add_to_cache_if_pulled: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<FeeResult>, Option<Arc<DataContractFetchInfo>>), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = Vec::new();

        let contract_fetch_info = self.get_contract_with_fetch_info_and_add_to_operations_v0(
            contract_id,
            epoch,
            add_to_cache_if_pulled,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let fee_result = epoch.map_or(Ok(None), |epoch| {
            Drive::calculate_fee(
                None,
                Some(drive_operations),
                epoch,
                self.config.epochs_per_era,
                platform_version,
                None,
            )
            .map(Some)
        })?;
        Ok((fee_result, contract_fetch_info))
    }

    /// Returns the contract with fetch info and operations with the given ID.
    ///
    /// `add_to_cache_if_pulled` is whether this call may write to the cache at all: storing a
    /// contract it pulled from state, and storing the fee it calculated for a cached contract.
    /// Validation modes that run off the consensus thread (`check_tx`) pass `false`, so that
    /// only block execution ever writes the block cache.
    #[inline(always)]
    pub(super) fn get_contract_with_fetch_info_and_add_to_operations_v0(
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

        match self.cache.data_contracts.get(contract_id, is_transactional) {
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
                // we only need to pay if epoch is set
                if let Some(epoch) = epoch {
                    let fee = if let Some(known_fee) = &contract_fetch_info.fee {
                        known_fee.clone()
                    } else {
                        // we need to calculate new fee
                        let op = vec![CalculatedCostOperation(contract_fetch_info.cost.clone())];
                        let fee = Drive::calculate_fee(
                            None,
                            Some(op),
                            epoch,
                            self.config.epochs_per_era,
                            platform_version,
                            None,
                        )?;

                        if add_to_cache_if_pulled {
                            let updated_contract_fetch_info = Arc::new(DataContractFetchInfo {
                                contract: contract_fetch_info.contract.clone(),
                                storage_flags: contract_fetch_info.storage_flags.clone(),
                                cost: contract_fetch_info.cost.clone(),
                                fee: Some(fee.clone()),
                            });
                            // we override the cache for the contract as the fee is now calculated
                            cache_contract(updated_contract_fetch_info);
                        }

                        fee
                    };
                    drive_operations.push(PreCalculatedFeeResult(fee));
                }
                Ok(Some(contract_fetch_info))
            }
        }
    }
}
