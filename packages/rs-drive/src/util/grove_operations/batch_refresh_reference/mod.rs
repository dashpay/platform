mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;
use grovedb::Element;

impl Drive {
    /// Pushes an "refresh reference" operation to `drive_operations`.
    ///
    /// # Parameters
    /// * `path`: The path of the reference to be refreshed.
    /// * `key`: The key of the reference to be refreshed.
    /// * `document_reference`: The element to be referenced.
    /// * `trust_refresh_reference`: Flag to trust the refresh reference.
    /// * `drive_operations`: The list of drive operations to append to.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the reference was successfully refreshed.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    /// * `Err(DriveError::CorruptedCodeExecution)` if expected a reference on refresh.
    pub fn batch_refresh_reference(
        &self,
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        document_reference: Element,
        trust_refresh_reference: bool,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version.grove_methods.batch.batch_refresh_reference {
            0 => self.batch_refresh_reference_v0(
                path,
                key,
                document_reference,
                trust_refresh_reference,
                drive_operations,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_refresh_reference".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
