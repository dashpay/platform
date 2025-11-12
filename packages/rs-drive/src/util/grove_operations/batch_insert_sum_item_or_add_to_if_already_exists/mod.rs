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
    /// Pushes an "insert sum item or add to it if the item already exists" operation to `drive_operations`.
    /// This operation either inserts a new sum item at the given path and key or adds the value to the existing sum item.
    ///
    /// # Parameters
    /// - `path_key_element_info`: Information about the path, key, and element.
    /// - `apply_type`: The apply type for the operation.
    /// - `transaction`: The transaction argument for the operation.
    /// - `drive_operations`: The list of drive operations to append to.
    /// - `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// - `Ok(())` if the operation was successful.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    /// - `Err(DriveError::CorruptedCodeExecution)` (rare) if the operation is not supported.
    /// - `Err(DriveError::CorruptedElementType)` (rare) if drive is in a corrupted state and
    ///   gives back an incorrect element type.
    pub fn batch_insert_sum_item_or_add_to_if_already_exists<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        apply_type: BatchInsertApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .grove_methods
            .batch
            .batch_insert_sum_item_or_add_to_if_already_exists
        {
            0 => self.batch_insert_sum_item_or_add_to_if_already_exists_v0(
                path_key_element_info,
                apply_type,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_insert_sum_item_or_add_to_if_already_exists".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
