use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::fees::op::{LowLevelDriveOperation, LowLevelDriveOperationTreeTypeConverter};
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::PathKeyInfo;
use crate::util::object_size_info::PathKeyInfo::{
    PathFixedSizeKey, PathFixedSizeKeyRef, PathKey, PathKeyRef, PathKeySize,
};
use crate::util::storage_flags::StorageFlags;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::GroveOp;
use grovedb::{TransactionArg, TreeType};

impl Drive {
    /// Pushes an "insert empty tree where path key does not yet exist" operation to `drive_operations`.
    /// Will also check the current drive operations
    #[allow(clippy::too_many_arguments)]
    pub(super) fn batch_insert_empty_tree_if_not_exists_v0<const N: usize>(
        &self,
        path_key_info: PathKeyInfo<N>,
        tree_type: TreeType,
        wrap_in_non_aggregated_for_parent_tree_type: Option<TreeType>,
        storage_flags: Option<&StorageFlags>,
        apply_type: BatchInsertTreeApplyType,
        transaction: TransactionArg,
        check_existing_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        // The index walker passes the parent value tree's TreeType when
        // the parent aggregates count, sum, or both. The
        // `wrap_in_non_aggregated_for_parent_tree_type` dispatcher
        // then picks the right wrapper variant
        // (NonCounted / NotSummed / NotCountedOrSummed) based on what
        // axes the parent aggregates. For non-aggregating parents
        // (`wrap_in_non_aggregated_for_parent_tree_type: None`), no wrapping is
        // needed and we fall through to the plain empty-tree op.
        let build_op =
            |path: Vec<Vec<u8>>, key: Vec<u8>| -> Result<LowLevelDriveOperation, Error> {
                if let Some(parent_tt) = wrap_in_non_aggregated_for_parent_tree_type {
                    LowLevelDriveOperation::wrap_in_non_aggregated_for_parent_tree_type(
                        path,
                        key,
                        parent_tt,
                        tree_type,
                        storage_flags,
                    )
                } else {
                    tree_type.empty_tree_operation_for_known_path_key(path, key, storage_flags)
                }
            };
        //todo: clean up the duplication
        match path_key_info {
            PathKeyRef((path, key)) => {
                let drive_operation = build_op(path.clone(), key.to_vec())?;
                // we only add the operation if it doesn't already exist in the current batch
                if let Some(existing_operations) = check_existing_operations {
                    let mut i = 0;
                    let mut found = false;
                    while i < existing_operations.len() {
                        // we need to check every drive operation
                        // if it already exists then just ignore things
                        // if we had a delete then we need to remove the delete
                        let previous_drive_operation = &existing_operations[i];
                        if previous_drive_operation == &drive_operation {
                            found = true;
                            break;
                        } else if let GroveOperation(grove_op) = previous_drive_operation {
                            if grove_op.key == Some(KeyInfo::KnownKey(key.to_vec()))
                                && grove_op.path == path
                                && matches!(grove_op.op, GroveOp::DeleteTree(_, _))
                            {
                                found = true;
                                existing_operations.remove(i);
                                break;
                            } else {
                                i += 1;
                            }
                        } else {
                            i += 1;
                        }
                    }
                    if !found {
                        let has_raw = self.grove_has_raw(
                            path.as_slice().into(),
                            key,
                            apply_type.to_direct_query_type(),
                            transaction,
                            drive_operations,
                            drive_version,
                        )?;
                        if !has_raw {
                            drive_operations.push(drive_operation);
                        }
                        Ok(!has_raw)
                    } else {
                        Ok(false)
                    }
                } else {
                    let has_raw = self.grove_has_raw(
                        path.as_slice().into(),
                        key,
                        apply_type.to_direct_query_type(),
                        transaction,
                        drive_operations,
                        drive_version,
                    )?;
                    if !has_raw {
                        drive_operations.push(drive_operation);
                    }
                    Ok(!has_raw)
                }
            }
            PathKeySize(_key_path_info, _key_info) => Err(Error::Drive(
                DriveError::NotSupportedPrivate("document sizes in batch operations not supported"),
            )),
            PathKey((path, key)) => {
                let drive_operation = build_op(path.clone(), key.to_vec())?;
                // we only add the operation if it doesn't already exist in the current batch
                if let Some(existing_operations) = check_existing_operations {
                    let mut i = 0;
                    let mut found = false;
                    while i < existing_operations.len() {
                        // we need to check every drive operation
                        // if it already exists then just ignore things
                        // if we had a delete then we need to remove the delete
                        let previous_drive_operation = &existing_operations[i];
                        if previous_drive_operation == &drive_operation {
                            found = true;
                            break;
                        } else if let GroveOperation(grove_op) = previous_drive_operation {
                            if grove_op.key == Some(KeyInfo::KnownKey(key.to_vec()))
                                && grove_op.path == path
                                && matches!(grove_op.op, GroveOp::DeleteTree(_, _))
                            {
                                found = true;
                                existing_operations.remove(i);
                                break;
                            } else {
                                i += 1;
                            }
                        } else {
                            i += 1;
                        }
                    }
                    if !found {
                        let has_raw = self.grove_has_raw(
                            path.as_slice().into(),
                            key.as_slice(),
                            apply_type.to_direct_query_type(),
                            transaction,
                            drive_operations,
                            drive_version,
                        )?;
                        if !has_raw {
                            drive_operations.push(drive_operation);
                        }
                        Ok(!has_raw)
                    } else {
                        Ok(false)
                    }
                } else {
                    let has_raw = self.grove_has_raw(
                        path.as_slice().into(),
                        key.as_slice(),
                        apply_type.to_direct_query_type(),
                        transaction,
                        drive_operations,
                        drive_version,
                    )?;
                    if !has_raw {
                        drive_operations.push(drive_operation);
                    }
                    Ok(!has_raw)
                }
            }
            PathFixedSizeKey((path, key)) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                let drive_operation = build_op(path_items, key.to_vec())?;
                // we only add the operation if it doesn't already exist in the current batch
                if let Some(existing_operations) = check_existing_operations {
                    let mut i = 0;
                    let mut found = false;
                    while i < existing_operations.len() {
                        // we need to check every drive operation
                        // if it already exists then just ignore things
                        // if we had a delete then we need to remove the delete
                        let previous_drive_operation = &existing_operations[i];
                        if previous_drive_operation == &drive_operation {
                            found = true;
                            break;
                        } else if let GroveOperation(grove_op) = previous_drive_operation {
                            if grove_op.key == Some(KeyInfo::KnownKey(key.to_vec()))
                                && grove_op.path == path
                                && matches!(grove_op.op, GroveOp::DeleteTree(_, _))
                            {
                                found = true;
                                existing_operations.remove(i);
                                break;
                            } else {
                                i += 1;
                            }
                        } else {
                            i += 1;
                        }
                    }
                    if !found {
                        let has_raw = self.grove_has_raw(
                            path.as_ref().into(),
                            key.as_slice(),
                            apply_type.to_direct_query_type(),
                            transaction,
                            drive_operations,
                            drive_version,
                        )?;
                        if !has_raw {
                            drive_operations.push(drive_operation);
                        }
                        Ok(!has_raw)
                    } else {
                        Ok(false)
                    }
                } else {
                    let has_raw = self.grove_has_raw(
                        path.as_ref().into(),
                        key.as_slice(),
                        apply_type.to_direct_query_type(),
                        transaction,
                        drive_operations,
                        drive_version,
                    )?;
                    if !has_raw {
                        drive_operations.push(drive_operation);
                    }
                    Ok(!has_raw)
                }
            }
            PathFixedSizeKeyRef((path, key)) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                let drive_operation = build_op(path_items, key.to_vec())?;
                // we only add the operation if it doesn't already exist in the current batch
                if let Some(existing_operations) = check_existing_operations {
                    let mut i = 0;
                    let mut found = false;
                    while i < existing_operations.len() {
                        // we need to check every drive operation
                        // if it already exists then just ignore things
                        // if we had a delete then we need to remove the delete
                        let previous_drive_operation = &existing_operations[i];
                        if previous_drive_operation == &drive_operation {
                            found = true;
                            break;
                        } else if let GroveOperation(grove_op) = previous_drive_operation {
                            if grove_op.key == Some(KeyInfo::KnownKey(key.to_vec()))
                                && grove_op.path == path
                                && matches!(grove_op.op, GroveOp::DeleteTree(_, _))
                            {
                                found = true;
                                existing_operations.remove(i);
                                break;
                            } else {
                                i += 1;
                            }
                        } else {
                            i += 1;
                        }
                    }
                    if !found {
                        let has_raw = self.grove_has_raw(
                            path.as_ref().into(),
                            key,
                            apply_type.to_direct_query_type(),
                            transaction,
                            drive_operations,
                            drive_version,
                        )?;
                        if !has_raw {
                            drive_operations.push(drive_operation);
                        }
                        Ok(!has_raw)
                    } else {
                        Ok(false)
                    }
                } else {
                    let has_raw = self.grove_has_raw(
                        path.as_ref().into(),
                        key,
                        apply_type.to_direct_query_type(),
                        transaction,
                        drive_operations,
                        drive_version,
                    )?;
                    if !has_raw {
                        drive_operations.push(drive_operation);
                    }
                    Ok(!has_raw)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::grove_operations::BatchInsertTreeApplyType;
    use crate::util::object_size_info::PathKeyInfo;
    use crate::util::test_helpers::setup::setup_drive;
    use grovedb::TreeType;
    use grovedb_path::SubtreePath;
    use platform_version::version::PlatformVersion;

    /// PathKeyRef variant, no existing operations, tree doesn't exist -> inserts.
    #[test]
    fn test_batch_insert_empty_tree_if_not_exists_path_key_ref_new() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::NormalTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        let mut ops = vec![];
        let info = PathKeyInfo::<0>::PathKeyRef((vec![b"root".to_vec()], b"child"));

        let inserted = drive
            .batch_insert_empty_tree_if_not_exists_v0(
                info,
                TreeType::NormalTree,
                None,
                None,
                BatchInsertTreeApplyType::StatefulBatchInsertTree,
                Some(&tx),
                &mut None,
                &mut ops,
                &pv.drive,
            )
            .expect("expected operation to succeed");

        assert!(inserted);
    }

    /// PathKeyRef variant, tree already exists -> returns false.
    #[test]
    fn test_batch_insert_empty_tree_if_not_exists_path_key_ref_exists() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::NormalTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        drive
            .grove_insert_empty_tree(
                [b"root".as_slice()].as_slice().into(),
                b"child",
                TreeType::NormalTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        let mut ops = vec![];
        let info = PathKeyInfo::<0>::PathKeyRef((vec![b"root".to_vec()], b"child"));

        let inserted = drive
            .batch_insert_empty_tree_if_not_exists_v0(
                info,
                TreeType::NormalTree,
                None,
                None,
                BatchInsertTreeApplyType::StatefulBatchInsertTree,
                Some(&tx),
                &mut None,
                &mut ops,
                &pv.drive,
            )
            .expect("expected operation to succeed");

        assert!(!inserted);
    }

    /// PathKeyRef with check_existing_operations where element is not found in ops
    /// but also doesn't exist in grove -> inserts via check_existing_operations path.
    #[test]
    fn test_batch_insert_empty_tree_if_not_exists_with_check_ops_new() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::NormalTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        let mut ops = vec![];
        let mut existing_ops = vec![];
        let info = PathKeyInfo::<0>::PathKeyRef((vec![b"root".to_vec()], b"child"));

        let inserted = drive
            .batch_insert_empty_tree_if_not_exists_v0(
                info,
                TreeType::NormalTree,
                None,
                None,
                BatchInsertTreeApplyType::StatefulBatchInsertTree,
                Some(&tx),
                &mut Some(&mut existing_ops),
                &mut ops,
                &pv.drive,
            )
            .expect("expected operation to succeed");

        assert!(inserted);
    }

