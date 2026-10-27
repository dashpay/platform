use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::pending_grove_operations::pending_grove_operations;
use crate::util::grove_operations::{push_drive_operation_result, BatchDeleteApplyType};
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::{GroveOp, KeyInfoPath};
use grovedb::operations::delete::DeleteOptions;
use grovedb::BackwardsReferences;
use grovedb::{Element, GroveDb, TransactionArg};
use grovedb_path::SubtreePath;
use grovedb_storage::rocksdb_storage::RocksDbStorage;

impl Drive {
    /// Pushes a "delete element" operation to `drive_operations` and returns the current element.
    /// If the element didn't exist does nothing.
    /// It is raw, because it does not use references.
    pub(crate) fn batch_remove_raw_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        apply_type: BatchDeleteApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Option<Element>, Error> {
        let options = DeleteOptions {
            // Drive stores no backward-reference participants; GroveDB checks the
            // claim for free from the value it reads for the write.
            backwards_references: BackwardsReferences::DontCheck,
            allow_deleting_non_empty_trees: false,
            deleting_non_empty_trees_returns_error: true,
            base_root_storage_is_free: true,
            validate_tree_at_path_exists: false, //todo: not sure about this one
        };

        // The first pending operation on this key, found as `remove_if_insert` found it on a
        // copy of the whole batch. That copy then went to GroveDB without the operation when it
        // was an insert, but GroveDB reads only the operations under the deleted element, never
        // this one, so its delete and cost are the same at every protocol version.
        let known_path = KeyInfoPath(path.to_vec().into_iter().map(KeyInfo::KnownKey).collect());
        let known_key = Some(KeyInfo::KnownKey(key.to_vec()));
        let needs_removal_from_state = match pending_grove_operations(drive_operations)
            .find(|op| op.path == known_path && op.key == known_key)
            .map(|op| &op.op)
        {
            Some(
                GroveOp::InsertOrReplace { element }
                | GroveOp::InsertOrReplaceDontCheckForBackwardsReferences { element },
            )
            | Some(
                GroveOp::Replace { element }
                | GroveOp::ReplaceDontCheckForBackwardsReferences { element },
            )
            | Some(
                GroveOp::Patch { element, .. }
                | GroveOp::PatchDontCheckForBackwardsReferences { element, .. },
            ) => return Ok(Some(element.clone())),
            Some(GroveOp::InsertTreeWithRootHash { .. }) => {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "we should not be seeing internal grovedb operations",
                )));
            }
            Some(GroveOp::Delete | GroveOp::DeleteDontCheckForBackwardsReferences)
            | Some(
                GroveOp::DeleteTree(_, _)
                | GroveOp::DeleteTreeDontCheckForBackwardsReferences(_, _),
            ) => false,
            _ => true,
        };

        let maybe_element = self.grove_get_raw_optional(
            path.clone(),
            key,
            (&apply_type).into(),
            transaction,
            drive_operations,
            drive_version,
        )?;
        if maybe_element.is_none()
            && matches!(
                &apply_type,
                &BatchDeleteApplyType::StatefulBatchDelete { .. }
            )
        {
            return Ok(None);
        }
        if needs_removal_from_state {
            let delete_operation = match apply_type {
                BatchDeleteApplyType::StatelessBatchDelete {
                    in_tree_type: is_sum_tree,
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
                    BackwardsReferences::DontCheck,
                    &drive_version.grove_version,
                )
                .map(|r| r.map(Some)),
                BatchDeleteApplyType::StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum,
                } => {
                    // Every protocol version builds the same delete and cost as with the copy
                    // of the whole pending batch: GroveDB reads the same operations, borrowed.
                    self.grove.delete_operation_for_delete_internal(
                        path,
                        key,
                        &options,
                        is_known_to_be_subtree_with_sum,
                        pending_grove_operations(drive_operations),
                        transaction,
                        &drive_version.grove_version,
                    )
                }
            };

            if let Some(delete_operation) =
                push_drive_operation_result(delete_operation, drive_operations)?
            {
                // we also add the actual delete operation
                drive_operations.push(GroveOperation(delete_operation))
            }
        }

        Ok(maybe_element)
    }
}
