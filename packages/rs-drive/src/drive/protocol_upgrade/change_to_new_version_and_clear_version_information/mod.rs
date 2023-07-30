use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::drive_versions::DriveVersion;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::TransactionArg;

mod v0;

impl Drive {
    /// Clear all version information from the backing store, this is done on epoch change in
    /// execution logic
    pub fn change_to_new_version_and_clear_version_information(
        &self,
        current_version: ProtocolVersion,
        next_version: ProtocolVersion,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        let platform_version = PlatformVersion::get(current_version)
            .map_err(|a| ProtocolError::PlatformVersionError(a))?;
        match platform_version
            .drive
            .methods
            .protocol_upgrade
            .change_to_new_version_and_clear_version_information
        {
            0 => self.change_to_new_version_and_clear_version_information_v0(
                current_version,
                next_version,
                transaction,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "change_to_new_version_and_clear_version_information".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
