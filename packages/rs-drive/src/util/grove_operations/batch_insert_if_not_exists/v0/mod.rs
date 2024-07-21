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
use grovedb::{GroveDb, TransactionArg};

impl Drive {
    /// Pushes an "insert element if the path key does not yet exist" operation to `drive_operations`.
    /// Returns true if the path key already exists without references.
    pub(crate) fn batch_insert_if_not_exists_v0<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        apply_type: BatchInsertApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match path_key_element_info {
            PathKeyRefElement((path, key, element)) => {
                let has_raw = self.grove_has_raw(
                    path.as_slice().into(),
                    key,
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                if !has_raw {
                    drive_operations.push(
                        LowLevelDriveOperation::insert_for_known_path_key_element(
                            path,
                            key.to_vec(),
                            element,
                        ),
                    );
                }
                Ok(!has_raw)
            }
            PathKeyElement((path, key, element)) => {
                let has_raw = self.grove_has_raw(
                    path.as_slice().into(),
                    key.as_slice(),
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                if !has_raw {
                    drive_operations.push(
                        LowLevelDriveOperation::insert_for_known_path_key_element(
                            path, key, element,
                        ),
                    );
                }
                Ok(!has_raw)
            }
            PathFixedSizeKeyRefElement((path, key, element)) => {
                let has_raw = self.grove_has_raw(
                    path.as_slice().into(),
                    key,
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                if !has_raw {
                    let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                    drive_operations.push(
                        LowLevelDriveOperation::insert_for_known_path_key_element(
                            path_items,
                            key.to_vec(),
                            element,
                        ),
                    );
                }
                Ok(!has_raw)
            }
            PathKeyElementSize((key_info_path, key_info, element)) => {
                match apply_type {
                    BatchInsertApplyType::StatelessBatchInsert {
                        in_tree_using_sums, ..
                    } => {
                        // we can estimate that the element was the same size
                        drive_operations.push(CalculatedCostOperation(
                            GroveDb::average_case_for_has_raw(
                                &key_info_path,
                                &key_info,
                                element.serialized_size(&drive_version.grove_version)? as u32,
                                in_tree_using_sums,
                                &drive_version.grove_version,
                            )?,
                        ));
                        drive_operations.push(
                            LowLevelDriveOperation::insert_for_estimated_path_key_element(
                                key_info_path,
                                key_info,
                                element,
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
            }
            PathKeyUnknownElementSize(_) => Err(Error::Drive(DriveError::NotSupportedPrivate(
                "document sizes in batch operations not supported",
            ))),
        }
    }
}
