mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchDeleteUpTreeApplyType;

use dpp::version::drive_versions::DriveVersion;

use grovedb::batch::KeyInfoPath;

use grovedb::TransactionArg;

impl Drive {
    /// Pushes a "delete up tree while empty" operation to `drive_operations`.
    ///
    /// # Parameters
    /// * `path`: The path to delete.
    /// * `key`: The key of the item to delete.
    /// * `stop_path_height`: The maximum height to delete up the tree.
    /// * `apply_type`: The apply type for the delete operation.
    /// * `transaction`: The transaction argument.
    /// * `check_existing_operations`: The existing operations to check.
    /// * `drive_operations`: The vector containing low-level drive operations.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn batch_delete_up_tree_while_empty(
        &self,
        path: KeyInfoPath,
        key: &[u8],
        stop_path_height: Option<u16>,
        apply_type: BatchDeleteUpTreeApplyType,
        transaction: TransactionArg,
        check_existing_operations: &Option<&mut Vec<LowLevelDriveOperation>>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .grove_methods
            .batch
            .batch_delete_up_tree_while_empty
        {
            0 => self.batch_delete_up_tree_while_empty_v0(
                path,
                key,
                stop_path_height,
                apply_type,
                transaction,
                check_existing_operations,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_delete_up_tree_while_empty".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
