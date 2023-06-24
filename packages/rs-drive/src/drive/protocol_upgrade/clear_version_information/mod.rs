use grovedb::TransactionArg;
use dpp::version::PlatformVersion;
use crate::drive::Drive;
use crate::error::Error;

mod v0;

impl Drive {
    /// Clear all version information from the backing store, this is done on epoch change in
    /// execution logic
    pub fn clear_version_information(&self, transaction: TransactionArg, platform_version: &PlatformVersion) -> Result<(), Error> {
        match platform_version.drive.methods.protocol_upgrade.clear_version_information {
            0 => self.clear_version_information_v0(transaction, platform_version),
            version => Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "clear_version_information".to_string(),
                known_versions: vec![0],
                received: version,
            })
        }
    }
}