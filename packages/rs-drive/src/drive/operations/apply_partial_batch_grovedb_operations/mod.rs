mod v0;

use crate::drive::batch::GroveDbOpBatch;
use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use crate::fee::op::LowLevelDriveOperation;
use crate::query::GroveError;

use costs::OperationCost;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::{GroveDbOp, KeyInfoPath, OpsByLevelPath};
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Applies a partial batch of groveDB operations depending on the drive version.
    ///
    /// This method checks the drive version and calls the appropriate versioned method.
    /// If an unsupported version is passed, the function will return an `Error::Drive` with a `DriveError::UnknownVersionMismatch` error.
    ///
    /// # Arguments
    ///
    /// * `estimated_costs_only_with_layer_info` - An optional hashmap containing estimated layer information.
    /// * `transaction` - The transaction argument to pass to the groveDB.
    /// * `batch_operations` - A groveDB operation batch.
    /// * `add_on_operations` - A closure that processes additional operations.
    /// * `drive_operations` - A mutable reference to a vector of drive operations.
    /// * `drive_version` - A `DriveVersion` reference that dictates which version of the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - On success, returns `Ok(())`. On error, returns an `Error`.
    ///
    pub(crate) fn apply_partial_batch_grovedb_operations(
        &self,
        estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: GroveDbOpBatch,
        add_on_operations: impl FnMut(
            &OperationCost,
            &Option<OpsByLevelPath>,
        ) -> Result<Vec<GroveDbOp>, GroveError>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .operations
            .apply_partial_batch_grovedb_operations
        {
            0 => self.apply_partial_batch_grovedb_operations_v0(
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
                add_on_operations,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "apply_partial_batch_grovedb_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
