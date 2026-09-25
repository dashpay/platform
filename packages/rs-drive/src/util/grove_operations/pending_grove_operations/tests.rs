//! Every batch delete helper builds the same operations, costs included, with only the pending
//! operations GroveDB reads as it did with a copy of the whole pending batch.
//!
//! `old` holds the helpers as they were before, each copying the whole batch per delete; every
//! test builds the same batch through a helper and its old copy and compares the operation lists.

use super::COPIED_PENDING_GROVE_OPERATIONS;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::{
    CalculatedCostOperation, EphemeralGroveOperation, GroveOperation,
};
use crate::util::grove_operations::{
    BatchDeleteApplyType, BatchDeleteUpTreeApplyType, BatchMoveApplyType,
};
use crate::util::test_helpers::setup::setup_drive;
use grovedb::batch::{GroveOp, KeyInfoPath, QualifiedGroveDbOp};
use grovedb::operations::delete::DeleteOptions;
use grovedb::{BackwardsReferences, Element, MaybeTree, PathQuery, Query, SizedQuery, TreeType};
use grovedb_costs::OperationCost;
use grovedb_epoch_based_storage_flags::StorageFlags;
use platform_version::version::PlatformVersion;
use std::fmt::Debug;

/// The helpers as they were before, each copying the whole pending batch for GroveDB.
mod old {
    use crate::drive::Drive;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::fees::op::LowLevelDriveOperation::GroveOperation;
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::util::grove_operations::{
        push_drive_operation_result, BatchDeleteApplyType, BatchDeleteUpTreeApplyType,
        BatchMoveApplyType, QueryType,
    };
    use grovedb::batch::key_info::KeyInfo;
    use grovedb::batch::{GroveOp, KeyInfoPath, QualifiedGroveDbOp};
    use grovedb::operations::delete::{DeleteOptions, DeleteUpTreeOptions};
    use grovedb::query_result_type::{PathKeyElementTrio, QueryResultType};
    use grovedb::{BackwardsReferences, Element, GroveDb, PathQuery, TransactionArg};
    use grovedb_epoch_based_storage_flags::StorageFlags;
    use grovedb_path::SubtreePath;
    use grovedb_storage::rocksdb_storage::RocksDbStorage;
    use platform_version::version::drive_versions::DriveVersion;

