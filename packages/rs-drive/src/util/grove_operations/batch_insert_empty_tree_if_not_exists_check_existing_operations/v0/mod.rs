use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::PathKeyInfo;
use crate::util::object_size_info::PathKeyInfo::{
    PathFixedSizeKey, PathFixedSizeKeyRef, PathKey, PathKeyRef, PathKeySize,
};
use crate::util::storage_flags::StorageFlags;

use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Pushes an "insert empty tree where path key does not yet exist" operation to `drive_operations`.
    /// Will also check the current drive operations
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn batch_insert_empty_tree_if_not_exists_check_existing_operations_v0<
        const N: usize,
    >(
        &self,
        path_key_info: PathKeyInfo<N>,
        use_sum_tree: bool,
        storage_flags: Option<&StorageFlags>,
        apply_type: BatchInsertTreeApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match path_key_info {
            PathKeyRef((path, key)) => {
                let drive_operation = if use_sum_tree {
                    LowLevelDriveOperation::for_known_path_key_empty_sum_tree(
                        path.clone(),
                        key.to_vec(),
                        storage_flags,
                    )
                } else {
                    LowLevelDriveOperation::for_known_path_key_empty_tree(
                        path.clone(),
                        key.to_vec(),
                        storage_flags,
                    )
                };
                // we only add the operation if it doesn't already exist in the current batch
                if !drive_operations.contains(&drive_operation) {
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
            }
            PathKeySize(_key_path_info, _key_info) => Err(Error::Drive(
                DriveError::NotSupportedPrivate("document sizes in batch operations not supported"),
            )),
            PathKey((path, key)) => {
                let drive_operation = if use_sum_tree {
                    LowLevelDriveOperation::for_known_path_key_empty_sum_tree(
                        path.clone(),
                        key.to_vec(),
                        storage_flags,
                    )
                } else {
                    LowLevelDriveOperation::for_known_path_key_empty_tree(
                        path.clone(),
                        key.to_vec(),
                        storage_flags,
                    )
                };
                // we only add the operation if it doesn't already exist in the current batch
                if !drive_operations.contains(&drive_operation) {
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
            }
            PathFixedSizeKey((path, key)) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                let drive_operation = if use_sum_tree {
                    LowLevelDriveOperation::for_known_path_key_empty_sum_tree(
                        path_items,
                        key.to_vec(),
                        storage_flags,
                    )
                } else {
                    LowLevelDriveOperation::for_known_path_key_empty_tree(
                        path_items,
                        key.to_vec(),
                        storage_flags,
                    )
                };
                // we only add the operation if it doesn't already exist in the current batch
                if !drive_operations.contains(&drive_operation) {
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
            }
            PathFixedSizeKeyRef((path, key)) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                let drive_operation = if use_sum_tree {
                    LowLevelDriveOperation::for_known_path_key_empty_sum_tree(
                        path_items,
                        key.to_vec(),
                        storage_flags,
                    )
                } else {
                    LowLevelDriveOperation::for_known_path_key_empty_tree(
                        path_items,
                        key.to_vec(),
                        storage_flags,
                    )
                };
                // we only add the operation if it doesn't already exist in the current batch
                if !drive_operations.contains(&drive_operation) {
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

    /// Insert empty tree via PathKeyRef when it doesn't exist.
    #[test]
    fn test_check_existing_ops_path_key_ref_new() {
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
            .unwrap();

        let mut ops = vec![];
        let info = PathKeyInfo::<0>::PathKeyRef((vec![b"root".to_vec()], b"child"));

        let inserted = drive
            .batch_insert_empty_tree_if_not_exists_check_existing_operations_v0(
                info,
                false,
                None,
                BatchInsertTreeApplyType::StatefulBatchInsertTree,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .unwrap();

        assert!(inserted);
    }

    /// Insert duplicate in existing operations returns false.
    #[test]
    fn test_check_existing_ops_path_key_ref_duplicate() {
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
            .unwrap();

        let mut ops = vec![];
        let info = PathKeyInfo::<0>::PathKeyRef((vec![b"root".to_vec()], b"child"));

        // First insert - should succeed
        drive
            .batch_insert_empty_tree_if_not_exists_check_existing_operations_v0(
                info,
                false,
                None,
                BatchInsertTreeApplyType::StatefulBatchInsertTree,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .unwrap();

        // Second insert with same key - should return false
        let info2 = PathKeyInfo::<0>::PathKeyRef((vec![b"root".to_vec()], b"child"));
        let inserted = drive
            .batch_insert_empty_tree_if_not_exists_check_existing_operations_v0(
                info2,
                false,
                None,
                BatchInsertTreeApplyType::StatefulBatchInsertTree,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .unwrap();

        assert!(!inserted);
    }

    /// PathKeySize returns error.
    #[test]
    fn test_check_existing_ops_path_key_size_error() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();

        let mut ops = vec![];
        let key_info_path =
            grovedb::batch::KeyInfoPath::from_known_owned_path(vec![b"root".to_vec()]);
        let key_info = grovedb::batch::key_info::KeyInfo::KnownKey(b"child".to_vec());
        let info = PathKeyInfo::<0>::PathKeySize(key_info_path, key_info);

        let result = drive.batch_insert_empty_tree_if_not_exists_check_existing_operations_v0(
            info,
            false,
            None,
            BatchInsertTreeApplyType::StatefulBatchInsertTree,
            None,
            &mut ops,
            &pv.drive,
        );

        assert!(result.is_err());
    }

    /// PathKey variant new insert.
    #[test]
    fn test_check_existing_ops_path_key_new() {
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
            .unwrap();

        let mut ops = vec![];
        let info = PathKeyInfo::<0>::PathKey((vec![b"root".to_vec()], b"child".to_vec()));

        let inserted = drive
            .batch_insert_empty_tree_if_not_exists_check_existing_operations_v0(
                info,
                false,
                None,
                BatchInsertTreeApplyType::StatefulBatchInsertTree,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .unwrap();

        assert!(inserted);
    }

    /// PathFixedSizeKey variant new insert.
    #[test]
    fn test_check_existing_ops_fixed_size_key_new() {
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
            .unwrap();

        let mut ops = vec![];
        let path: [&[u8]; 1] = [b"root"];
        let info = PathKeyInfo::PathFixedSizeKey((path, b"child".to_vec()));

        let inserted = drive
            .batch_insert_empty_tree_if_not_exists_check_existing_operations_v0(
                info,
                false,
                None,
                BatchInsertTreeApplyType::StatefulBatchInsertTree,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .unwrap();

        assert!(inserted);
    }

    /// PathFixedSizeKeyRef variant new insert.
    #[test]
    fn test_check_existing_ops_fixed_size_key_ref_new() {
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
            .unwrap();

        let mut ops = vec![];
        let path: [&[u8]; 1] = [b"root"];
        let info = PathKeyInfo::PathFixedSizeKeyRef((path, b"child"));

        let inserted = drive
            .batch_insert_empty_tree_if_not_exists_check_existing_operations_v0(
                info,
                false,
                None,
                BatchInsertTreeApplyType::StatefulBatchInsertTree,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .unwrap();

        assert!(inserted);
    }

    /// Insert with sum_tree=true uses sum tree operations.
    #[test]
    fn test_check_existing_ops_sum_tree() {
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
            .unwrap();

        let mut ops = vec![];
        let info = PathKeyInfo::<0>::PathKeyRef((vec![b"root".to_vec()], b"child"));

        let inserted = drive
            .batch_insert_empty_tree_if_not_exists_check_existing_operations_v0(
                info,
                true, // use_sum_tree = true
                None,
                BatchInsertTreeApplyType::StatefulBatchInsertTree,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .unwrap();

        assert!(inserted);
    }
}
