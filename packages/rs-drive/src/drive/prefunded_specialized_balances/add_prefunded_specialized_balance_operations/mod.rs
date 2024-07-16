mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::batch::KeyInfoPath;
use std::collections::HashMap;

use crate::fees::op::LowLevelDriveOperation;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::{EstimatedLayerInformation, TransactionArg};

impl Drive {
    /// Adds a new prefunded specialized balance
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount of credits to be added to the prefunded balance.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used for adding to the system credits.
    /// * `platform_version` - A `PlatformVersion` object specifying the version of Platform.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - If successful, returns `Ok(())`. If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the version of Platform is unknown.
    pub fn add_prefunded_specialized_balance_operations(
        &self,
        specialized_balance_id: Identifier,
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
            .prefunded_specialized_balances
            .add_prefunded_specialized_balance_operations
        {
            0 => self.add_prefunded_specialized_balance_operations_v0(
                specialized_balance_id,
                amount,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_prefunded_specialized_balance_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
