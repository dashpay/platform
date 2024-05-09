use crate::drive::grove_operations::BatchInsertApplyType;
use crate::drive::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyRefElement, PathKeyElement, PathKeyElementSize, PathKeyRefElement,
    PathKeyUnknownElementSize,
};

use crate::drive::object_size_info::PathKeyElementInfo;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::op::LowLevelDriveOperation::CalculatedCostOperation;
use dpp::version::drive_versions::DriveVersion;
use grovedb::{Element, GroveDb, TransactionArg};

impl Drive {
    /// Pushes an "insert element if element was changed or is new" operation to `drive_operations`.
    /// Returns true if the path key already exists without references.
    pub(super) fn batch_insert_if_changed_value_v0<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        apply_type: BatchInsertApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(bool, Option<Element>), Error> {
        match path_key_element_info {
            PathKeyRefElement((path, key, element)) => {
                let previous_element = self.grove_get_raw_optional(
                    path.as_slice().into(),
                    key,
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                let needs_insert = match &previous_element {
                    None => true,
                    Some(previous_element) => previous_element != &element,
                };
                if needs_insert {
                    drive_operations.push(
                        LowLevelDriveOperation::insert_for_known_path_key_element(
                            path,
                            key.to_vec(),
                            element,
                        ),
                    );
                }
                Ok((needs_insert, previous_element))
            }
            PathKeyElement((path, key, element)) => {
                let previous_element = self.grove_get_raw_optional(
                    path.as_slice().into(),
                    key.as_slice(),
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                let needs_insert = match &previous_element {
                    None => true,
                    Some(previous_element) => previous_element != &element,
                };
                if needs_insert {
                    drive_operations.push(
                        LowLevelDriveOperation::insert_for_known_path_key_element(
                            path, key, element,
                        ),
                    );
                }
                Ok((needs_insert, previous_element))
            }
            PathFixedSizeKeyRefElement((path, key, element)) => {
                let previous_element = self.grove_get_raw_optional(
                    (&path).into(),
                    key,
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                let needs_insert = match &previous_element {
                    None => true,
                    Some(previous_element) => previous_element != &element,
                };
                if needs_insert {
                    let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                    drive_operations.push(
                        LowLevelDriveOperation::insert_for_known_path_key_element(
                            path_items,
                            key.to_vec(),
                            element,
                        ),
                    );
                }
                Ok((needs_insert, previous_element))
            }
            PathKeyElementSize((key_info_path, key_info, element)) => {
                match apply_type {
                    BatchInsertApplyType::StatelessBatchInsert {
                        in_tree_using_sums, ..
                    } => {
                        // we can estimate that the element was the same size
                        drive_operations.push(CalculatedCostOperation(
                            GroveDb::average_case_for_get_raw(
                                &key_info_path,
                                &key_info,
                                element.serialized_size()? as u32,
                                in_tree_using_sums,
                            ),
                        ));
                        drive_operations.push(
                            LowLevelDriveOperation::insert_for_estimated_path_key_element(
                                key_info_path,
                                key_info,
                                element,
                            ),
                        );
                        Ok((true, None))
                    }
                    BatchInsertApplyType::StatefulBatchInsert => {
                        Err(Error::Drive(DriveError::NotSupportedPrivate(
                            "document sizes for stateful insert in batch operations not supported",
                        )))
                    }
                }
            }
            PathKeyUnknownElementSize(_) => Err(Error::Drive(DriveError::NotSupportedPrivate(
                "document sizes in batch operations not supported",
            ))),
        }
    }
}
