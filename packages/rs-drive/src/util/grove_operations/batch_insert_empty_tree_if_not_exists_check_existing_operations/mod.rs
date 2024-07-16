mod v0;

use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::PathKeyInfo;
use crate::util::storage_flags::StorageFlags;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Pushes an "insert empty tree where path key does not yet exist" operation to `drive_operations`.
    /// Will also check the current drive operations.
    ///
    /// # Parameters
    /// * `path_key_info`: Information about the path and key.
    /// * `storage_flags`: Optional flags for the storage.
    /// * `apply_type`: The apply type for the operation.
    /// * `transaction`: The transaction argument for the operation.
    /// * `drive_operations`: The list of drive operations to append to.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(bool)` if the operation was successful. Returns true if the path key already exists without references.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    /// * `Err(DriveError::CorruptedCodeExecution)` if the operation is not supported.
    pub fn batch_insert_empty_tree_if_not_exists_check_existing_operations<const N: usize>(
        &self,
        path_key_info: PathKeyInfo<N>,
        use_sum_tree: bool,
        storage_flags: Option<&StorageFlags>,
        apply_type: BatchInsertTreeApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match drive_version
            .grove_methods
            .batch
            .batch_insert_empty_tree_if_not_exists_check_existing_operations
        {
            0 => self.batch_insert_empty_tree_if_not_exists_check_existing_operations_v0(
                path_key_info,
                use_sum_tree,
                storage_flags,
                apply_type,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_insert_empty_tree_if_not_exists_check_existing_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
