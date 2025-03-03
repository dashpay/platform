use crate::util::grove_operations::BatchInsertApplyType;
use crate::util::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyRefElement, PathKeyElement, PathKeyElementSize, PathKeyRefElement,
    PathKeyUnknownElementSize,
};

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
use crate::util::object_size_info::PathKeyElementInfo;
use dpp::version::drive_versions::DriveVersion;
use grovedb::{Element, GroveDb, TransactionArg};

impl Drive {
    /// Inserts a sum item at the specified path and key if it doesn't already exist.
    /// If a sum item exists at the specified location and `error_if_exists` is true, an error is returned.
    /// If a sum item exists and `error_if_exists` is false, no changes are made.
    /// If no sum item exists, a new sum item is inserted at the specified path and key.
    ///
    /// # Parameters
    /// * `path_key_element_info`: Contains information about the path, key, and element to be processed.
    /// * `error_if_exists`: A flag that determines whether to return an error if the sum item already exists.
    /// * `apply_type`: Defines the type of batch insert to be performed (stateful or stateless).
    /// * `transaction`: The transaction argument for the operation.
    /// * `drive_operations`: A mutable reference to a vector of low-level drive operations to which new operations will be appended.
    /// * `drive_version`: The version of the drive to ensure compatibility with the operation.
    ///
    /// # Returns
    /// * `Ok(())` if the operation is successful.
    /// * `Err(Error)` if the operation fails for any reason, such as corrupted state or unsupported operation.
    ///
    /// # Description
    /// This function checks whether an existing sum item exists at the given path and key:
    /// - If a sum item is found and `error_if_exists` is true, an error is returned.
    /// - If a sum item is found and `error_if_exists` is false, no changes are made.
    /// - If no sum item exists, a new sum item is inserted at the specified path and key.
    ///
    /// This function supports several types of paths and keys, including:
    /// - `PathKeyRefElement`: A path with a reference to a key and element.
    /// - `PathKeyElement`: A path with a direct key and element.
    /// - `PathFixedSizeKeyRefElement`: A fixed-size key reference.
    /// - `PathKeyElementSize`: An element with an associated size.
    /// - `PathKeyUnknownElementSize`: An unknown element size type.
    ///
    /// Depending on the element type (`SumItem` in this case), the appropriate operations will be applied.
    ///
    /// **Note**: Stateful batch insertions of document sizes are not supported.
    pub(super) fn batch_insert_sum_item_if_not_exists_v0<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        error_if_exists: bool,
        apply_type: BatchInsertApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match path_key_element_info {
            PathKeyRefElement((path, key, element)) => {
                if let Element::SumItem(new_value, _) = element {
                    // Check if the sum item already exists
                    let existing_element = self.grove_get_raw_optional(
                        path.as_slice().into(),
                        key,
                        apply_type.to_direct_query_type(),
                        transaction,
                        drive_operations,
                        drive_version,
                    )?;

                    if let Some(Element::SumItem(..)) = existing_element {
                        return if error_if_exists {
                            Err(Error::Drive(DriveError::CorruptedDriveState(
                                "expected no sum item".to_string(),
                            )))
                        } else {
                            Ok(false)
                        };
                        // Else do nothing
                    } else if existing_element.is_some() {
                        return Err(Error::Drive(DriveError::CorruptedElementType(
                            "expected sum item element type",
                        )));
                    } else {
                        // Insert as a new sum item
                        drive_operations.push(
                            LowLevelDriveOperation::insert_for_known_path_key_element(
                                path,
                                key.to_vec(),
                                Element::new_sum_item(new_value),
                            ),
                        );
                    }
                } else {
                    return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "expected sum item element type",
                    )));
                }
                Ok(true)
            }
            PathKeyElement((path, key, element)) => {
                if let Element::SumItem(new_value, _) = element {
                    // Check if the sum item already exists
                    let existing_element = self.grove_get_raw_optional(
                        path.as_slice().into(),
                        key.as_slice(),
                        apply_type.to_direct_query_type(),
                        transaction,
                        drive_operations,
                        drive_version,
                    )?;

                    if let Some(Element::SumItem(..)) = existing_element {
                        return if error_if_exists {
                            Err(Error::Drive(DriveError::CorruptedDriveState(
                                "expected no sum item".to_string(),
                            )))
                        } else {
                            Ok(false)
                        };
                        // Else do nothing
                    } else if existing_element.is_some() {
                        return Err(Error::Drive(DriveError::CorruptedElementType(
                            "expected sum item element type",
                        )));
                    } else {
                        // Insert as a new sum item
                        drive_operations.push(
                            LowLevelDriveOperation::insert_for_known_path_key_element(
                                path,
                                key,
                                Element::new_sum_item(new_value),
                            ),
                        );
                    }
                } else {
                    return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "expected sum item element type",
                    )));
                }
                Ok(true)
            }
            PathFixedSizeKeyRefElement((path, key, element)) => {
                if let Element::SumItem(new_value, _) = element {
                    // Check if the sum item already exists
                    let existing_element = self.grove_get_raw_optional(
                        path.as_slice().into(),
                        key,
                        apply_type.to_direct_query_type(),
                        transaction,
                        drive_operations,
                        drive_version,
                    )?;

                    if let Some(Element::SumItem(..)) = existing_element {
                        return if error_if_exists {
                            Err(Error::Drive(DriveError::CorruptedDriveState(
                                "expected no sum item".to_string(),
                            )))
                        } else {
                            Ok(false)
                        };
                        // Else do nothing
                    } else if existing_element.is_some() {
                        return Err(Error::Drive(DriveError::CorruptedElementType(
                            "expected sum item element type",
                        )));
                    } else {
                        // Insert as a new sum item
                        let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                        drive_operations.push(
                            LowLevelDriveOperation::insert_for_known_path_key_element(
                                path_items,
                                key.to_vec(),
                                Element::new_sum_item(new_value),
                            ),
                        );
                    }
                } else {
                    return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "expected sum item element type",
                    )));
                }
                Ok(true)
            }
            PathKeyElementSize((key_info_path, key_info, element)) => {
                if let Element::SumItem(new_value, _) = element {
                    match apply_type {
                        BatchInsertApplyType::StatelessBatchInsert {
                            in_tree_type, ..
                        } => {
                            // Estimate if the sum item with the given size already exists
                            drive_operations.push(CalculatedCostOperation(
                                GroveDb::average_case_for_has_raw(
                                    &key_info_path,
                                    &key_info,
                                    element.serialized_size(&drive_version.grove_version)? as u32,
                                    in_tree_type,
                                    &drive_version.grove_version,
                                )?,
                            ));

                            drive_operations.push(
                                LowLevelDriveOperation::insert_for_estimated_path_key_element(
                                    key_info_path,
                                    key_info,
                                    Element::new_sum_item(new_value),
                                ),
                            );
                            Ok(true)
                        }
                        BatchInsertApplyType::StatefulBatchInsert => {
                            Err(Error::Drive(DriveError::NotSupportedPrivate(
                                "document sizes for stateful insert in batch operations not supported",
                            )))
                        }
                    }
                } else {
                    Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "expected sum item element type",
                    )))
                }
            }
            PathKeyUnknownElementSize(_) => Err(Error::Drive(DriveError::NotSupportedPrivate(
                "document sizes in batch operations not supported",
            ))),
        }
    }
}
