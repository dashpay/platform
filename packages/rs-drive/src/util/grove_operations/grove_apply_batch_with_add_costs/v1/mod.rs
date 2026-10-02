use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::GroveError;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;
use crate::util::grove_operations::push_drive_operation_result_with_refund_owners;
use crate::util::storage_flags::StorageFlags;
use dpp::fee::refund_owner::RefundOwnersByIdentifier;
use grovedb::batch::{BatchApplyOptions, QualifiedGroveDbOp};
use grovedb::TransactionArg;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Applies the given groveDB operations batch and passes the costs to
    /// `push_drive_operation_result_with_refund_owners`.
    ///
    /// This generation splits removed bytes with the typed storage flags:
    /// every owned removal is sectioned under the owner's carrier key and
    /// the owner is recorded next to it, so the cost operation it pushes
    /// carries the recorded owners for the fee decoder to route. It accepts
    /// contract bucket owned flags, which the previous generation rejects.
    pub(super) fn grove_apply_batch_with_add_costs_v1(
        &self,
        ops: GroveDbOpBatch,
        validate: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        if ops.is_empty() {
            return Err(Error::Drive(DriveError::BatchIsEmpty(
                "batch is empty when trying to apply batch with add costs".to_string(),
            )));
        }

        if self.config.batching_consistency_verification {
            let consistency_results =
                QualifiedGroveDbOp::verify_consistency_of_operations(&ops.operations);
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
        let maybe_params_for_logs = if tracing::event_enabled!(target: "drive_grovedb_operations", tracing::Level::TRACE)
        {
            let root_hash = self
                .grove
                .root_hash(transaction, &drive_version.grove_version)
                .unwrap()
                .map_err(Error::from)?;

            Some((ops.clone(), root_hash))
        } else {
            None
        };

        let mut refund_owners = RefundOwnersByIdentifier::new();

        let cost_context = self.grove.apply_batch_with_element_flags_update(
            ops.operations,
            Some(BatchApplyOptions {
                validate_insertion_does_not_override: validate,
                validate_insertion_does_not_override_tree: validate,
                disable_operation_consistency_check: !self.config.batching_consistency_verification,
                base_root_storage_is_free: true,
                batch_pause_height: None,
            }),
            |cost, old_flags, new_flags| {
                StorageFlags::update_element_flags_typed(cost, old_flags, new_flags)
                    .map_err(|e| GroveError::JustInTimeElementFlagsClientError(e.to_string()))
            },
            |flags, removed_key_bytes, removed_value_bytes| {
                StorageFlags::split_removal_bytes_typed(
                    flags,
                    removed_key_bytes,
                    removed_value_bytes,
                    &mut refund_owners,
                )
                .map_err(|e| GroveError::SplitRemovalBytesClientError(e.to_string()))
            },
            transaction,
            &drive_version.grove_version,
        );

        #[cfg(feature = "grovedb_operations_logging")]
        if tracing::event_enabled!(target: "drive_grovedb_operations", tracing::Level::TRACE)
            && cost_context.value.is_ok()
        {
            if let Some((ops, previous_root_hash)) = maybe_params_for_logs {
                let root_hash = self
                    .grove
                    .root_hash(transaction, &drive_version.grove_version)
                    .unwrap()
                    .map_err(Error::from)?;

                tracing::trace!(
                    target: "drive_grovedb_operations",
                    ?ops,
                    ?root_hash,
                    ?previous_root_hash,
                    is_transactional = transaction.is_some(),
                    "grovedb batch applied",
                );
            }
        }

        push_drive_operation_result_with_refund_owners(
            cost_context,
            refund_owners,
            drive_operations,
        )
    }
}
