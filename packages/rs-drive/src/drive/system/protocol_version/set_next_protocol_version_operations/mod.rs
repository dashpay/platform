mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Sets the next protocol version
    ///
    /// # Arguments
    ///
    /// * `protocol_version` - A `ProtocolVersion` object representing the next protocol version.
    /// * `transaction` - A `TransactionArg` object representing the transaction.
    /// * `drive_operations` - A mutable reference to a vector of `LowLevelDriveOperation` objects.
    /// * `drive_version` - A `DriveVersion` object representing the version of the Drive.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - If successful, returns an `Ok(())`. If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the version of the Drive is unknown.
    // TODO: We should store it for epoch. Will be changed in upcoming PR
    pub fn set_next_protocol_version_operations(
        &self,
        protocol_version: ProtocolVersion,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .platform_system
            .protocol_version
            .set_next_protocol_version_operations
        {
            0 => self.set_next_protocol_version_operations_v0(
                protocol_version,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "set_next_protocol_version_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
