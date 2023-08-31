use crate::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::drive::grove_operations::{push_drive_operation_result, BatchDeleteUpTreeApplyType};
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::op::LowLevelDriveOperation::GroveOperation;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::operations::delete::DeleteUpTreeOptions;
use grovedb::{GroveDb, TransactionArg};
use grovedb_storage::rocksdb_storage::RocksDbStorage;

impl Drive {
    /// Pushes a "delete up tree while empty" operation to `drive_operations`.
    pub(super) fn batch_delete_up_tree_while_empty_v0(
        &self,
        path: KeyInfoPath,
        key: &[u8],
        stop_path_height: Option<u16>,
        apply_type: BatchDeleteUpTreeApplyType,
        transaction: TransactionArg,
        check_existing_operations: &Option<&mut Vec<LowLevelDriveOperation>>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        //these are the operations in the current operations (eg, delete/add)
        let mut current_batch_operations =
            LowLevelDriveOperation::grovedb_operations_batch(drive_operations);

        //These are the operations in the same batch, but in a different operation
        if let Some(existing_operations) = check_existing_operations {
            let mut other_batch_operations =
                LowLevelDriveOperation::grovedb_operations_batch(existing_operations);
            current_batch_operations.append(&mut other_batch_operations);
        }
        let cost_context = match apply_type {
            BatchDeleteUpTreeApplyType::StatelessBatchDelete {
                estimated_layer_info,
            } => GroveDb::average_case_delete_operations_for_delete_up_tree_while_empty::<
                RocksDbStorage,
            >(
                &path,
                &KeyInfo::KnownKey(key.to_vec()),
                stop_path_height,
                true,
                estimated_layer_info,
            ),
            BatchDeleteUpTreeApplyType::StatefulBatchDelete {
                is_known_to_be_subtree_with_sum,
            } => {
                let options = DeleteUpTreeOptions {
                    allow_deleting_non_empty_trees: false,
                    deleting_non_empty_trees_returns_error: true,
                    base_root_storage_is_free: true,
                    validate_tree_at_path_exists: false,
                    stop_path_height,
                };
                self.grove.delete_operations_for_delete_up_tree_while_empty(
                    path.to_path_refs().as_slice().into(),
                    key,
                    &options,
                    is_known_to_be_subtree_with_sum,
                    current_batch_operations.operations,
                    transaction,
                )
            }
        };
        let delete_operations = push_drive_operation_result(cost_context, drive_operations)?;
        delete_operations
            .into_iter()
            .for_each(|op| drive_operations.push(GroveOperation(op)));

        Ok(())
    }
}
