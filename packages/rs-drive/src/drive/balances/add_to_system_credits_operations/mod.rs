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
    /// Provides the operations needed to add to system credits
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount of system credits to be added.
    /// * `estimated_costs_only_with_layer_info` - An optional mutable reference to a HashMap which contains the estimated costs for each layer information.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used for adding to the system credits.
    /// * `drive_version` - A `DriveVersion` object specifying the version of the Drive.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<LowLevelDriveOperation>, Error>` - If successful, returns a vector of `LowLevelDriveOperation`. If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the version of the Drive is unknown.
    pub fn add_to_system_credits_operations(
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
            .add_to_system_credits_operations
        {
            0 => self.add_to_system_credits_operations_v0(
                amount,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_to_system_credits_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
