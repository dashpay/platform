use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::query::GroveError;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::{KeyInfoPath, OpsByLevelPath};
use grovedb::{EstimatedLayerInformation, TransactionArg};
use grovedb_costs::OperationCost;
use std::collections::HashMap;

impl Drive {
    //this will be used later
    /// Applies a batch of Drive operations to groveDB.
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
    pub(crate) fn apply_partial_batch_low_level_drive_operations_v0(
        &self,
        estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: Vec<LowLevelDriveOperation>,
        mut add_on_operations: impl FnMut(
            &OperationCost,
            &Option<OpsByLevelPath>,
        ) -> Result<Vec<LowLevelDriveOperation>, GroveError>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let grove_db_operations =
            LowLevelDriveOperation::grovedb_operations_batch(&batch_operations);
        self.apply_partial_batch_grovedb_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            grove_db_operations,
            |cost, left_over_ops| {
                let additional_low_level_drive_operations = add_on_operations(cost, left_over_ops)?;
                let new_grove_db_operations = LowLevelDriveOperation::grovedb_operations_batch(
                    &additional_low_level_drive_operations,
                )
                .operations;
                Ok(new_grove_db_operations)
            },
            drive_operations,
            drive_version,
        )?;
        batch_operations.into_iter().for_each(|op| match op {
            GroveOperation(_) => (),
            _ => drive_operations.push(op),
        });
        Ok(())
    }
}
