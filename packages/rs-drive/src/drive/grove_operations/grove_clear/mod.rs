mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::drive_versions::DriveVersion;

use grovedb::TransactionArg;
use grovedb_path::SubtreePath;

impl Drive {
    /// Handles the deletion of an element in GroveDB at the specified path and key.
    /// The operation cost is added to `drive_operations` for later processing.
    ///
    /// # Parameters
    /// * `path`: The groveDB hierarchical authenticated structure path where the subtree is to be cleared.
    /// * `transaction`: The groveDB transaction associated with this operation.
    /// * `drive_operations`: A vector to collect the costs of operations for later computation.
    /// * `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the platform version does not match known versions.
    pub fn grove_clear<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version.grove_methods.basic.grove_clear {
            0 => self.grove_clear_v0(path, transaction, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_clear".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
