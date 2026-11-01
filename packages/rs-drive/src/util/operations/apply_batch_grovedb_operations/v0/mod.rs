#![allow(clippy::result_large_err)] // Operation application returns drive::Error with rich causes
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::GroveDbOpBatch;
use crate::util::estimated_layers::EstimatedLayers;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Applies a batch of groveDB operations if apply is True, otherwise gets the cost of the operations.
    pub(super) fn apply_batch_grovedb_operations_v0(
        &self,
        estimated_costs_only_with_layer_info: Option<EstimatedLayers>,
        transaction: TransactionArg,
        batch_operations: GroveDbOpBatch,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        if let Some(estimated_layer_info) = estimated_costs_only_with_layer_info {
            // Leave this for future debugging
            // dbg!(&estimated_layer_info);
            self.grove_batch_operations_costs(
                batch_operations,
                estimated_layer_info,
                false,
                drive_operations,
                drive_version,
            )?;
        } else {
            self.grove_apply_batch_with_add_costs(
                batch_operations,
                false,
                transaction,
                drive_operations,
                drive_version,
            )?;
        }
        Ok(())
    }
}
