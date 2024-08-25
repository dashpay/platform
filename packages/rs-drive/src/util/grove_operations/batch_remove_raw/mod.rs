mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchDeleteApplyType;

use dpp::version::drive_versions::DriveVersion;

use grovedb::{Element, TransactionArg};
use grovedb_path::SubtreePath;

impl Drive {
    /// Pushes a "delete element" operation to `drive_operations` and returns the current element.
    /// If the element didn't exist does nothing.
    /// It is raw, because it does not use references.
    ///
    /// # Parameters
    /// * `path`: The path to the element to delete.
    /// * `key`: The key of the element to delete.
    /// * `apply_type`: The delete operation type.
    /// * `transaction`: The groveDB transaction associated with this operation.
    /// * `drive_operations`: The list of drive operations to append to.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(Some(Element))` if the element was successfully deleted.
    /// * `Ok(None)` if the element does not exist.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn batch_remove_raw<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        apply_type: BatchDeleteApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Option<Element>, Error> {
        match drive_version.grove_methods.batch.batch_remove_raw {
            0 => self.batch_remove_raw_v0(
                path,
                key,
                apply_type,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_remove_raw".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
