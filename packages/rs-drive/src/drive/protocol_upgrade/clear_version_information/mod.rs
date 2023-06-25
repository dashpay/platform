use grovedb::TransactionArg;
use dpp::version::drive_versions::DriveVersion;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

mod v0;

impl Drive {
    /// Clear all version information from the backing store, this is done on epoch change in
    /// execution logic
    pub fn clear_version_information(&self, transaction: TransactionArg, drive_version: &DriveVersion,) -> Result<(), Error> {
        match drive_version.methods.protocol_upgrade.clear_version_information {
            0 => self.clear_version_information_v0(transaction, platform_version),
            version => Error::Drive(DriveError::UnknownVersionMismatch  {
                method: "clear_version_information".to_string(),
                known_versions: vec![0],
                received: version,
            })
        }
    }
}