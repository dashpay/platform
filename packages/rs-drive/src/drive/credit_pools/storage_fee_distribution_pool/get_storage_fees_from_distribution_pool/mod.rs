mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;

use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

use crate::error::Error;

impl Drive {
    /// Returns the amount of credits in the storage fee distribution pool based on the provided platform version.
    ///
    /// # Parameters
    ///
    /// - `transaction`: A transaction argument to facilitate the database operation.
    /// - `platform_version`: The platform version against which to get the storage fees.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with the `Credits` from the storage fee distribution pool.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - An unknown or unsupported platform version is provided.
    /// - Any other error as documented in the specific versioned function.
    pub fn get_storage_fees_from_distribution_pool(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .storage_fee_distribution_pool
            .get_storage_fees_from_distribution_pool
        {
            0 => self.get_storage_fees_from_distribution_pool_v0(transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_storage_fees_from_distribution_pool".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
