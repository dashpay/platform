mod v0;

use crate::util::grove_operations::BatchInsertApplyType;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::PathKeyElementInfo;

use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Attempts to batch insert a sum item at the specified path and key if it doesn't already exist.
    /// This method dispatches to the appropriate version of the `batch_insert_sum_item_if_not_exists` function
    /// based on the version of the drive. Currently, version `0` is supported.
    ///
    /// # Parameters
    /// * `path_key_element_info`: Contains the path, key, and element information to be inserted.
    /// * `error_if_exists`: A flag that determines whether an error is returned if a sum item already exists at the given path and key.
    /// * `apply_type`: Defines the batch insert type, such as stateless or stateful insertion.
    /// * `transaction`: The transaction argument used for the operation.
    /// * `drive_operations`: A mutable reference to a vector that collects low-level drive operations to be executed.
    /// * `drive_version`: The version of the drive that influences the behavior of the batch insert operation.
    ///
    /// # Returns
    /// * `Ok(())` if the batch insert is successful.
    /// * `Err(Error)` if the operation fails, including an error for unknown version mismatches.
    ///
    /// # Description
    /// This function checks the version of the drive's batch methods and dispatches the operation to the appropriate version of
    /// `batch_insert_sum_item_if_not_exists`. Currently, only version `0` is supported, which delegates to the function
    /// `batch_insert_sum_item_if_not_exists_v0`. If the drive version is not supported, an error is returned.
    ///
    /// In version `0`, the function performs the following:
    /// - Checks if a sum item exists at the specified path and key.
    /// - If the sum item exists and `error_if_exists` is true, an error is returned.
    /// - If no sum item exists, a new sum item is inserted at the path and key.
    ///
    /// This method allows flexibility for future versions of the drive to implement different behaviors for batch insertion.
    pub fn batch_insert_sum_item_if_not_exists<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        error_if_exists: bool,
        apply_type: BatchInsertApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match drive_version
            .grove_methods
            .batch
            .batch_insert_sum_item_if_not_exists
        {
            0 => self.batch_insert_sum_item_if_not_exists_v0(
                path_key_element_info,
                error_if_exists,
                apply_type,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_insert_sum_item_if_not_exists".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
