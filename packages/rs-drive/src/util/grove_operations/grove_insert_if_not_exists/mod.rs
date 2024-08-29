mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;
use grovedb::{Element, TransactionArg};
use grovedb_path::SubtreePath;

impl Drive {
    /// Inserts an element into groveDB only if the specified path and key do not exist.
    /// This operation costs are then stored in `drive_operations`.
    ///
    /// # Parameters
    /// * `path`: The groveDB hierarchical authenticated structure path where the new element is to be inserted.
    /// * `key`: The key where the new element should be inserted in the subtree.
    /// * `element`: The element to be inserted.
    /// * `transaction`: The groveDB transaction associated with this operation.
    /// * `drive_operations`: A vector to collect the costs of operations for later computation. In this case,
    /// it collects the cost of this insert operation if the path and key did not exist.
    /// * `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(true)` if the insertion was successful.
    /// * `Ok(false)` if the path and key already existed.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the platform version does not match known versions.
    pub fn grove_insert_if_not_exists<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        element: Element,
        transaction: TransactionArg,
        drive_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match drive_version.grove_methods.basic.grove_insert_if_not_exists {
            0 => self.grove_insert_if_not_exists_v0(
                path,
                key,
                element,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_insert_if_not_exists".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
