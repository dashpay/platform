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
use dpp::ProtocolError;
use grovedb::{Element, GroveDb, TransactionArg};

impl Drive {
    /// Version 0 implementation of the "insert sum item or add to it if the item already exists" operation.
    /// This operation either inserts a new sum item at the given path and key or adds the value to the existing sum item.
    ///
    /// # Parameters
    /// * `path_key_element_info`: Information about the path, key, and element.
    /// * `apply_type`: The apply type for the operation.
    /// * `transaction`: The transaction argument for the operation.
    /// * `drive_operations`: The list of drive operations to append to.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::CorruptedCodeExecution)` if the operation is not supported.
    pub(crate) fn batch_insert_sum_item_or_add_to_if_already_exists_v0<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        apply_type: BatchInsertApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
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

                    if let Some(Element::SumItem(existing_value, _)) = existing_element {
                        // Add to the existing sum item
                        let updated_value = existing_value
                            .checked_add(new_value)
                            .ok_or(ProtocolError::Overflow("overflow when adding to sum item"))?;
                        drive_operations.push(
                            LowLevelDriveOperation::insert_for_known_path_key_element(
                                path,
                                key.to_vec(),
                                Element::new_sum_item(updated_value),
                            ),
                        );
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
                Ok(())
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

                    if let Some(Element::SumItem(existing_value, _)) = existing_element {
                        // Add to the existing sum item
                        let updated_value = existing_value
                            .checked_add(new_value)
                            .ok_or(ProtocolError::Overflow("overflow when adding to sum item"))?;
                        drive_operations.push(
                            LowLevelDriveOperation::insert_for_known_path_key_element(
                                path,
                                key,
                                Element::new_sum_item(updated_value),
                            ),
                        );
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
                Ok(())
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

                    if let Some(Element::SumItem(existing_value, _)) = existing_element {
                        // Add to the existing sum item
                        let updated_value = existing_value
                            .checked_add(new_value)
                            .ok_or(ProtocolError::Overflow("overflow when adding to sum item"))?;
                        let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                        drive_operations.push(
                            LowLevelDriveOperation::insert_for_known_path_key_element(
                                path_items,
                                key.to_vec(),
                                Element::new_sum_item(updated_value),
                            ),
                        );
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
                Ok(())
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
                            Ok(())
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
