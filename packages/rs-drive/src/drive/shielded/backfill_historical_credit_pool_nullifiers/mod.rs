mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::Transaction;

impl Drive {
    /// Adds every stored CREDIT note's rho to the permanent nullifier set.
    ///
    /// Existing Items, including their bytes and flags, are preserved. All reads and
    /// writes belong to the caller's candidate transaction; this method never commits it.
    ///
    /// # Parameters
    ///
    /// * `transaction`: The mandatory candidate transaction.
    /// * `platform_version`: The version selecting the migration and its fixed limits.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the complete union, or an error on inactive/unknown version,
    /// malformed state, an incomplete scan, or a failed write.
    pub fn backfill_historical_credit_pool_nullifiers(
        &self,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .shielded
            .backfill_historical_credit_pool_nullifiers
        {
            Some(0) => {
                self.backfill_historical_credit_pool_nullifiers_v0(transaction, platform_version)
            }
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "backfill_historical_credit_pool_nullifiers".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "backfill_historical_credit_pool_nullifiers".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
