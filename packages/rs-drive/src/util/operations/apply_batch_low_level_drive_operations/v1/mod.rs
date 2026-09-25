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
        let (grove_db_operations, ephemeral_batches, mut other_operations) =
            LowLevelDriveOperation::grovedb_operations_batch_consume_split_ephemeral(
                batch_operations,
            );
        // Ephemeral operations (a `timeRange` index's TTL'd sub-levels, the
        // writes of a document whose type declares a `ttl`) apply as their
        // own batch per pricing rule, so each batch's cost is known
        // separately and can be consumed under that rule. Cloning the layer
        // info keeps the estimation path symmetric: the dry run prices every
        // batch through the same worst-case machinery, under the same rule,
        // so estimated stays an upper bound of actual per fee class.
        //
        // The batches must still commit as one. GroveDB opens and commits
        // an owned transaction per batch when none is supplied, which would
        // leave an earlier batch committed if a later one failed — a
        // document row and its permanent index entries without their TTL'd
        // entries. Span them with one owned transaction instead and commit
        // only after every batch applied.
        let batch_count = usize::from(!grove_db_operations.is_empty())
            + ephemeral_batches
                .iter()
                .filter(|(_, operations)| !operations.is_empty())
                .count();
        let owned_transaction = (transaction.is_none()
            && estimated_costs_only_with_layer_info.is_none()
            && batch_count > 1)
            .then(|| self.grove.start_transaction());
        let transaction = owned_transaction.as_ref().or(transaction);
        let ephemeral_layer_infos: Vec<_> = ephemeral_batches
            .iter()
            .map(|_| estimated_costs_only_with_layer_info.clone())
            .collect();
        if !grove_db_operations.is_empty() {
            self.apply_batch_grovedb_operations(
                estimated_costs_only_with_layer_info,
                transaction,
                grove_db_operations,
                drive_operations,
                drive_version,
            )?;
        }
        for ((pricing, ephemeral_grove_db_operations), ephemeral_layer_info) in
            ephemeral_batches.into_iter().zip(ephemeral_layer_infos)
        {
            if ephemeral_grove_db_operations.is_empty() {
                continue;
            }
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
                    .map(|operation| operation.retag_ephemeral_with(pricing)),
            );
        }
        drive_operations.append(&mut other_operations);
        if let Some(owned_transaction) = owned_transaction {
            self.commit_transaction(owned_transaction, drive_version)?;
        }
        Ok(())
    }
}
