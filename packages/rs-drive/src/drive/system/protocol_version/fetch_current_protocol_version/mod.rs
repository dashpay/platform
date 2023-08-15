mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::util::deserializer::ProtocolVersion;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Gets the current protocol version from the backing store
    ///
    /// # Arguments
    ///
    /// * `transaction` - A `TransactionArg` object representing the transaction.
    ///
    /// # Returns
    ///
    /// * `Result<Option<ProtocolVersion>, Error>` - If successful, returns an `Ok(Option<ProtocolVersion>)`. If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the Drive version is unknown.
    pub fn fetch_current_protocol_version(
        &self,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Option<ProtocolVersion>, Error> {
        match drive_version
            .methods
            .platform_system
            .protocol_version
            .fetch_current_protocol_version
        {
            0 => self.fetch_current_protocol_version_v0(transaction),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_current_protocol_version".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
