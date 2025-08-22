mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::batch::QualifiedGroveDbOp;

use dpp::block::epoch::Epoch;
use dpp::block::finalized_epoch_info::FinalizedEpochInfo;

use dpp::version::PlatformVersion;

impl Drive {
    /// Adds a finalized epoch information operation to the batch.
    ///
    /// This method creates a `LowLevelDriveOperation` that records the finalized
    /// details of an epoch, such as processing fees, storage fees, block proposers,
    /// and protocol version. The operation is added to the batch for execution.
    ///
    /// The method dispatches to the appropriate versioned implementation based on
    /// the provided `platform_version`.
    ///
    /// # Parameters
    ///
    /// - `epoch`: A reference to the `Epoch` being finalized.
    /// - `finalized_epoch_info`: The finalized information for the epoch, including
    ///   fees, block statistics, and proposer data.
    /// - `platform_version`: The platform version, which determines the correct
    ///   implementation of this method.
    ///
    /// # Returns
    ///
    /// - `Ok(LowLevelDriveOperation)`: A low-level drive operation to be executed.
    /// - `Err(Error)`: If the platform version is unknown or if an internal error occurs.
    ///
    /// # Errors
    ///
    /// - Returns `Error::Drive(DriveError::UnknownVersionMismatch)` if an unsupported
    ///   platform version is encountered.
    ///
    /// # Versioning
    ///
    /// This method supports multiple versions, allowing changes to the internal logic
    /// while maintaining backward compatibility. The method currently supports:
    ///
    /// - **Version 0:** Calls `add_epoch_final_info_operation_v0`.
    ///
    pub fn add_epoch_final_info_operation(
        &self,
        epoch: &Epoch,
        finalized_epoch_info: FinalizedEpochInfo,
        platform_version: &PlatformVersion,
    ) -> Result<QualifiedGroveDbOp, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .epochs
            .add_epoch_final_info_operation
        {
            0 => self.add_epoch_final_info_operation_v0(epoch, finalized_epoch_info),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_epoch_final_info_operation".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
