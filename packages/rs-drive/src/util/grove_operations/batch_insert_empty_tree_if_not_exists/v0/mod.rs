use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::PathKeyInfo;
use crate::util::object_size_info::PathKeyInfo::{
    PathFixedSizeKey, PathFixedSizeKeyRef, PathKey, PathKeyRef, PathKeySize,
};
use crate::util::storage_flags::StorageFlags;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::GroveOp;
use grovedb::TransactionArg;

impl Drive {
    /// Pushes an "insert empty tree where path key does not yet exist" operation to `drive_operations`.
    /// Will also check the current drive operations
    pub(crate) fn batch_insert_empty_tree_if_not_exists_v0<const N: usize>(
        &self,
        path_key_info: PathKeyInfo<N>,
        use_sum_tree: bool,
        storage_flags: Option<&StorageFlags>,
        apply_type: BatchInsertTreeApplyType,
        transaction: TransactionArg,
        check_existing_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        //todo: clean up the duplication
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
                            if grove_op.key == key
                                && grove_op.path == path
                                && matches!(grove_op.op, GroveOp::DeleteTree)
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
                            if grove_op.key == key
                                && grove_op.path == path
                                && matches!(grove_op.op, GroveOp::DeleteTree)
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
                            if grove_op.key == key
                                && grove_op.path == path
                                && matches!(grove_op.op, GroveOp::DeleteTree)
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
                            if grove_op.key == key
                                && grove_op.path == path
                                && matches!(grove_op.op, GroveOp::DeleteTree)
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
