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
    /// Version 0 implementation of the "insert element if the path key does not yet exist" operation.
    /// If the element already exists, it returns the existing element.
    ///
    /// This function checks whether an element exists at a specified path and key.
    /// If the element exists, it returns the existing element. If not, it inserts the new element
    /// into the database and returns `None`. This operation supports different types of path, key, and element configurations
    /// and can be applied in either stateless or stateful contexts.
    ///
    /// # Parameters
    ///
    /// * `path_key_element_info`: Information about the path, key, and element to insert.
    ///   - Supports multiple configurations: direct references, owned elements, fixed size keys, or estimated sizes.
    /// * `apply_type`: The application type of the operation, defining whether the operation is stateless or stateful.
    /// * `transaction`: The transaction context for the operation, allowing it to be atomic within a batch.
    /// * `drive_operations`: A mutable reference to the list of drive operations to which this operation will be appended.
    /// * `drive_version`: The version of the drive being used, ensuring compatibility with the function version.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Element))`: If the element already exists at the specified path and key, returning the existing element.
    /// * `Ok(None)`: If the element was successfully inserted because it did not exist before.
    /// * `Err(Error)`: Returns an error if:
    ///   - The insertion operation is not supported in the current state.
    ///   - The operation encounters any unexpected issues related to invalid configurations or unsupported features.
    ///
    /// # Errors
    ///
    /// * `Error::Drive(DriveError::NotSupportedPrivate)`: If the function encounters unsupported configurations, such as document sizes for stateful inserts.
    /// * `Error::Drive(DriveError::UnknownVersionMismatch)`: If the drive version is not supported for the operation.
    ///
    /// # PathKeyElementInfo Variants
    ///
    /// The function supports various `PathKeyElementInfo` variants:
    /// * `PathKeyRefElement`: Reference to the path, key, and element.
    /// * `PathKeyElement`: Owned path, key, and element.
    /// * `PathFixedSizeKeyRefElement`: Reference to the path with a fixed-size key and element.
    /// * `PathKeyElementSize`: Path and key with known element size, used for estimation.
    /// * `PathKeyUnknownElementSize`: Unsupported in this version and returns an error.
    pub(super) fn batch_insert_if_not_exists_return_existing_element_v0<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        apply_type: BatchInsertApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Option<Element>, Error> {
        match path_key_element_info {
            PathKeyRefElement((path, key, element)) => {
                // Check if the element already exists
                let existing_element = self.grove_get_raw_optional(
                    path.as_slice().into(),
                    key,
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                if let Some(existing_element) = existing_element {
                    return Ok(Some(existing_element));
                }

                // Element does not exist, proceed with insertion
                drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
                    path,
                    key.to_vec(),
                    element,
                ));
                Ok(None)
            }
            PathKeyElement((path, key, element)) => {
                // Check if the element already exists
                let existing_element = self.grove_get_raw_optional(
                    path.as_slice().into(),
                    key.as_slice(),
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                if let Some(existing_element) = existing_element {
                    return Ok(Some(existing_element));
                }

                // Element does not exist, proceed with insertion
                drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
                    path, key, element,
                ));
                Ok(None)
            }
            PathFixedSizeKeyRefElement((path, key, element)) => {
                // Check if the element already exists
                let existing_element = self.grove_get_raw_optional(
                    path.as_slice().into(),
                    key,
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                if let Some(existing_element) = existing_element {
                    return Ok(Some(existing_element));
                }

                // Element does not exist, proceed with insertion
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
                    path_items,
                    key.to_vec(),
                    element,
                ));
                Ok(None)
            }
            PathKeyElementSize((key_info_path, key_info, element)) => {
                match apply_type {
                    BatchInsertApplyType::StatelessBatchInsert {
                        in_tree_type: in_tree_using_sums,
                        ..
                    } => {
                        // Estimate if the element with the given size already exists
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
                        Ok(None)
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
