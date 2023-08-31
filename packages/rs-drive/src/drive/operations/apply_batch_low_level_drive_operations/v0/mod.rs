use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::op::LowLevelDriveOperation::GroveOperation;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Applies a batch of Drive operations to groveDB.
    pub(super) fn apply_batch_low_level_drive_operations_v0(
        &self,
        estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: Vec<LowLevelDriveOperation>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let grove_db_operations =
            LowLevelDriveOperation::grovedb_operations_batch(&batch_operations);
        self.apply_batch_grovedb_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            grove_db_operations,
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
