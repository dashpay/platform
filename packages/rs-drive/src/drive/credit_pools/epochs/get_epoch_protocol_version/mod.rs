mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::block::epoch::Epoch;

use grovedb::TransactionArg;

use dpp::version::PlatformVersion;
use platform_version::version::ProtocolVersion;

impl Drive {
    /// Returns the protocol version for the epoch
    ///
    /// # Arguments
    ///
    /// * `epoch_tree` - An Epoch instance.
    /// * `transaction` - A TransactionArg instance.
    /// * `platform_version` - A PlatformVersion instance representing the version of Platform.
    ///
    /// # Returns
    ///
    /// A Result containing the start block height or an Error.
    pub fn get_epoch_protocol_version(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ProtocolVersion, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .epochs
            .get_epoch_protocol_version
        {
            0 => self.get_epoch_protocol_version_v0(epoch_tree, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_epoch_protocol_version".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Returns the protocol version for the epoch and pushes the cost of reading it onto
    /// `drive_operations`, for a read a fee has to pay for.
    ///
    /// # Arguments
    ///
    /// * `epoch_tree` - An Epoch instance.
    /// * `transaction` - A TransactionArg instance.
    /// * `drive_operations` - Receives the cost of the read.
    /// * `platform_version` - A PlatformVersion instance representing the version of Platform.
    ///
    /// # Returns
    ///
    /// A Result containing the protocol version or an Error.
    pub fn get_epoch_protocol_version_operations(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ProtocolVersion, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .epochs
            .get_epoch_protocol_version
        {
            0 => self.get_epoch_protocol_version_operations_v0(
                epoch_tree,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_epoch_protocol_version_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