    pub(super) fn batch_delete<B: AsRef<[u8]>>(
        drive: &Drive,
        path: SubtreePath<'_, B>,
        key: &[u8],
        apply_type: BatchDeleteApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let options = DeleteOptions {
            backwards_references: BackwardsReferences::DontCheck,
            allow_deleting_non_empty_trees: false,
            deleting_non_empty_trees_returns_error: true,
            base_root_storage_is_free: true,
            validate_tree_at_path_exists: false,
        };
        batch_delete_with_options(
            drive,
            path,
            key,
            apply_type,
            options,
            transaction,
            drive_operations,
            drive_version,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn batch_delete_with_options<B: AsRef<[u8]>>(
        drive: &Drive,
        path: SubtreePath<'_, B>,
        key: &[u8],
        apply_type: BatchDeleteApplyType,
        options: DeleteOptions,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let current_batch_operations =
            LowLevelDriveOperation::grovedb_operations_batch(drive_operations);

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
            } => drive.grove.delete_operation_for_delete_internal(
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
            drive_operations.push(GroveOperation(delete_operation))
        }

        Ok(())
    }

    pub(super) fn batch_remove_raw<B: AsRef<[u8]>>(
        drive: &Drive,
        path: SubtreePath<'_, B>,
        key: &[u8],
        apply_type: BatchDeleteApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Option<Element>, Error> {
        let mut current_batch_operations =
            LowLevelDriveOperation::grovedb_operations_batch(drive_operations);
        let options = DeleteOptions {
            backwards_references: BackwardsReferences::DontCheck,
            allow_deleting_non_empty_trees: false,
            deleting_non_empty_trees_returns_error: true,
            base_root_storage_is_free: true,
            validate_tree_at_path_exists: false,
        };

        let needs_removal_from_state =
            match current_batch_operations.remove_if_insert(path.to_vec(), key) {
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
                ) => return Ok(Some(element)),
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

        let maybe_element = drive.grove_get_raw_optional(
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
                } => drive.grove.delete_operation_for_delete_internal(
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
                drive_operations.push(GroveOperation(delete_operation))
            }
        }

        Ok(maybe_element)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn batch_move<B: AsRef<[u8]>>(
        drive: &Drive,
        from_path: SubtreePath<'_, B>,
        key: &[u8],
        to_path: Vec<Vec<u8>>,
        apply_type: BatchMoveApplyType,
        alter_flags_to_new_flags: Option<Option<StorageFlags>>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let mut element = match apply_type {
            BatchMoveApplyType::StatelessBatchMove {
                estimated_value_size,
                flags_len,
                ..
            } => {
                let value = vec![0u8; estimated_value_size as usize];
                let flags = vec![0u8; flags_len as usize];
                Element::new_item_with_flags(value, Some(flags))
            }
            BatchMoveApplyType::StatefulBatchMove { .. } => drive
                .grove_get(
                    from_path.clone(),
                    key,
                    QueryType::StatefulQuery,
                    transaction,
                    drive_operations,
                    drive_version,
                )?
                .ok_or_else(|| {
                    Error::Drive(DriveError::ElementNotFound("element to move not found"))
                })?,
        };

        if element.is_any_tree() {
            return Err(Error::Drive(DriveError::NotSupported(
                "batch_move does not support moving trees",
            )));
        }

        let current_batch = LowLevelDriveOperation::grovedb_operations_batch(drive_operations);
        let delete_opts = DeleteOptions {
            backwards_references: BackwardsReferences::DontCheck,
            allow_deleting_non_empty_trees: false,
            deleting_non_empty_trees_returns_error: true,
            base_root_storage_is_free: true,
            validate_tree_at_path_exists: false,
        };

        let delete_op = match apply_type {
            BatchMoveApplyType::StatelessBatchMove {
                in_tree_type,
                estimated_key_size,
                estimated_value_size,
                ..
            } => GroveDb::average_case_delete_operation_for_delete::<RocksDbStorage>(
                &KeyInfoPath::from_known_owned_path(from_path.to_vec()),
                &KeyInfo::KnownKey(key.to_vec()),
                in_tree_type,
                false,
                true,
                0,
                (estimated_key_size, estimated_value_size),
                BackwardsReferences::DontCheck,
                &drive_version.grove_version,
            )
            .map(|r| r.map(Some)),
            BatchMoveApplyType::StatefulBatchMove {
                is_known_to_be_subtree_with_sum,
            } => drive.grove.delete_operation_for_delete_internal(
                from_path,
                key,
                &delete_opts,
                is_known_to_be_subtree_with_sum,
                &current_batch.operations,
                transaction,
                &drive_version.grove_version,
            ),
        };

        if let Some(delete_op) = push_drive_operation_result(delete_op, drive_operations)? {
            if let Some(flags) = alter_flags_to_new_flags.as_ref() {
                element.set_flags(StorageFlags::map_to_some_element_flags(flags.as_ref()));
            }

            drive_operations.push(GroveOperation(delete_op));
            drive_operations.push(GroveOperation(
                QualifiedGroveDbOp::insert_or_replace_op(to_path, key.to_vec(), element)
                    .dont_check_for_backwards_references(),
            ));
        }

        Ok(())
    }

    fn path_query_elements(
        drive: &Drive,
        path_query: &PathQuery,
        error_if_intermediate_path_tree_not_present: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Vec<PathKeyElementTrio>, Error> {
        if path_query.query.limit.is_none() {
            return Err(Error::Drive(DriveError::NotSupported(
                "Limits are required for path_query",
            )));
        }
        Ok(
            if path_query
                .query
                .query
                .items
                .iter()
                .all(|query_item| query_item.is_key())
            {
                drive
                    .grove_get_raw_path_query_with_optional(
                        path_query,
                        error_if_intermediate_path_tree_not_present,
                        transaction,
                        drive_operations,
                        drive_version,
                    )?
                    .into_iter()
                    .filter_map(|(path, key, maybe_element)| {
                        maybe_element.map(|element| (path, key, element))
                    })
                    .collect()
            } else {
                drive
                    .grove_get_raw_path_query(
                        path_query,
                        transaction,
                        QueryResultType::QueryPathKeyElementTrioResultType,
                        drive_operations,
                        drive_version,
                    )?
                    .0
                    .to_path_key_elements()
            },
        )
    }

    pub(super) fn batch_delete_items_in_path_query(
        drive: &Drive,
        path_query: &PathQuery,
        error_if_intermediate_path_tree_not_present: bool,
        apply_type: BatchDeleteApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let query_result = path_query_elements(
            drive,
            path_query,
            error_if_intermediate_path_tree_not_present,
            transaction,
            drive_operations,
            drive_version,
        )?;

        for (path, key, _) in query_result {
            let current_batch_operations =
                LowLevelDriveOperation::grovedb_operations_batch(drive_operations);
            let options = DeleteOptions {
                backwards_references: BackwardsReferences::DontCheck,
                allow_deleting_non_empty_trees: false,
                deleting_non_empty_trees_returns_error: true,
                base_root_storage_is_free: true,
                validate_tree_at_path_exists: false,
            };
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
                } => drive.grove.delete_operation_for_delete_internal(
                    path.as_slice().into(),
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
                drive_operations.push(GroveOperation(delete_operation));
            }
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn batch_move_items_in_path_query(
        drive: &Drive,
        path_query: &PathQuery,
        new_path: Vec<Vec<u8>>,
        error_if_intermediate_path_tree_not_present: bool,
        apply_type: BatchMoveApplyType,
        alter_flags_to_new_flags: Option<Option<StorageFlags>>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let query_result = path_query_elements(
            drive,
            path_query,
            error_if_intermediate_path_tree_not_present,
            transaction,
            drive_operations,
            drive_version,
        )?;

        for (path, key, mut element) in query_result {
            let current_batch_operations =
                LowLevelDriveOperation::grovedb_operations_batch(drive_operations);
            let options = DeleteOptions {
                backwards_references: BackwardsReferences::DontCheck,
                allow_deleting_non_empty_trees: false,
                deleting_non_empty_trees_returns_error: true,
                base_root_storage_is_free: true,
                validate_tree_at_path_exists: false,
            };
            let delete_operation = match apply_type {
                BatchMoveApplyType::StatelessBatchMove {
                    in_tree_type,
                    estimated_key_size,
                    estimated_value_size,
                    ..
                } => GroveDb::average_case_delete_operation_for_delete::<RocksDbStorage>(
                    &KeyInfoPath::from_known_owned_path(path.to_vec()),
                    &KeyInfo::KnownKey(key.to_vec()),
                    in_tree_type,
                    false,
                    true,
                    0,
                    (estimated_key_size, estimated_value_size),
                    BackwardsReferences::DontCheck,
                    &drive_version.grove_version,
                )
                .map(|r| r.map(Some)),
                BatchMoveApplyType::StatefulBatchMove {
                    is_known_to_be_subtree_with_sum,
                } => drive.grove.delete_operation_for_delete_internal(
                    path.as_slice().into(),
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
                if let Some(altered_flags) = alter_flags_to_new_flags.as_ref() {
                    element.set_flags(StorageFlags::map_to_some_element_flags(
                        altered_flags.as_ref(),
                    ))
                }
                drive_operations.push(GroveOperation(delete_operation));
                drive_operations.push(GroveOperation(
                    QualifiedGroveDbOp::insert_or_replace_op(new_path.clone(), key, element)
                        .dont_check_for_backwards_references(),
                ));
            }
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn batch_delete_up_tree_while_empty(
        drive: &Drive,
        path: KeyInfoPath,
        key: &[u8],
        stop_path_height: Option<u16>,
        apply_type: BatchDeleteUpTreeApplyType,
        transaction: TransactionArg,
        check_existing_operations: &Option<&mut Vec<LowLevelDriveOperation>>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let mut current_batch_operations =
            LowLevelDriveOperation::grovedb_operations_batch(drive_operations);

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
                BackwardsReferences::DontCheck,
                &drive_version.grove_version,
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
                    backwards_references: BackwardsReferences::DontCheck,
                };
                drive
                    .grove
                    .delete_operations_for_delete_up_tree_while_empty(
                        path.to_path_refs().as_slice().into(),
                        key,
                        &options,
                        is_known_to_be_subtree_with_sum,
                        current_batch_operations.operations,
                        transaction,
                        &drive_version.grove_version,
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

fn copied_pending_grove_operations() -> usize {
    COPIED_PENDING_GROVE_OPERATIONS.with(|count| count.get())
}

fn reset_copied_pending_grove_operations() {
    COPIED_PENDING_GROVE_OPERATIONS.with(|count| count.set(0));
}

fn path(segments: &[&[u8]]) -> Vec<Vec<u8>> {
    segments.iter().map(|segment| segment.to_vec()).collect()
}

/// A drive holding
///
/// ```text
/// root
/// ├── a: tree { x: item, y: item }
/// ├── b: empty tree
/// ├── c: tree { d: tree { e: item } }
/// ├── f: tree { g: tree { h: item, i: item } }
/// ├── s: sum tree { k: sum item }
/// ├── i1: item
/// └── i2: item
/// ```
fn setup_fixture() -> Drive {
    let drive = setup_drive(None);
    let grove_version = &PlatformVersion::latest().drive.grove_version;
    let insert = |path: Vec<Vec<u8>>, key: &[u8], element: Element| {
        drive
            .grove
            .insert(path.as_slice(), key, element, None, None, grove_version)
            .unwrap()
            .expect("expected to insert the fixture");
    };
    insert(vec![], b"root", Element::empty_tree());
    insert(path(&[b"root"]), b"a", Element::empty_tree());
    insert(
        path(&[b"root", b"a"]),
        b"x",
        Element::new_item(b"x".to_vec()),
    );
    insert(
        path(&[b"root", b"a"]),
        b"y",
        Element::new_item(b"y".to_vec()),
    );
    insert(path(&[b"root"]), b"b", Element::empty_tree());
    insert(path(&[b"root"]), b"c", Element::empty_tree());
    insert(path(&[b"root", b"c"]), b"d", Element::empty_tree());
    insert(
        path(&[b"root", b"c", b"d"]),
        b"e",
        Element::new_item(b"e".to_vec()),
    );
    insert(path(&[b"root"]), b"f", Element::empty_tree());
    insert(path(&[b"root", b"f"]), b"g", Element::empty_tree());
    insert(
        path(&[b"root", b"f", b"g"]),
        b"h",
        Element::new_item(b"h".to_vec()),
    );
    insert(
        path(&[b"root", b"f", b"g"]),
        b"i",
        Element::new_item(b"i".to_vec()),
    );
    insert(path(&[b"root"]), b"s", Element::empty_sum_tree());
    insert(path(&[b"root", b"s"]), b"k", Element::new_sum_item(7));
    insert(path(&[b"root"]), b"i1", Element::new_item(b"i1".to_vec()));
    insert(path(&[b"root"]), b"i2", Element::new_item(b"i2".to_vec()));
    drive
}

fn delete(path: Vec<Vec<u8>>, key: &[u8]) -> LowLevelDriveOperation {
    GroveOperation(QualifiedGroveDbOp::delete_op(path, key.to_vec()))
}

fn insert_item(path: Vec<Vec<u8>>, key: &[u8]) -> LowLevelDriveOperation {
    GroveOperation(QualifiedGroveDbOp::insert_or_replace_op(
        path,
        key.to_vec(),
        Element::new_item(b"pending".to_vec()),
    ))
}

/// Operations GroveDB never reads for any delete in these tests, mixed with operations that are
/// not grove operations at all.
fn unrelated_operations() -> Vec<LowLevelDriveOperation> {
    let mut operations = vec![CalculatedCostOperation(OperationCost::with_seek_count(1))];
    for n in 0..20u8 {
        operations.push(insert_item(path(&[b"elsewhere"]), &[n]));
        operations.push(delete(path(&[b"elsewhere", &[n]]), b"z"));
    }
    operations.push(insert_item(path(&[b"root"]), b"new"));
    operations
}

fn stateful(is_known_to_be_subtree_with_sum: Option<MaybeTree>) -> BatchDeleteApplyType {
    BatchDeleteApplyType::StatefulBatchDelete {
        is_known_to_be_subtree_with_sum,
    }
}

fn stateless() -> BatchDeleteApplyType {
    BatchDeleteApplyType::StatelessBatchDelete {
        in_tree_type: TreeType::NormalTree,
        estimated_key_size: 1,
        estimated_value_size: 8,
    }
}

fn default_delete_options() -> DeleteOptions {
    DeleteOptions {
        backwards_references: BackwardsReferences::DontCheck,
        allow_deleting_non_empty_trees: false,
        deleting_non_empty_trees_returns_error: true,
        base_root_storage_is_free: true,
        validate_tree_at_path_exists: false,
    }
}

/// Builds `pending()` followed by what `new` adds, then `pending()` followed by what `old` adds,
/// and asserts the two agree on the result and on every operation. Returns the new operations.
fn assert_builds_as_before<T: Debug>(
    pending: impl Fn() -> Vec<LowLevelDriveOperation>,
    new: impl FnOnce(&mut Vec<LowLevelDriveOperation>) -> Result<T, Error>,
    old: impl FnOnce(&mut Vec<LowLevelDriveOperation>) -> Result<T, Error>,
) -> (Result<T, Error>, Vec<LowLevelDriveOperation>) {
    let mut new_operations = pending();
    let new_result = new(&mut new_operations);
    let mut old_operations = pending();
    let old_result = old(&mut old_operations);
    assert_eq!(format!("{new_result:?}"), format!("{old_result:?}"));
    assert_eq!(new_operations, old_operations);
    (new_result, new_operations)
}

fn last_grove_op(operations: &[LowLevelDriveOperation]) -> Option<&GroveOp> {
    operations
        .iter()
        .rev()
        .find_map(|operation| match operation {
            GroveOperation(op) | EphemeralGroveOperation(op) => Some(&op.op),
            _ => None,
        })
}

fn assert_ends_with_tree_delete(operations: &[LowLevelDriveOperation]) {
    assert!(
        matches!(
            last_grove_op(operations),
            Some(GroveOp::DeleteTreeDontCheckForBackwardsReferences(..))
        ),
        "expected a tree delete, got {:?}",
        last_grove_op(operations)
    );
}

fn assert_non_empty_tree_refused<T: Debug>(result: &Result<T, Error>) {
    assert!(
        matches!(result, Err(Error::GroveDB(e)) if matches!(e.as_ref(), grovedb::Error::DeletingNonEmptyTree(_))),
        "expected the non-empty tree to be refused, got {result:?}"
    );
}

/// Checks `batch_delete_with_options` against its old copy for `key` under `path`.
fn check_delete(
    drive: &Drive,
    pending: impl Fn() -> Vec<LowLevelDriveOperation>,
    path: Vec<Vec<u8>>,
    key: &[u8],
    apply_type: BatchDeleteApplyType,
    options: DeleteOptions,
) -> (Result<(), Error>, Vec<LowLevelDriveOperation>) {
    let drive_version = &PlatformVersion::latest().drive;
    assert_builds_as_before(
        pending,
        |operations| {
            drive.batch_delete_with_options(
                path.as_slice().into(),
                key,
                apply_type,
                options.clone(),
                None,
                operations,
                drive_version,
            )
        },
        |operations| {
            old::batch_delete_with_options(
                drive,
                path.as_slice().into(),
                key,
                apply_type,
                options.clone(),
                None,
                operations,
                drive_version,
            )
        },
    )
}

#[test]
fn should_build_item_deletes_as_before() {
    let drive = setup_fixture();
    for hint in [None, Some(MaybeTree::NotTree)] {
        let (result, operations) = check_delete(
            &drive,
            unrelated_operations,
            path(&[b"root", b"a"]),
            b"x",
            stateful(hint),
            default_delete_options(),
        );
        result.expect("expected the item delete");
        assert!(matches!(
            last_grove_op(&operations),
            Some(GroveOp::DeleteDontCheckForBackwardsReferences)
        ));
    }
    let (result, _) = check_delete(
        &drive,
        unrelated_operations,
        path(&[b"root", b"a"]),
        b"x",
        stateless(),
        default_delete_options(),
    );
    result.expect("expected the estimated item delete");
}

#[test]
fn should_build_tree_deletes_as_before() {
    let drive = setup_fixture();
    let normal_tree = Some(MaybeTree::Tree(TreeType::NormalTree));

    // An empty tree, with and without the hint.
    for hint in [None, normal_tree] {
        let (result, operations) = check_delete(
            &drive,
            unrelated_operations,
            path(&[b"root"]),
            b"b",
            stateful(hint),
            default_delete_options(),
        );
        result.expect("expected the empty tree delete");
        assert_ends_with_tree_delete(&operations);
    }

    // A tree with children and nothing pending under it.
    let (result, _) = check_delete(
        &drive,
        unrelated_operations,
        path(&[b"root"]),
        b"a",
        stateful(normal_tree),
        default_delete_options(),
    );
    assert_non_empty_tree_refused(&result);

    // A non-empty tree the caller lets through: no delete, only the cost.
    let (result, operations) = check_delete(
        &drive,
        unrelated_operations,
        path(&[b"root"]),
        b"a",
        stateful(None),
        DeleteOptions {
            deleting_non_empty_trees_returns_error: false,
            ..default_delete_options()
        },
    );
    result.expect("expected no delete and no error");
    assert!(matches!(
        operations.last(),
        Some(CalculatedCostOperation(_))
    ));

    // A sum tree.
    let (result, _) = check_delete(
        &drive,
        || vec![delete(path(&[b"root", b"s"]), b"k")],
        path(&[b"root"]),
        b"s",
        stateful(Some(MaybeTree::Tree(TreeType::SumTree))),
        default_delete_options(),
    );
    result.expect("expected the emptied sum tree delete");
}

#[test]
fn should_build_tree_deletes_that_depend_on_earlier_deletes_as_before() {
    let drive = setup_fixture();
    let normal_tree = Some(MaybeTree::Tree(TreeType::NormalTree));
    let with_unrelated = |mut operations: Vec<LowLevelDriveOperation>| {
        let mut all = unrelated_operations();
        all.append(&mut operations);
        all.append(&mut unrelated_operations());
        all
    };

    // Both children deleted earlier in the batch: the tree counts as empty.
    for hint in [None, normal_tree] {
        let (result, operations) = check_delete(
            &drive,
            || {
                with_unrelated(vec![
                    delete(path(&[b"root", b"a"]), b"x"),
                    delete(path(&[b"root", b"a"]), b"y"),
                ])
            },
            path(&[b"root"]),
            b"a",
            stateful(hint),
            default_delete_options(),
        );
        result.expect("expected the emptied tree delete");
        assert_ends_with_tree_delete(&operations);
    }

    // The same, with the child deletes in the ephemeral batch.
    let (result, operations) = check_delete(
        &drive,
        || {
            vec![
                EphemeralGroveOperation(QualifiedGroveDbOp::delete_op(
                    path(&[b"root", b"a"]),
                    b"x".to_vec(),
                )),
                EphemeralGroveOperation(QualifiedGroveDbOp::delete_op(
                    path(&[b"root", b"a"]),
                    b"y".to_vec(),
                )),
            ]
        },
        path(&[b"root"]),
        b"a",
        stateful(normal_tree),
        default_delete_options(),
    );
    result.expect("expected the emptied tree delete");
    assert_ends_with_tree_delete(&operations);

    // Only one child deleted earlier: the tree is still not empty.
    let (result, _) = check_delete(
        &drive,
        || with_unrelated(vec![delete(path(&[b"root", b"a"]), b"x")]),
        path(&[b"root"]),
        b"a",
        stateful(normal_tree),
        default_delete_options(),
    );
    assert_non_empty_tree_refused(&result);

    // A sibling deleted earlier (at the tree's own level) changes nothing.
    let (result, _) = check_delete(
        &drive,
        || {
            with_unrelated(vec![
                delete(path(&[b"root"]), b"i1"),
                delete(path(&[b"root", b"a"]), b"x"),
            ])
        },
        path(&[b"root"]),
        b"a",
        stateful(normal_tree),
        default_delete_options(),
    );
    assert_non_empty_tree_refused(&result);

    // A grandchild deleted earlier does not empty the child tree.
    let (result, _) = check_delete(
        &drive,
        || with_unrelated(vec![delete(path(&[b"root", b"c", b"d"]), b"e")]),
        path(&[b"root"]),
        b"c",
        stateful(normal_tree),
        default_delete_options(),
    );
    assert_non_empty_tree_refused(&result);

    // Something written into an empty tree earlier in the batch makes it non-empty.
    let (result, _) = check_delete(
        &drive,
        || with_unrelated(vec![insert_item(path(&[b"root", b"b"]), b"new")]),
        path(&[b"root"]),
        b"b",
        stateful(normal_tree),
        default_delete_options(),
    );
    assert_non_empty_tree_refused(&result);
}

/// Checks `batch_delete_up_tree_while_empty` against its old copy.
fn check_delete_up_tree(
    drive: &Drive,
    pending: impl Fn() -> Vec<LowLevelDriveOperation>,
    existing: Option<fn() -> Vec<LowLevelDriveOperation>>,
    path: Vec<Vec<u8>>,
    key: &[u8],
    stop_path_height: Option<u16>,
    is_known_to_be_subtree_with_sum: Option<MaybeTree>,
) -> (Result<(), Error>, Vec<LowLevelDriveOperation>) {
    let drive_version = &PlatformVersion::latest().drive;
    let mut new_existing = existing.map(|existing| existing());
    let mut old_existing = existing.map(|existing| existing());
    let apply_type = BatchDeleteUpTreeApplyType::StatefulBatchDelete {
        is_known_to_be_subtree_with_sum,
    };
    assert_builds_as_before(
        pending,
        |operations| {
            drive.batch_delete_up_tree_while_empty(
                KeyInfoPath::from_known_owned_path(path.clone()),
                key,
                stop_path_height,
                apply_type.clone(),
                None,
                &new_existing.as_mut(),
                operations,
                drive_version,
            )
        },
        |operations| {
            old::batch_delete_up_tree_while_empty(
                drive,
                KeyInfoPath::from_known_owned_path(path.clone()),
                key,
                stop_path_height,
                apply_type.clone(),
                None,
                &old_existing.as_mut(),
                operations,
                drive_version,
            )
        },
    )
}

fn tree_deletes(operations: &[LowLevelDriveOperation]) -> Vec<Vec<u8>> {
    operations
        .iter()
        .filter_map(|operation| match operation {
            GroveOperation(op)
                if matches!(
                    op.op,
                    GroveOp::DeleteTree(..)
                        | GroveOp::DeleteTreeDontCheckForBackwardsReferences(..)
                ) =>
            {
                op.key.as_ref().map(|key| key.as_slice().to_vec())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn should_build_delete_up_tree_chains_as_before() {
    let drive = setup_fixture();

    // e is d's only child and d is c's: the chain climbs until it reaches the stop height, or
    // to the root tree, which GroveDB refuses to delete.
    for hint in [None, Some(MaybeTree::NotTree)] {
        for (stop_path_height, climbs_to) in [
            (Some(3), None),
            (Some(2), Some(vec![])),
            (Some(1), Some(vec![b"d".to_vec()])),
            (Some(0), Some(vec![b"d".to_vec(), b"c".to_vec()])),
            (Some(7), None),
            (None, None),
        ] {
            let (result, operations) = check_delete_up_tree(
                &drive,
                unrelated_operations,
                None,
                path(&[b"root", b"c", b"d"]),
                b"e",
                stop_path_height,
                hint,
            );
            match climbs_to {
                Some(tree_deletes_expected) => {
                    result.expect("expected the chain up to the stop height");
                    assert_eq!(tree_deletes(&operations), tree_deletes_expected);
                }
                // At its own height the chain deletes nothing; past the root it fails.
                None if stop_path_height == Some(3) => {
                    result.expect("expected nothing to delete");
                    assert_eq!(operations, unrelated_operations());
                }
                None => assert!(result.is_err()),
            }
        }
    }

    // h has a sibling: the chain stops at g.
    let (result, operations) = check_delete_up_tree(
        &drive,
        unrelated_operations,
        None,
        path(&[b"root", b"f", b"g"]),
        b"h",
        Some(1),
        Some(MaybeTree::NotTree),
    );
    result.expect("expected the item delete alone");
    assert!(tree_deletes(&operations).is_empty());

    // The sibling deleted earlier in the batch: the chain climbs through g and f.
    let (result, operations) = check_delete_up_tree(
        &drive,
        || {
            let mut operations = unrelated_operations();
            operations.push(delete(path(&[b"root", b"f", b"g"]), b"i"));
            operations
        },
        None,
        path(&[b"root", b"f", b"g"]),
        b"h",
        Some(0),
        Some(MaybeTree::NotTree),
    );
    result.expect("expected the chain through g and f");
    assert_eq!(
        tree_deletes(&operations),
        vec![b"g".to_vec(), b"f".to_vec()]
    );

    // The sibling deleted in another operation of the same batch.
    let (result, operations) = check_delete_up_tree(
        &drive,
        unrelated_operations,
        Some(|| vec![delete(path(&[b"root", b"f", b"g"]), b"i")]),
        path(&[b"root", b"f", b"g"]),
        b"h",
        Some(0),
        Some(MaybeTree::NotTree),
    );
    result.expect("expected the chain through g and f");
    assert_eq!(
        tree_deletes(&operations),
        vec![b"g".to_vec(), b"f".to_vec()]
    );

    // Something written into g earlier in the batch stops the chain at g.
    let (result, operations) = check_delete_up_tree(
        &drive,
        || {
            vec![
                delete(path(&[b"root", b"f", b"g"]), b"i"),
                insert_item(path(&[b"root", b"f", b"g"]), b"new"),
            ]
        },
        None,
        path(&[b"root", b"f", b"g"]),
        b"h",
        Some(1),
        Some(MaybeTree::NotTree),
    );
    result.expect("expected the item delete alone");
    assert!(tree_deletes(&operations).is_empty());

    // Something written beside the emptied trees, or above the stop height, changes nothing.
    let (result, operations) = check_delete_up_tree(
        &drive,
        || {
            vec![
                insert_item(vec![], b"new"),
                insert_item(path(&[b"root"]), b"new"),
                delete(path(&[b"root", b"f", b"g"]), b"i"),
            ]
        },
        None,
        path(&[b"root", b"f", b"g"]),
        b"h",
        Some(0),
        Some(MaybeTree::NotTree),
    );
    result.expect("expected the chain through g and f");
    assert_eq!(
        tree_deletes(&operations),
        vec![b"g".to_vec(), b"f".to_vec()]
    );

    // Deleting a tree up the tree.
    let (result, operations) = check_delete_up_tree(
        &drive,
        || vec![delete(path(&[b"root", b"c", b"d"]), b"e")],
        None,
        path(&[b"root", b"c"]),
        b"d",
        Some(0),
        None,
    );
    result.expect("expected the chain through d and c");
    assert_eq!(
        tree_deletes(&operations),
        vec![b"d".to_vec(), b"c".to_vec()]
    );
}

/// Checks `batch_remove_raw` against its old copy.
fn check_remove_raw(
    drive: &Drive,
    pending: impl Fn() -> Vec<LowLevelDriveOperation>,
    path: Vec<Vec<u8>>,
    key: &[u8],
    apply_type: BatchDeleteApplyType,
) -> (Result<Option<Element>, Error>, Vec<LowLevelDriveOperation>) {
    let drive_version = &PlatformVersion::latest().drive;
    assert_builds_as_before(
        pending,
        |operations| {
            drive.batch_remove_raw(
                path.as_slice().into(),
                key,
                apply_type,
                None,
                operations,
                drive_version,
            )
        },
        |operations| {
            old::batch_remove_raw(
                drive,
                path.as_slice().into(),
                key,
                apply_type,
                None,
                operations,
                drive_version,
            )
        },
    )
}

#[test]
fn should_build_raw_removals_as_before() {
    let drive = setup_fixture();
    let root = || path(&[b"root"]);
    let pending_with = |operation: fn() -> LowLevelDriveOperation| {
        move || {
            let mut operations = unrelated_operations();
            operations.push(operation());
            operations.append(&mut unrelated_operations());
            operations
        }
    };

    for hint in [None, Some(MaybeTree::NotTree)] {
        // Nothing pending at the key: the stored element and its delete.
        let (result, operations) =
            check_remove_raw(&drive, unrelated_operations, root(), b"i1", stateful(hint));
        assert_eq!(
            result.expect("expected the removal"),
            Some(Element::new_item(b"i1".to_vec()))
        );
        assert!(matches!(
            last_grove_op(&operations),
            Some(GroveOp::DeleteDontCheckForBackwardsReferences)
        ));

        // A missing key.
        let (result, _) = check_remove_raw(
            &drive,
            unrelated_operations,
            root(),
            b"none",
            stateful(hint),
        );
        assert_eq!(result.expect("expected nothing to remove"), None);
    }

    // A pending insert, replace or patch at the key is returned as is.
    for pending in [
        pending_with(|| insert_item(path(&[b"root"]), b"i1")),
        pending_with(|| {
            GroveOperation(QualifiedGroveDbOp::replace_op(
                path(&[b"root"]),
                b"i1".to_vec(),
                Element::new_item(b"replaced".to_vec()),
            ))
        }),
    ] {
        let (result, _) = check_remove_raw(
            &drive,
            pending,
            root(),
            b"i1",
            stateful(Some(MaybeTree::NotTree)),
        );
        assert!(result.expect("expected the pending element").is_some());
    }

    // A pending delete at the key: the stored element, no second delete.
    let (result, _) = check_remove_raw(
        &drive,
        pending_with(|| delete(path(&[b"root"]), b"i1")),
        root(),
        b"i1",
        stateful(Some(MaybeTree::NotTree)),
    );
    assert!(result.expect("expected the stored element").is_some());

    // A pending insert of a new element at the key still deletes the stored one.
    let (result, _) = check_remove_raw(
        &drive,
        pending_with(|| {
            GroveOperation(
                QualifiedGroveDbOp::insert_only_known_to_not_already_exist_op(
                    path(&[b"root"]),
                    b"i1".to_vec(),
                    Element::new_item(b"new".to_vec()),
                ),
            )
        }),
        root(),
        b"i1",
        stateful(None),
    );
    assert!(result.expect("expected the stored element").is_some());

    // A tree whose children the batch deletes.
    let (result, operations) = check_remove_raw(
        &drive,
        || {
            vec![
                delete(path(&[b"root", b"a"]), b"x"),
                delete(path(&[b"root", b"a"]), b"y"),
            ]
        },
        root(),
        b"a",
        stateful(None),
    );
    assert!(result.expect("expected the stored tree").is_some());
    assert_ends_with_tree_delete(&operations);

    // A tree the batch leaves non-empty.
    let (result, _) = check_remove_raw(
        &drive,
        || vec![delete(path(&[b"root", b"a"]), b"x")],
        root(),
        b"a",
        stateful(None),
    );
    assert_non_empty_tree_refused(&result);
}

#[test]
fn should_build_moves_as_before() {
    let drive = setup_fixture();
    let drive_version = &PlatformVersion::latest().drive;
    for (key, is_known_to_be_subtree_with_sum) in [
        (b"i1".as_slice(), None),
        (b"i1".as_slice(), Some(MaybeTree::NotTree)),
        // A tree is refused before GroveDB builds anything.
        (b"a".as_slice(), Some(MaybeTree::Tree(TreeType::NormalTree))),
    ] {
        let apply_type = BatchMoveApplyType::StatefulBatchMove {
            is_known_to_be_subtree_with_sum,
        };
        let (result, _) = assert_builds_as_before(
            unrelated_operations,
            |operations| {
                drive.batch_move(
                    path(&[b"root"]).as_slice().into(),
                    key,
                    path(&[b"root", b"b"]),
                    apply_type,
                    Some(None),
                    None,
                    operations,
                    drive_version,
                )
            },
            |operations| {
                old::batch_move(
                    &drive,
                    path(&[b"root"]).as_slice().into(),
                    key,
                    path(&[b"root", b"b"]),
                    apply_type,
                    Some(None),
                    None,
                    operations,
                    drive_version,
                )
            },
        );
        assert_eq!(result.is_ok(), key == b"i1");
    }
}

fn range_query(path: Vec<Vec<u8>>) -> PathQuery {
    PathQuery::new(
        path,
        SizedQuery::new(Query::new_range_full(), Some(10), None),
    )
}

fn keys_query(path: Vec<Vec<u8>>, keys: &[&[u8]]) -> PathQuery {
    let mut query = Query::new();
    for key in keys {
        query.insert_key(key.to_vec());
    }
    PathQuery::new(path, SizedQuery::new(query, Some(10), None))
}

#[test]
fn should_build_path_query_deletes_as_before() {
    let drive = setup_fixture();
    let drive_version = &PlatformVersion::latest().drive;
    // Each case: the query, the pending batch, whether the deletes succeed, and whether the moves
    // into b do (a tree moved into b leaves b non-empty for the next move).
    type Case = (PathQuery, fn() -> Vec<LowLevelDriveOperation>, bool, bool);
    let cases: Vec<Case> = vec![
        // Items.
        (
            range_query(path(&[b"root", b"a"])),
            unrelated_operations,
            true,
            true,
        ),
        (
            keys_query(path(&[b"root", b"f", b"g"]), &[b"h", b"i", b"none"]),
            unrelated_operations,
            true,
            true,
        ),
        // Trees, one emptied earlier in the batch and one empty.
        (
            keys_query(path(&[b"root"]), &[b"a", b"b"]),
            || {
                vec![
                    delete(path(&[b"root", b"a"]), b"x"),
                    delete(path(&[b"root", b"a"]), b"y"),
                ]
            },
            true,
            false,
        ),
        // A tree left non-empty.
        (
            keys_query(path(&[b"root"]), &[b"b", b"c"]),
            unrelated_operations,
            false,
            false,
        ),
    ];
    for (path_query, pending, succeeds, moves_succeed) in cases {
        for apply_type in [stateful(None), stateless()] {
            let (result, _) = assert_builds_as_before(
                pending,
                |operations| {
                    drive.batch_delete_items_in_path_query(
                        &path_query,
                        true,
                        apply_type,
                        None,
                        operations,
                        drive_version,
                    )
                },
                |operations| {
                    old::batch_delete_items_in_path_query(
                        &drive,
                        &path_query,
                        true,
                        apply_type,
                        None,
                        operations,
                        drive_version,
                    )
                },
            );
            if matches!(apply_type, BatchDeleteApplyType::StatefulBatchDelete { .. }) {
                assert_eq!(result.is_ok(), succeeds, "{result:?}");
            }
        }

        let apply_type = BatchMoveApplyType::StatefulBatchMove {
            is_known_to_be_subtree_with_sum: None,
        };
        let flags = Some(StorageFlags::new_single_epoch(1, None));
        let (result, _) = assert_builds_as_before(
            pending,
            |operations| {
                drive.batch_move_items_in_path_query(
                    &path_query,
                    path(&[b"root", b"b"]),
                    true,
                    apply_type,
                    Some(flags.clone()),
                    None,
                    operations,
                    drive_version,
                )
            },
            |operations| {
                old::batch_move_items_in_path_query(
                    &drive,
                    &path_query,
                    path(&[b"root", b"b"]),
                    true,
                    apply_type,
                    Some(flags.clone()),
                    None,
                    operations,
                    drive_version,
                )
            },
        );
        assert_eq!(result.is_ok(), moves_succeed, "{result:?}");
    }
}

/// Building deletes into one batch copies none of the pending batch unless an operation sits
/// where GroveDB looks, while the old helpers copied every earlier operation for each delete.
#[test]
fn should_not_copy_the_pending_batch_for_each_delete() {
    const DELETES: usize = 400;
    let drive = setup_fixture();
    let drive_version = &PlatformVersion::latest().drive;
    let root = path(&[b"root"]);
    let key = |n: usize| format!("key {n}").into_bytes();

    // Item deletes the caller knows are not trees: GroveDB reads nothing.
    reset_copied_pending_grove_operations();
    let mut operations = vec![];
    for n in 0..DELETES {
        drive
            .batch_delete(
                root.as_slice().into(),
                &key(n),
                stateful(Some(MaybeTree::NotTree)),
                None,
                &mut operations,
                drive_version,
            )
            .expect("expected the item delete");
    }
    assert_eq!(copied_pending_grove_operations(), 0);

    // The same through the old helper copies every earlier delete for each new one.
    reset_copied_pending_grove_operations();
    let mut old_operations = vec![];
    for n in 0..DELETES {
        old::batch_delete(
            &drive,
            root.as_slice().into(),
            &key(n),
            stateful(Some(MaybeTree::NotTree)),
            None,
            &mut old_operations,
            drive_version,
        )
        .expect("expected the item delete");
    }
    assert_eq!(old_operations, operations);
    assert_eq!(
        copied_pending_grove_operations(),
        DELETES * (DELETES - 1) / 2
    );

    // Tree deletes, and up-tree deletes that climb through one tree each, copy nothing when
    // nothing earlier in the batch sits under the trees they read.
    let grove_version = &drive_version.grove_version;
    for n in 0..DELETES / 4 {
        let tree = format!("tree {n}").into_bytes();
        for (path, key, element) in [
            (root.clone(), tree.clone(), Element::empty_tree()),
            (
                vec![b"root".to_vec(), tree.clone()],
                b"e".to_vec(),
                Element::new_item(vec![1]),
            ),
        ] {
            drive
                .grove
                .insert(path.as_slice(), &key, element, None, None, grove_version)
                .unwrap()
                .expect("expected to insert");
        }
    }
    reset_copied_pending_grove_operations();
    for n in 0..DELETES / 4 {
        let tree = format!("tree {n}").into_bytes();
        drive
            .batch_delete(
                root.as_slice().into(),
                b"b",
                stateful(Some(MaybeTree::Tree(TreeType::NormalTree))),
                None,
                &mut operations,
                drive_version,
            )
            .expect("expected the empty tree delete");
        for stop_path_height in [Some(0), Some(1)] {
            drive
                .batch_delete_up_tree_while_empty(
                    KeyInfoPath::from_known_owned_path(vec![b"root".to_vec(), tree.clone()]),
                    b"e",
                    stop_path_height,
                    BatchDeleteUpTreeApplyType::StatefulBatchDelete {
                        is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                    },
                    None,
                    &None,
                    &mut operations,
                    drive_version,
                )
                .expect("expected the up-tree delete");
        }
    }
    assert_eq!(copied_pending_grove_operations(), 0);
}

/// The cleanup after a contested vote poll ends builds its votes, documents and contenders
/// removals as before.
mod contested_poll_cleanup {
    use super::{copied_pending_grove_operations, old, reset_copied_pending_grove_operations};
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::fees::op::LowLevelDriveOperation::GroveOperation;
    use crate::util::grove_operations::BatchDeleteApplyType;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DataContractOwnedResolvedInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::{
        CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::document::DocumentV0Setters;
    use dpp::identifier::Identifier;
    use dpp::platform_value::{Bytes32, Value};
    use dpp::prelude::TimestampMillis;
    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
    use grovedb::batch::GroveOp;
    use grovedb::MaybeTree;
    use platform_version::version::PlatformVersion;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::BTreeMap;

    const CONTENDERS: usize = 200;

    #[test]
    fn should_build_the_cleanup_of_an_ended_contested_poll_as_before() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let dpns_contract = get_dpns_data_contract_fixture(
            Some(Identifier::from([7; 32])),
            0,
            platform_version.protocol_version,
        )
        .data_contract_owned();
        drive
            .apply_contract(
                &dpns_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply the DPNS contract");
        let document_type = dpns_contract
            .document_type_for_name("domain")
            .expect("expected the domain document type");
        let vote_poll = ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(dpns_contract.clone()),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text("dash".to_string()),
                Value::Text("quantum".to_string()),
            ],
        };

        let mut rng = StdRng::seed_from_u64(433);
        let mut votes: BTreeMap<ResourceVoteChoice, Vec<Identifier>> = BTreeMap::new();
        for n in 0..CONTENDERS {
            let mut owner = [0u8; 32];
            owner[..8].copy_from_slice(&(n as u64 + 1).to_be_bytes());
            let owner_id = Identifier::from(owner);
            let mut document = document_type
                .random_document_with_params(
                    owner_id,
                    Bytes32::random_with_rng(&mut rng),
                    Some(1),
                    Some(1),
                    Some(1),
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::MinDocumentFillSize,
                    &mut rng,
                    platform_version,
                )
                .expect("expected a random domain");
            document.set("parentDomainName", "dash".into());
            document.set("normalizedParentDomainName", "dash".into());
            document.set("label", "quantum".into());
            document.set("normalizedLabel", "quantum".into());
            document.set("records.identity", owner_id.into());
            document.set("subdomainRules.allowSubdomains", false.into());
            let stored_info = (n == 0).then(|| {
                ContestedDocumentVotePollStoredInfo::new(BlockInfo::default(), platform_version)
                    .expect("expected the poll's stored info")
            });
            drive
                .add_contested_document(
                    OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(owner_id.to_buffer()),
                    },
                    vote_poll.clone(),
                    false,
                    stored_info,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add the contender");
            // A few voters for some contenders, so the votes removal deletes votes too.
            let voters = (0..n % 3)
                .map(|voter| Identifier::from([voter as u8 + 1; 32]))
                .collect();
            votes.insert(ResourceVoteChoice::TowardsIdentity(owner_id), voters);
        }
        let end_time: TimestampMillis = 1;
        let finished_polls = [(&vote_poll, &end_time, &votes)];

        reset_copied_pending_grove_operations();
        let mut operations = vec![];
        drive
            .remove_contested_resource_vote_poll_votes_operations(
                &finished_polls,
                true,
                &mut operations,
                None,
                platform_version,
            )
            .expect("expected the votes removal");
        drive
            .remove_contested_resource_vote_poll_documents_operations(
                &finished_polls,
                false,
                &mut operations,
                None,
                platform_version,
            )
            .expect("expected the documents removal");
        drive
            .remove_contested_resource_vote_poll_contenders_operations(
                &finished_polls,
                &mut operations,
                None,
                platform_version,
            )
            .expect("expected the contenders removal");
        assert_eq!(copied_pending_grove_operations(), 0);

        // Each builder passes every delete to `batch_delete` as a known non-tree, and reads
        // nothing back from the batch between calls, so passing the deletes it built through
        // the old helper, in order, rebuilds what it built before.
        let mut old_operations: Vec<LowLevelDriveOperation> = vec![];
        let mut deletes = 0;
        for operation in &operations {
            if let GroveOperation(op) = operation {
                assert!(matches!(
                    op.op,
                    GroveOp::DeleteDontCheckForBackwardsReferences
                ));
                let key = op.key.as_ref().expect("expected a keyed delete").as_slice();
                old::batch_delete(
                    &drive,
                    op.path.to_path().as_slice().into(),
                    key,
                    BatchDeleteApplyType::StatefulBatchDelete {
                        is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                    },
                    None,
                    &mut old_operations,
                    &platform_version.drive,
                )
                .expect("expected the old delete");
                deletes += 1;
            }
        }
        assert_eq!(old_operations, operations);
        let votes_cast: usize = votes.values().map(Vec::len).sum();
        // Per contender its vote tree, document, document reference and contender entry, the
        // votes cast, and the abstain and lock trees.
        assert_eq!(deletes, 4 * CONTENDERS + votes_cast + 2);
    }
}
