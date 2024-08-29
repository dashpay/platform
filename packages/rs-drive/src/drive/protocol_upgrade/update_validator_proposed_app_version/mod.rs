mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Updates the proposed app version for a validator.
    ///
    /// # Arguments
    ///
    /// * `validator_pro_tx_hash` - The ProTx hash of the validator.
    /// * `version` - The proposed app version to be set.
    /// * `transaction` - A transaction argument to interact with the underlying storage.
    ///
    /// # Returns
    ///
    /// * `Result<bool, Error>` - Returns a boolean indicating if the value was changed,
    ///                            or an error if an issue was encountered.
    ///
    /// # Errors
    ///
    /// This function may return an error if any of the following conditions are met:
    ///
    /// * There is an issue interacting with the underlying storage.
    /// * The cache state is corrupted.
    pub fn update_validator_proposed_app_version(
        &self,
        validator_pro_tx_hash: [u8; 32],
        version: ProtocolVersion,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match drive_version
            .methods
            .protocol_upgrade
            .update_validator_proposed_app_version
        {
            0 => self.update_validator_proposed_app_version_v0(
                validator_pro_tx_hash,
                version,
                transaction,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_validator_proposed_app_version".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Update the validator proposed app version and returns the drive operations
    /// required to commit the changes to the database.
    ///
    /// # Arguments
    ///
    /// * `validator_pro_tx_hash` - The ProTx hash of the validator.
    /// * `version` - The proposed app version to be set.
    /// * `transaction` - A transaction argument to interact with the underlying storage.
    /// * `drive_operations` - A mutable reference to a vector of low-level drive operations
    ///                        that will be populated with the required changes.
    ///
    /// # Returns
    ///
    /// * `Result<bool, Error>` - Returns a boolean indicating if the value was changed,
    ///                            or an error if an issue was encountered.
    ///
    /// # Errors
    ///
    /// This function may return an error if any of the following conditions are met:
    ///
    /// * There is an issue interacting with the underlying storage.
    /// * The cache state is corrupted.
    pub fn update_validator_proposed_app_version_operations(
        &self,
        validator_pro_tx_hash: [u8; 32],
        version: ProtocolVersion,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match drive_version
            .methods
            .protocol_upgrade
            .update_validator_proposed_app_version
        {
            0 => self.update_validator_proposed_app_version_operations_v0(
                validator_pro_tx_hash,
                version,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_validator_proposed_app_version_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
