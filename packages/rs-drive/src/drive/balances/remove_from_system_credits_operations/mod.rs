mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Returns the operations required to remove from system credits.
    ///
    /// System credits are removed when an identity withdraws some of their balance.
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount to remove from the system credits.
    /// * `estimated_costs_only_with_layer_info` - An optional mutable reference to a HashMap for storing the estimated layer information.
    /// * `transaction` - A `TransactionArg` object representing the transaction for which to perform the operations.
    /// * `drive_version` - A `DriveVersion` object specifying the version of the Drive.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<LowLevelDriveOperation>, Error>` - If successful, returns a vector of `LowLevelDriveOperation` objects representing the operations required to remove the amount from the system credits.
    ///   If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the version of the Drive is unknown.
    pub fn remove_from_system_credits_operations(
        &self,
        amount: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .balances
            .remove_from_system_credits_operations
        {
            0 => self.remove_from_system_credits_operations_v0(
                amount,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_from_system_credits_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
