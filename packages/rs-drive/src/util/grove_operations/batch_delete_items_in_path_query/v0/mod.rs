use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::{push_drive_operation_result, BatchDeleteApplyType};
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::operations::delete::DeleteOptions;
use grovedb::query_result_type::{QueryResultElement, QueryResultElements};
use grovedb::{GroveDb, PathQuery, TransactionArg};
use grovedb_storage::rocksdb_storage::RocksDbStorage;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Version 0 implementation of the "delete multiple elements" operation based on a `PathQuery`.
    /// Deletes items in the specified path that match the given query.
    ///
    /// # Parameters
    /// * `path_query`: The path query specifying the items to delete within the path.
    /// * `error_if_intermediate_path_tree_not_present`: Tells the function to either error or do nothing if an intermediate tree is not present.
    /// * `apply_type`: The apply type for the delete operations.
    /// * `transaction`: The transaction argument.
    /// * `drive_operations`: The vector containing low-level drive operations.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::CorruptedCodeExecution)` if the operation is not supported.
    pub(super) fn batch_delete_items_in_path_query_v0(
        &self,
        path_query: &PathQuery,
        error_if_intermediate_path_tree_not_present: bool,
        apply_type: BatchDeleteApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        // Fetch the elements that match the path query
        let query_result = self.grove_get_raw_path_query_with_optional_v0(
            path_query,
            error_if_intermediate_path_tree_not_present,
            transaction,
            drive_operations,
            drive_version,
        )?;

        // Iterate over each element and add a delete operation for it
        for (path, key, _) in query_result {
            let current_batch_operations =
                LowLevelDriveOperation::grovedb_operations_batch(drive_operations);
            let options = DeleteOptions {
                allow_deleting_non_empty_trees: false,
                deleting_non_empty_trees_returns_error: true,
                base_root_storage_is_free: true,
                validate_tree_at_path_exists: false,
            };
            let delete_operation = match apply_type {
                BatchDeleteApplyType::StatelessBatchDelete {
                    is_sum_tree,
                    estimated_key_size,
                    estimated_value_size,
                } => GroveDb::average_case_delete_operation_for_delete::<RocksDbStorage>(
                    &KeyInfoPath::from_known_owned_path(path.to_vec()),
                    &KeyInfo::KnownKey(key.to_vec()),
                    is_sum_tree,
                    false,
                    true,
                    0,
                    (estimated_key_size, estimated_value_size),
                    &drive_version.grove_version,
                )
                .map(|r| r.map(Some)),
                BatchDeleteApplyType::StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum,
                } => self.grove.delete_operation_for_delete_internal(
                    (path.as_slice()).into(),
                    key.as_slice(),
                    &options,
                    is_known_to_be_subtree_with_sum,
                    &current_batch_operations.operations,
                    transaction,
                    &drive_version.grove_version,
                ),
            };

            if let Some(delete_operation) =
                push_drive_operation_result(delete_operation, drive_operations)?
            {
                // Add the delete operation to the batch of drive operations
                drive_operations.push(GroveOperation(delete_operation));
            }
        }

        Ok(())
    }
}
