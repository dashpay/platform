mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::PathKeyElementInfo;
use dpp::version::drive_versions::DriveVersion;

impl Drive {
    /// Pushes a "replace element" operation to `drive_operations`.
    ///
    /// # Parameters
    /// * `path_key_element_info`: The key information of the document and element to insert.
    /// * `drive_operations`: The vector containing low-level drive operations.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn batch_replace<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version.grove_methods.batch.batch_replace {
            0 => self.batch_replace_v0(path_key_element_info, drive_operations),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_replace".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