    /// PathKeySize returns error.
    #[test]
    fn test_batch_insert_empty_tree_if_not_exists_path_key_size_error() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();

        let mut ops = vec![];
        let key_info_path =
            grovedb::batch::KeyInfoPath::from_known_owned_path(vec![b"root".to_vec()]);
        let key_info = grovedb::batch::key_info::KeyInfo::KnownKey(b"child".to_vec());
        let info = PathKeyInfo::<0>::PathKeySize(key_info_path, key_info);

        let result = drive.batch_insert_empty_tree_if_not_exists_v0(
            info,
            TreeType::NormalTree,
            None,
            None,
            BatchInsertTreeApplyType::StatefulBatchInsertTree,
            None,
            &mut None,
            &mut ops,
            &pv.drive,
        );

        assert!(result.is_err());
    }

    /// PathKey variant, no check_existing_operations, tree doesn't exist -> inserts.
    #[test]
    fn test_batch_insert_empty_tree_if_not_exists_path_key_new() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::NormalTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        let mut ops = vec![];
        let info = PathKeyInfo::<0>::PathKey((vec![b"root".to_vec()], b"child".to_vec()));

        let inserted = drive
            .batch_insert_empty_tree_if_not_exists_v0(
                info,
                TreeType::NormalTree,
                None,
                None,
                BatchInsertTreeApplyType::StatefulBatchInsertTree,
                Some(&tx),
                &mut None,
                &mut ops,
                &pv.drive,
            )
            .expect("expected operation to succeed");

        assert!(inserted);
    }

    /// PathFixedSizeKey variant, tree doesn't exist -> inserts.
    #[test]
    fn test_batch_insert_empty_tree_if_not_exists_fixed_size_key_new() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::NormalTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        let mut ops = vec![];
        let path: [&[u8]; 1] = [b"root"];
        let info = PathKeyInfo::PathFixedSizeKey((path, b"child".to_vec()));

        let inserted = drive
            .batch_insert_empty_tree_if_not_exists_v0(
                info,
                TreeType::NormalTree,
                None,
                None,
                BatchInsertTreeApplyType::StatefulBatchInsertTree,
                Some(&tx),
                &mut None,
                &mut ops,
                &pv.drive,
            )
            .expect("expected operation to succeed");

        assert!(inserted);
    }

    /// PathFixedSizeKeyRef variant, tree doesn't exist -> inserts.
    #[test]
    fn test_batch_insert_empty_tree_if_not_exists_fixed_size_key_ref_new() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::NormalTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        let mut ops = vec![];
        let path: [&[u8]; 1] = [b"root"];
        let info = PathKeyInfo::PathFixedSizeKeyRef((path, b"child"));

        let inserted = drive
            .batch_insert_empty_tree_if_not_exists_v0(
                info,
                TreeType::NormalTree,
                None,
                None,
                BatchInsertTreeApplyType::StatefulBatchInsertTree,
                Some(&tx),
                &mut None,
                &mut ops,
                &pv.drive,
            )
            .expect("expected operation to succeed");

        assert!(inserted);
    }
}
