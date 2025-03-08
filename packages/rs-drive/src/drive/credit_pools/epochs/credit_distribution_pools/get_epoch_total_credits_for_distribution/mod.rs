mod v0;

use grovedb::TransactionArg;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::block::pool_credits::StorageAndProcessingPoolCredits;

use dpp::version::PlatformVersion;

impl Drive {
    /// Gets the total credits to be distributed for the Epoch.
    ///
    /// # Arguments
    ///
    /// * `epoch_tree` - A reference to the Epoch.
    /// * `transaction` - A TransactionArg instance.
    /// * `drive_version` - A DriveVersion instance representing the version of the drive.
    ///
    /// # Returns
    ///
    /// A Result containing either the total credits for the epoch as a StorageAndProcessingPoolCredits
    /// if found, or an Error if something goes wrong.
    pub fn get_epoch_total_credits_for_distribution(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<StorageAndProcessingPoolCredits, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .epochs
            .get_epoch_total_credits_for_distribution
        {
            0 => self.get_epoch_total_credits_for_distribution_v0(
                epoch_tree,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_epoch_total_credits_for_distribution".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
