mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::fee_result::LifetimeStorageFees;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Reads the lifetime storage fee pools: the storage fees, by the number of epochs their
    /// storage lives, waiting for the next epoch change to spread them over those epochs
    /// (protocol version 14, document time to live). Empty when the pools tree does not exist.
    ///
    /// # Parameters
    /// - `transaction`: the transaction to read in.
    /// - `platform_version`: selects the method version.
    ///
    /// # Returns
    /// The credits of each lifetime pool, by its number of epochs.
    pub fn fetch_lifetime_storage_fee_pools(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<LifetimeStorageFees, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .storage_fee_distribution_pool
            .fetch_lifetime_storage_fee_pools
        {
            0 => self.fetch_lifetime_storage_fee_pools_v0(transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_lifetime_storage_fee_pools".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
