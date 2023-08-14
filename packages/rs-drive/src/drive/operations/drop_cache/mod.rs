mod v0;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use dpp::version::drive_versions::DriveVersion;

impl Drive {
    /// Drops the drive cache.
    ///
    /// This is a versioned method that resets the current drive cache to the default state
    /// based on the drive configuration.
    ///
    /// # Arguments
    ///
    /// * `drive_version` - A `DriveVersion` reference that dictates which version of the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - On success, returns `Ok(())`. On error, returns an `Error`.
    ///
    pub fn drop_cache(&self, drive_version: &DriveVersion) -> Result<(), Error> {
        match drive_version.methods.operations.drop_cache {
            0 => {
                self.drop_cache_v0();
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "drop_cache".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
