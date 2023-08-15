mod v0;

use grovedb::TransactionArg;
use std::ops::Range;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::PlatformVersion;

impl Drive {
    /// Returns a list of storage credits to be distributed to proposers from a range of epochs.
    ///
    /// # Arguments
    ///
    /// * `epoch_range` - A Range<u16> specifying the epochs for which to get the storage credits.
    /// * `transaction` - A TransactionArg instance.
    /// * `drive_version` - A DriveVersion instance representing the version of the drive.
    ///
    /// # Returns
    ///
    /// A Vec<u64> containing the storage credits for each epoch in the specified range.
    pub fn get_storage_credits_for_distribution_for_epochs_in_range(
        &self,
        epoch_range: Range<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u64>, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .epochs
            .get_storage_credits_for_distribution_for_epochs_in_range
        {
            0 => Ok(
                self.get_storage_credits_for_distribution_for_epochs_in_range_v0(
                    epoch_range,
                    transaction,
                    platform_version,
                ),
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_storage_credits_for_distribution_for_epochs_in_range".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
