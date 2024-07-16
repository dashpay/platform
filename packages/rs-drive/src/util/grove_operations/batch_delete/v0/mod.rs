use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::{push_drive_operation_result, BatchDeleteApplyType};
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::operations::delete::DeleteOptions;
use grovedb::{GroveDb, TransactionArg};
use grovedb_path::SubtreePath;
use grovedb_storage::rocksdb_storage::RocksDbStorage;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Pushes a "delete element" operation to `drive_operations`.
    pub(crate) fn batch_delete_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        apply_type: BatchDeleteApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let current_batch_operations =
            LowLevelDriveOperation::grovedb_operations_batch(drive_operations);
        let options = DeleteOptions {
            allow_deleting_non_empty_trees: false,
            deleting_non_empty_trees_returns_error: true,
            base_root_storage_is_free: true,
            validate_tree_at_path_exists: false, //todo: not sure about this one
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
                path,
                key,
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
            // we also add the actual delete operation
            drive_operations.push(GroveOperation(delete_operation))
        }

        Ok(())
    }
}
