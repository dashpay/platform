mod v0;

use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::block::epoch::Epoch;
use grovedb_costs::{CostResult, CostsExt, OperationCost};

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

use std::sync::Arc;

/// Drive contract fetching methods.
impl Drive {
    /// Fetches a contract.
    ///
    /// This method delegates the contract fetching to the appropriate versioned method
    /// according to the drive version.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - The ID of the contract to be fetched.
    /// * `epoch` - An optional reference to an `Epoch` object. If provided, the function calculates
    ///   the fee for the contract operations.
    /// * `known_keeps_history` - An optional boolean indicating whether the contract keeps its history.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for fetching the contract.
    /// * `drive_version` - The `DriveVersion` to determine which version of contract fetching to use.
    ///
    /// # Returns
    ///
    /// * `CostResult<Option<Arc<DataContractFetchInfo>>, Error>` - If successful, returns a `CostResult`
    ///   containing an `Option` with an `Arc` to the fetched `ContractFetchInfo`. If an error occurs
    ///   during the contract fetching or fee calculation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract fetching or fee calculation fails, or if
    /// the provided drive version does not match any known versions.
    pub fn fetch_contract(
        &self,
        contract_id: [u8; 32],
        epoch: Option<&Epoch>,
        known_keeps_history: Option<bool>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> CostResult<Option<Arc<DataContractFetchInfo>>, Error> {
        match platform_version.drive.methods.contract.get.fetch_contract {
            0 => self.fetch_contract_v0(
                contract_id,
                epoch,
                known_keeps_history,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract".to_string(),
                known_versions: vec![0],
                received: version,
            }))
            .wrap_with_cost(OperationCost::default()),
        }
    }

    /// Fetches a contract and adds operations.
    ///
    /// This method delegates the contract fetching and operation adding to the appropriate versioned method
    /// according to the drive version.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - The ID of the contract to be fetched.
    /// * `epoch` - An optional reference to an `Epoch` object. If provided, the function calculates
    ///   the fee for the contract operations.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for fetching the contract.
    /// * `drive_operations` - A mutable reference to a vector of `LowLevelDriveOperation` objects,
    ///   where the operations for the fetched contract will be added.
    /// * `drive_version` - The `DriveVersion` to determine which version of contract fetching and operation adding to use.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Arc<DataContractFetchInfo>>, Error>` - If successful, returns an `Option` with an `Arc`
    ///   to the fetched `ContractFetchInfo`. If an error occurs during the contract fetching or operation adding,
    ///   returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract fetching or operation adding fails, or if
    /// the provided drive version does not match any known versions.
    pub(crate) fn fetch_contract_and_add_operations(
        &self,
        contract_id: [u8; 32],
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContractFetchInfo>>, Error> {
        match platform_version.drive.methods.contract.get.fetch_contract {
            0 => self.fetch_contract_and_add_operations_v0(
                contract_id,
                epoch,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_and_add_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
