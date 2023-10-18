use crate::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::drive::batch::GroveDbOpBatch;
use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::push_drive_operation_result;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::query::GroveError;
use grovedb::batch::{BatchApplyOptions, GroveDbOp};
use grovedb::TransactionArg;
use grovedb_costs::storage_cost::removal::StorageRemovedBytes::BasicStorageRemoval;
use grovedb_costs::storage_cost::transition::OperationStorageTransitionType;
use tracing::Level;

impl Drive {
    /// Applies the given groveDB operations batch and gets and passes the costs to `push_drive_operation_result`.
    pub(super) fn grove_apply_batch_with_add_costs_v0(
        &self,
        ops: GroveDbOpBatch,
        validate: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        if ops.is_empty() {
            return Err(Error::Drive(DriveError::BatchIsEmpty()));
        }
        // if ops.operations.len() < 500 {
        //     //no initialization
        //     dbg!("batch {:#?}", &ops);
        // }

        if self.config.batching_consistency_verification {
            let consistency_results = GroveDbOp::verify_consistency_of_operations(&ops.operations);
            if !consistency_results.is_empty() {
                tracing::error!(
                    ?consistency_results,
                    "grovedb consistency verification failed"
                );
                return Err(Error::Drive(DriveError::GroveDBInsertion(
                    "insertion order error",
                )));
            }
        }

        // Clone ops only if we log them
        #[cfg(feature = "grovedb_operations_logging")]
        let maybe_ops_for_logs = if tracing::event_enabled!(target: "drive_grovedb_operations", Level::TRACE)
        {
            Some(ops.clone())
        } else {
            None
        };

        let cost_context = self.grove.apply_batch_with_element_flags_update(
            ops.operations,
            Some(BatchApplyOptions {
                validate_insertion_does_not_override: validate,
                validate_insertion_does_not_override_tree: validate,
                allow_deleting_non_empty_trees: false,
                deleting_non_empty_trees_returns_error: true,
                disable_operation_consistency_check: !self.config.batching_consistency_verification,
                base_root_storage_is_free: true,
                batch_pause_height: None,
            }),
            |cost, old_flags, new_flags| {
                // if there were no flags before then the new flags are used
                if old_flags.is_none() {
                    return Ok(false);
                }
                // This could be none only because the old element didn't exist
                // If they were empty we get an error
                let maybe_old_storage_flags = StorageFlags::map_some_element_flags_ref(&old_flags)
                    .map_err(|_| {
                        GroveError::JustInTimeElementFlagsClientError(
                            "drive did not understand flags of old item being updated",
                        )
                    })?;
                let new_storage_flags = StorageFlags::from_element_flags_ref(new_flags)
                    .map_err(|_| {
                        GroveError::JustInTimeElementFlagsClientError(
                            "drive did not understand updated item flag information",
                        )
                    })?
                    .ok_or(GroveError::JustInTimeElementFlagsClientError(
                        "removing flags from an item with flags is not allowed",
                    ))?;
                match &cost.transition_type() {
                    OperationStorageTransitionType::OperationUpdateBiggerSize => {
                        let combined_storage_flags = StorageFlags::optional_combine_added_bytes(
                            maybe_old_storage_flags,
                            new_storage_flags,
                            cost.added_bytes,
                        )
                        .map_err(|_| {
                            GroveError::JustInTimeElementFlagsClientError(
                                "drive could not combine storage flags (new flags were bigger)",
                            )
                        })?;
                        let combined_flags = combined_storage_flags.to_element_flags();
                        // it's possible they got bigger in the same epoch
                        if combined_flags == *new_flags {
                            // they are the same there was no update
                            Ok(false)
                        } else {
                            *new_flags = combined_flags;
                            Ok(true)
                        }
                    }
                    OperationStorageTransitionType::OperationUpdateSmallerSize => {
                        let combined_storage_flags = StorageFlags::optional_combine_removed_bytes(
                            maybe_old_storage_flags,
                            new_storage_flags,
                            &cost.removed_bytes,
                        )
                        .map_err(|_| {
                            GroveError::JustInTimeElementFlagsClientError(
                                "drive could not combine storage flags (new flags were smaller)",
                            )
                        })?;
                        let combined_flags = combined_storage_flags.to_element_flags();
                        // it's possible they got bigger in the same epoch
                        if combined_flags == *new_flags {
                            // they are the same there was no update
                            Ok(false)
                        } else {
                            *new_flags = combined_flags;
                            Ok(true)
                        }
                    }
                    _ => Ok(false),
                }
            },
            |flags, removed_key_bytes, removed_value_bytes| {
                let maybe_storage_flags =
                    StorageFlags::from_element_flags_ref(flags).map_err(|_| {
                        GroveError::SplitRemovalBytesClientError(
                            "drive did not understand flags of item being updated",
                        )
                    })?;
                // if there were no flags before then the new flags are used
                match maybe_storage_flags {
                    None => Ok((
                        BasicStorageRemoval(removed_key_bytes),
                        BasicStorageRemoval(removed_value_bytes),
                    )),
                    Some(storage_flags) => storage_flags
                        .split_storage_removed_bytes(removed_key_bytes, removed_value_bytes),
                }
            },
            transaction,
        );

        #[cfg(feature = "grovedb_operations_logging")]
        if tracing::event_enabled!(target: "drive_grovedb_operations", Level::TRACE)
            && cost_context.value.is_ok()
        {
            let root_hash = self
                .grove
                .root_hash(transaction)
                .unwrap()
                .map_err(Error::GroveDB)?;

            tracing::trace!(
                target = "drive_grovedb_operations",
                ops = ?maybe_ops_for_logs.unwrap(),
                root_hash = ?root_hash,
                is_transactional = transaction.is_some(),
                "grovedb batch applied",
            );
        }

        push_drive_operation_result(cost_context, drive_operations)
    }
}
