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
use grovedb::batch::{BatchApplyOptions, OpsByLevelPath, QualifiedGroveDbOp};
use grovedb::TransactionArg;
use grovedb_costs::OperationCost;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Applies the given groveDB operations batch with add-on operations and
    /// passes the costs to `push_drive_operation_result_with_refund_owners`.
    ///
    /// This generation splits removed bytes with the typed storage flags and
    /// records the owner of every owned removal on the cost operation it
    /// pushes. The flag update closure is the typed one as well, which also
    /// removes the previous generation's inline copy of that logic.
    pub(super) fn grove_apply_partial_batch_with_add_costs_v1(
        &self,
        ops: GroveDbOpBatch,
        validate: bool,
        transaction: TransactionArg,
        add_on_operations: impl FnMut(
            &OperationCost,
            &Option<OpsByLevelPath>,
        ) -> Result<Vec<QualifiedGroveDbOp>, GroveError>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        if ops.is_empty() {
            return Err(Error::Drive(DriveError::BatchIsEmpty(
                "batch is empty when trying to apply partial batch with add costs".to_string(),
            )));
        }
        if self.config.batching_consistency_verification {
            let consistency_results =
                QualifiedGroveDbOp::verify_consistency_of_operations(&ops.operations);
            if !consistency_results.is_empty() {
                return Err(Error::Drive(DriveError::GroveDBInsertion(
                    "insertion order error",
                )));
            }
        }

        let mut refund_owners = RefundOwnersByIdentifier::new();

        let cost_context = self.grove.apply_partial_batch_with_element_flags_update(
            ops.operations,
            Some(BatchApplyOptions {
                validate_insertion_does_not_override: validate,
                validate_insertion_does_not_override_tree: validate,
                disable_operation_consistency_check: false,
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
            add_on_operations,
            transaction,
            &drive_version.grove_version,
        );
        push_drive_operation_result_with_refund_owners(
            cost_context,
            refund_owners,
            drive_operations,
        )
    }
}
