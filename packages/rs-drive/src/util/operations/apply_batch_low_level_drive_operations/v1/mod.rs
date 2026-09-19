#![allow(clippy::result_large_err)] // Operation application returns drive::Error with rich causes
use crate::drive::identity::contract_info::keys::coalesce_current_key_alias_operations;
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
    ///
    /// v1 first coalesces the current-key alias writes of bound keys stored under
    /// `MultipleReferenceToLatest`, so an identity update that registers and revokes keys
    /// covering one contract queues a single operation per alias slot (see
    /// [`coalesce_current_key_alias_operations`]).
    pub(crate) fn apply_batch_low_level_drive_operations_v1(
        &self,
        estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        mut batch_operations: Vec<LowLevelDriveOperation>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        coalesce_current_key_alias_operations(&mut batch_operations);
        let (grove_db_operations, ephemeral_grove_db_operations, mut other_operations) =
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
        // Two batches must still commit as one. GroveDB opens and commits
        // an owned transaction per batch when none is supplied, which would
        // leave the standing batch committed if the ephemeral one failed —
        // a document row and its permanent index entries without their
        // TTL'd entries. Span both with one owned transaction instead and
        // commit only after both applied.
        let owned_transaction = (transaction.is_none()
            && estimated_costs_only_with_layer_info.is_none()
            && !grove_db_operations.is_empty()
            && !ephemeral_grove_db_operations.is_empty())
        .then(|| self.grove.start_transaction());
        let transaction = owned_transaction.as_ref().or(transaction);
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
        drive_operations.append(&mut other_operations);
        if let Some(owned_transaction) = owned_transaction {
            self.commit_transaction(owned_transaction, drive_version)?;
        }
        Ok(())
    }
}
