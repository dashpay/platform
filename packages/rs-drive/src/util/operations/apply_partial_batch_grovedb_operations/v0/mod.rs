use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::GroveError;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::{GroveDbOp, KeyInfoPath, OpsByLevelPath};
use grovedb::{EstimatedLayerInformation, TransactionArg};
use grovedb_costs::storage_cost::StorageCost;
use grovedb_costs::OperationCost;
use std::collections::HashMap;

impl Drive {
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
    /// Applies a partial batch of groveDB operations if apply is True, otherwise gets the cost of the operations.
    pub(crate) fn apply_partial_batch_grovedb_operations_v0(
        &self,
        estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        mut batch_operations: GroveDbOpBatch,
        mut add_on_operations: impl FnMut(
            &OperationCost,
            &Option<OpsByLevelPath>,
        ) -> Result<Vec<GroveDbOp>, GroveError>,
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
            // the estimated fees are the same for partial batches
            let additional_operations = add_on_operations(
                &OperationCost {
                    seek_count: 1,
                    storage_cost: StorageCost {
                        added_bytes: 1,
                        replaced_bytes: 1,
                        removed_bytes: Default::default(),
                    },
                    storage_loaded_bytes: 1,
                    hash_node_calls: 1,
                },
                &None,
            )?;
            batch_operations.extend(additional_operations);
            self.grove_batch_operations_costs(
                batch_operations,
                estimated_layer_info,
                false,
                drive_operations,
                drive_version,
            )?;
        } else {
            self.grove_apply_partial_batch_with_add_costs(
                batch_operations,
                false,
                transaction,
                add_on_operations,
                drive_operations,
                drive_version,
            )?;
        }
        Ok(())
    }
}
