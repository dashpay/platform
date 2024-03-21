use crate::drive::contract::{paths, DataContractFetchInfo};

use crate::drive::flags::StorageFlags;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::op::LowLevelDriveOperation::{CalculatedCostOperation, PreCalculatedFeeResult};
use dpp::block::epoch::Epoch;
use dpp::data_contract::DataContract;
use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};
use grovedb_costs::{
    cost_return_on_error_no_add, CostContext, CostResult, CostsExt, OperationCost,
};
use std::ops::AddAssign;
use std::sync::Arc;

impl Drive {
    /// Retrieves the specified contract from storage and inserts it into the cache.
    ///
    /// This function fetches the contract with the given ID from storage and, if successful,
    /// inserts the contract into the cache. Additionally, the fee for the contract operations
    /// is calculated if an epoch is provided.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A contract ID as a 32-byte array. The contract ID is used to
    ///   fetch the corresponding contract from storage.
    /// * `epoch` - An optional reference to an `Epoch` object. If provided, the function calculates
    ///   the fee for the contract operations.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for fetching the contract from storage.
    ///
    /// # Returns
    ///
    /// * `CostResult<Option<Arc<DataContractFetchInfo>>, Error>` - If successful, returns a `CostResult`
    ///   containing an `Option` with an `Arc` to the fetched `ContractFetchInfo`. If an error occurs
    ///   during the contract fetching or fee calculation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract fetching or fee calculation fails.
    #[inline(always)]
    pub(super) fn fetch_contract_v0(
        &self,
        contract_id: [u8; 32],
        epoch: Option<&Epoch>,
        known_keeps_history: Option<bool>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> CostResult<Option<Arc<DataContractFetchInfo>>, Error> {
        // As we want deterministic costs, we want the cost to always be the same for
        // fetching the contract.
        // We need to pass allow cache to false
        let (value, mut cost) = if known_keeps_history.unwrap_or_default() {
            let CostContext { value, cost } = self.grove.get_caching_optional(
                (&paths::contract_keeping_history_root_path(&contract_id)).into(),
                &[0],
                false,
                transaction,
            );
            (value, cost)
        } else {
            let CostContext { value, cost } = self.grove.get_raw_caching_optional(
                (&paths::contract_root_path(&contract_id)).into(),
                &[0],
                false,
                transaction,
            );
            (value, cost)
        };

        match value {
            Ok(Element::Item(stored_contract_bytes, element_flag)) => {
                let contract = cost_return_on_error_no_add!(
                    &cost,
                    DataContract::versioned_deserialize(
                        &stored_contract_bytes,
                        false,
                        platform_version
                    )
                    .map_err(Error::Protocol)
                );
                let drive_operation = CalculatedCostOperation(cost.clone());
                let fee = if let Some(epoch) = epoch {
                    Some(cost_return_on_error_no_add!(
                        &cost,
                        Drive::calculate_fee(
                            None,
                            Some(vec![drive_operation]),
                            epoch,
                            self.config.epochs_per_era,
                            platform_version
                        )
                    ))
                } else {
                    None
                };

                let storage_flags = cost_return_on_error_no_add!(
                    &cost,
                    StorageFlags::map_some_element_flags_ref(&element_flag)
                );
                let contract_fetch_info = Arc::new(DataContractFetchInfo {
                    contract,
                    storage_flags,
                    cost: cost.clone(),
                    fee,
                });

                Ok(Some(Arc::clone(&contract_fetch_info))).wrap_with_cost(cost)
            }
            Ok(Element::Tree(..)) => {
                // This contract might keep history, take the latest version
                let CostContext {
                    value,
                    cost: secondary_cost,
                } = self.grove.get_caching_optional(
                    (&paths::contract_keeping_history_root_path(&contract_id)).into(),
                    &[0],
                    false,
                    transaction,
                );

                cost.add_assign(secondary_cost);

                match value {
                    Ok(Element::Item(stored_contract_bytes, element_flag)) => {
                        let contract = cost_return_on_error_no_add!(
                            &cost,
                            DataContract::versioned_deserialize(
                                &stored_contract_bytes,
                                false,
                                platform_version
                            )
                            .map_err(Error::Protocol)
                        );
                        let drive_operation = CalculatedCostOperation(cost.clone());
                        let fee = if let Some(epoch) = epoch {
                            Some(cost_return_on_error_no_add!(
                                &cost,
                                Drive::calculate_fee(
                                    None,
                                    Some(vec![drive_operation]),
                                    epoch,
                                    self.config.epochs_per_era,
                                    platform_version
                                )
                            ))
                        } else {
                            None
                        };

                        let storage_flags = cost_return_on_error_no_add!(
                            &cost,
                            StorageFlags::map_some_element_flags_ref(&element_flag)
                        );

                        let contract_fetch_info = Arc::new(DataContractFetchInfo {
                            contract,
                            storage_flags,
                            cost: cost.clone(),
                            fee,
                        });

                        Ok(Some(Arc::clone(&contract_fetch_info))).wrap_with_cost(cost)
                    }
                    Ok(_) => Err(Error::Drive(DriveError::CorruptedContractPath(
                        "contract path did not refer to a contract element",
                    )))
                    .wrap_with_cost(cost),
                    Err(
                        grovedb::Error::PathKeyNotFound(_)
                        | grovedb::Error::PathParentLayerNotFound(_)
                        | grovedb::Error::PathNotFound(_),
                    ) => Ok(None).wrap_with_cost(cost),
                    Err(e) => Err(Error::GroveDB(e)).wrap_with_cost(cost),
                }
            }
            Ok(_) => Err(Error::Drive(DriveError::CorruptedContractPath(
                "contract path did not refer to a contract element",
            )))
            .wrap_with_cost(cost),
            Err(
                grovedb::Error::PathKeyNotFound(_)
                | grovedb::Error::PathParentLayerNotFound(_)
                | grovedb::Error::PathNotFound(_),
            ) => Ok(None).wrap_with_cost(cost),
            Err(e) => Err(Error::GroveDB(e)).wrap_with_cost(cost),
        }
    }

    /// Fetch contract from database and add operations
    #[inline(always)]
    pub(super) fn fetch_contract_and_add_operations_v0(
        &self,
        contract_id: [u8; 32],
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContractFetchInfo>>, Error> {
        let mut cost = OperationCost::default();

        //todo: there is a cost here that isn't returned on error
        // we should investigate if this could be a problem
        let maybe_contract_fetch_info = self
            .fetch_contract_v0(contract_id, epoch, None, transaction, platform_version)
            .unwrap_add_cost(&mut cost)?;

        if let Some(contract_fetch_info) = &maybe_contract_fetch_info {
            // we only need to pay if epoch is set
            if epoch.is_some() {
                let fee = contract_fetch_info
                    .fee
                    .as_ref()
                    .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "should be impossible to not have fee on something just fetched with an epoch",
                )))?;
                drive_operations.push(PreCalculatedFeeResult(fee.clone()));
            }
        } else if epoch.is_some() {
            drive_operations.push(CalculatedCostOperation(cost));
        }

        Ok(maybe_contract_fetch_info)
    }
}
