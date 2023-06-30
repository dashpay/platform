mod v0;


use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::block::epoch::Epoch;
use dpp::state_transition::fee::fee_result::FeeResult;
use dpp::version::drive_versions::DriveVersion;
use crate::drive::contract::ContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::error::drive::DriveError;
use crate::fee::op::LowLevelDriveOperation;

impl Drive {
    /// Retrieves the specified contract.
    ///
    /// The method version is selected based on the `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A contract ID as a 32-byte array. The contract ID is used to
    ///   fetch the corresponding contract.
    /// * `add_to_cache_if_pulled` - A boolean indicating whether to add the fetched contract to the
    ///   cache if it was pulled from storage.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for fetching the contract.
    /// * `epoch` - An optional reference to an `Epoch` object. If provided, the function calculates
    ///   the fee for the contract operations.
    /// * `drive_version` - The version of the drive used to select the correct method version.
    ///
    /// # Returns
    ///
    /// * `Result<(Option<FeeResult>, Option<Arc<ContractFetchInfo>>), Error>` - If successful,
    ///   returns a tuple containing an `Option` with the `FeeResult` (if an epoch was provided) and
    ///   an `Option` containing an `Arc` to the fetched `ContractFetchInfo`. If an error occurs
    ///   during the contract fetching or fee calculation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract fetching or fee calculation fails or if the
    /// drive version does not match any of the implemented method versions.
    pub fn get_contract_with_fetch_info_and_fee(
        &self,
        contract_id: [u8; 32],
        add_to_cache_if_pulled: bool,
        transaction: TransactionArg,
        epoch: Option<&Epoch>,
        drive_version: &DriveVersion,
    ) -> Result<(Option<FeeResult>, Option<Arc<ContractFetchInfo>>), Error> {
        match drive_version.methods.contract.get.get_contract_with_fetch_info {
            0 => self.get_contract_with_fetch_info_and_fee_v0(contract_id, epoch, add_to_cache_if_pulled, transaction, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_contract_with_fetch_info_and_fee".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Retrieves the specified contract.
    ///
    /// The method version is selected based on the `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A contract ID as a 32-byte array. The contract ID is used to
    ///   fetch the corresponding contract.
    /// * `add_to_cache_if_pulled` - A boolean indicating whether to add the fetched contract to the
    ///   cache if it was pulled from storage.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for fetching the contract.
    /// * `drive_version` - The version of the drive used to select the correct method version.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Arc<ContractFetchInfo>>, Error>` - If successful, returns an `Option` containing a
    ///   reference to the fetched `Contract`. If an error occurs during the contract fetching,
    ///   returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract fetching fails or if the
    /// drive version does not match any of the implemented method versions.
    pub fn get_contract_with_fetch_info(
        &self,
        contract_id: [u8; 32],
        add_to_cache_if_pulled: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Option<Arc<ContractFetchInfo>>, Error> {
        match drive_version.methods.contract.get.get_contract_with_fetch_info {
            0 => self.get_contract_with_fetch_info_v0(contract_id, add_to_cache_if_pulled, transaction),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_contract_with_fetch_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Returns the contract with fetch info and operations with the given ID.
    ///
    /// The method version is selected based on the `DriveVersion`.
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
    ///   for fetching the contract.
    /// * `drive_version` - The version of the drive used to select the correct method version.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Arc<ContractFetchInfo>>, Error>` - If successful, returns an `Option` containing a
    ///   reference to the fetched `Contract` and related operations. If an error occurs during the contract fetching,
    ///   returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract fetching fails or if the
    /// drive version does not match any of the implemented method versions.

    pub(crate) fn get_contract_with_fetch_info_and_add_to_operations(
        &self,
        contract_id: [u8; 32],
        epoch: Option<&Epoch>,
        add_to_cache_if_pulled: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Option<Arc<ContractFetchInfo>>, Error> {
        match drive_version.methods.contract.get.get_contract_with_fetch_info {
            0 => self.get_contract_with_fetch_info_and_add_to_operations_v0(contract_id, epoch, add_to_cache_if_pulled, transaction, drive_operations, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_contract_with_fetch_info_and_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}