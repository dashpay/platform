#![allow(clippy::result_large_err)] // Operation application returns drive::Error with rich causes
mod v0;
mod v1;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use crate::fees::op::LowLevelDriveOperation;

use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Applies a batch of Drive operations to groveDB depending on the drive version.
    ///
    /// This method checks the drive version and calls the appropriate versioned method.
    /// If an unsupported version is passed, the function will return an `Error::Drive` with a `DriveError::UnknownVersionMismatch` error.
    ///
    /// # Arguments
    ///
    /// * `estimated_costs_only_with_layer_info` - An optional hashmap containing estimated layer information.
    /// * `transaction` - The transaction argument to pass to the groveDB.
    /// * `batch_operations` - A vector of low-level drive operations to apply to the groveDB.
    /// * `drive_operations` - A mutable reference to a vector of drive operations.
    /// * `drive_version` - A `DriveVersion` reference that dictates which version of the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - On success, returns `Ok(())`. On error, returns an `Error`.
    ///   A batch still holding a [`LowLevelDriveOperation::RepaidIdentityDebt`] is refused
    ///   with `CorruptedCodeExecution`: its credits are owed to a fee pool by whoever built
    ///   the batch, and applying the rest would drop them.
    ///
    pub fn apply_batch_low_level_drive_operations(
        &self,
        estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: Vec<LowLevelDriveOperation>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .operations
            .apply_batch_low_level_drive_operations
        {
            0 => self.apply_batch_low_level_drive_operations_v0(
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
                drive_operations,
                drive_version,
            ),
            1 => {
                // Only `add_to_identity_balance_operations` 1 produces one, which the same
                // protocol version (14) selects as this generation, so the batches of earlier
                // versions are not scanned
                if LowLevelDriveOperation::holds_repaid_identity_debt(&batch_operations) {
                    return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "a repaid identity debt must be routed to the processing fee pool \
                         before its batch is applied",
                    )));
                }
                self.apply_batch_low_level_drive_operations_v1(
                    estimated_costs_only_with_layer_info,
                    transaction,
                    batch_operations,
                    drive_operations,
                    drive_version,
                )
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "apply_batch_low_level_drive_operations".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
