use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::GroveDbOpBatch;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Applies a batch of groveDB operations if apply is True, otherwise gets the cost of the operations.
    pub(crate) fn apply_batch_grovedb_operations_v0(
        &self,
        estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: GroveDbOpBatch,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        if let Some(estimated_layer_info) = estimated_costs_only_with_layer_info {
            // Leave this for future debugging
            // for (k, v) in estimated_layer_info.iter() {
            //     let path = k
            //         .to_path()
            //         .iter()
            //         .map(|k| hex::encode(k.as_slice()))
            //         .join("/");
            //     dbg!(path, v);
            // }
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
