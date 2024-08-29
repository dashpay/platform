mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use grovedb::TransactionArg;

use dpp::version::PlatformVersion;

impl Drive {
    /// Returns the start time of the given Epoch.
    ///
    /// # Arguments
    ///
    /// * `epoch_tree` - An Epoch instance representing the epoch.
    /// * `transaction` - A TransactionArg instance.
    /// * `platform_version` - A PlatformVersion instance representing the version of the drive.
    ///
    /// # Returns
    ///
    /// A Result containing the epoch start time or an Error.
    pub fn get_epoch_start_time(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .epochs
            .get_epoch_start_time
        {
            0 => self.get_epoch_start_time_v0(epoch_tree, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_epoch_start_time".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
