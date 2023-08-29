use crate::drive::grove_operations::push_drive_operation_result;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use grovedb::operations::delete::DeleteOptions;
use grovedb::TransactionArg;
use grovedb_path::SubtreePath;

impl Drive {
    /// Pushes the `OperationCost` of deleting an element in groveDB to `drive_operations`.
    pub(super) fn grove_delete_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        let options = DeleteOptions {
            allow_deleting_non_empty_trees: false,
            deleting_non_empty_trees_returns_error: true,
            base_root_storage_is_free: true,
            validate_tree_at_path_exists: false,
        };
        let cost_context = self.grove.delete(path, key, Some(options), transaction);
        push_drive_operation_result(cost_context, drive_operations)
    }
}
