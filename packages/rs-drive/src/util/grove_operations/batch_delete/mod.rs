mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchDeleteApplyType;

use dpp::version::drive_versions::DriveVersion;

use grovedb::TransactionArg;
use grovedb_path::SubtreePath;

impl Drive {
    /// Pushes a "delete element" operation to `drive_operations`.
    ///
    /// # Parameters
    /// * `path`: The path to delete.
    /// * `key`: The key of the item to delete.
    /// * `apply_type`: The apply type for the delete operation.
    /// * `transaction`: The transaction argument.
    /// * `drive_operations`: The vector containing low-level drive operations.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn batch_delete<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        apply_type: BatchDeleteApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version.grove_methods.batch.batch_delete {
            0 => self.batch_delete_v0(
                path,
                key,
                apply_type,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_delete".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
