#![allow(clippy::result_large_err)] // Operation application returns drive::Error with rich causes
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Applies a batch of Drive operations to groveDB.
    pub(crate) fn apply_batch_low_level_drive_operations_v0(
        &self,
        estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: Vec<LowLevelDriveOperation>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let (grove_db_operations, ephemeral_grove_db_operations, other_operations) =
            LowLevelDriveOperation::grovedb_operations_batch_consume_split_ephemeral(
                batch_operations,
            );
        // The ephemeral (TTL'd-subtree) operations apply as their own batch
        // so their cost is known separately and can be consumed at the
        // ephemeral price — added bytes to processing instead of storage.
        // Cloning the layer info keeps the estimation path symmetric: the
        // dry run prices the ephemeral batch through the same worst-case
        // machinery, under the same pricing rule, so estimated stays an
        // upper bound of actual per fee class.
        let ephemeral_layer_info = if ephemeral_grove_db_operations.is_empty() {
            None
        } else {
            estimated_costs_only_with_layer_info.clone()
        };
        if !grove_db_operations.is_empty() {
            self.apply_batch_grovedb_operations(
                estimated_costs_only_with_layer_info,
                transaction,
                grove_db_operations,
                drive_operations,
                drive_version,
            )?;
        }
        if !ephemeral_grove_db_operations.is_empty() {
            let mut ephemeral_cost_operations: Vec<LowLevelDriveOperation> = vec![];
            self.apply_batch_grovedb_operations(
                ephemeral_layer_info,
                transaction,
                ephemeral_grove_db_operations,
                &mut ephemeral_cost_operations,
                drive_version,
            )?;
            drive_operations.extend(
                ephemeral_cost_operations
                    .into_iter()
                    .map(LowLevelDriveOperation::retag_ephemeral),
            );
        }
        // Deferred TTL drainage runs only now, with every operation of the
        // transition applied: draining earlier could remove subtrees that
        // queued removals still targeted. One drain per level — requests
        // from indexes sharing a level collapse, so the per-write budget is
        // spent once per level. Unbilled, like the drain itself.
        let (drain_requests, mut other_operations): (Vec<_>, Vec<_>) = other_operations
            .into_iter()
            .partition(|op| matches!(op, LowLevelDriveOperation::TimeRangeTtlDrain(_)));
        let mut drained_levels: Vec<Vec<Vec<u8>>> = vec![];
        for request in drain_requests {
            let LowLevelDriveOperation::TimeRangeTtlDrain(request) = request else {
                continue;
            };
            if drained_levels.contains(&request.level_path) {
                continue;
            }
            self.drain_expired_time_range_buckets(&request, transaction, drive_version)?;
            drained_levels.push(request.level_path);
        }
        drive_operations.append(&mut other_operations);
        Ok(())
    }
}
