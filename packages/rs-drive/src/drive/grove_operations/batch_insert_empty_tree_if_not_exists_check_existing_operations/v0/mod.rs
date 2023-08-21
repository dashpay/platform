use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::BatchInsertTreeApplyType;
use crate::drive::object_size_info::PathKeyInfo;
use crate::drive::object_size_info::PathKeyInfo::{
    PathFixedSizeKey, PathFixedSizeKeyRef, PathKey, PathKeyRef, PathKeySize,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Pushes an "insert empty tree where path key does not yet exist" operation to `drive_operations`.
    /// Will also check the current drive operations
    pub(super) fn batch_insert_empty_tree_if_not_exists_check_existing_operations_v0<
        const N: usize,
    >(
        &self,
        path_key_info: PathKeyInfo<N>,
        storage_flags: Option<&StorageFlags>,
        apply_type: BatchInsertTreeApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match path_key_info {
            PathKeyRef((path, key)) => {
                let drive_operation = LowLevelDriveOperation::for_known_path_key_empty_tree(
                    path.clone(),
                    key.to_vec(),
                    storage_flags,
                );
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
                let drive_operation = LowLevelDriveOperation::for_known_path_key_empty_tree(
                    path.clone(),
                    key.clone(),
                    storage_flags,
                );
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
                let drive_operation = LowLevelDriveOperation::for_known_path_key_empty_tree(
                    path_items,
                    key.to_vec(),
                    storage_flags,
                );
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
                let drive_operation = LowLevelDriveOperation::for_known_path_key_empty_tree(
                    path_items,
                    key.to_vec(),
                    storage_flags,
                );
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
