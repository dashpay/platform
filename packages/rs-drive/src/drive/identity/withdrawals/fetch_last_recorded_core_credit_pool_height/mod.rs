mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Fetches the highest Core height whose credit pool balance was recorded: the last Core
    /// block the scan of `scan_core_blocks_for_withdrawals` read. Shares the
    /// `fetch_core_credit_pool_balances` method version.
    ///
    /// # Parameters
    ///
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(u32))`: The highest recorded Core height.
    /// * `Ok(None)`: When no balance was recorded yet.
    /// * `Err(Error)` when the method version is unknown or not active, the entry is corrupted,
    ///   or the read fails.
    pub fn fetch_last_recorded_core_credit_pool_height(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<u32>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .fetch_core_credit_pool_balances
        {
            Some(0) => {
                self.fetch_last_recorded_core_credit_pool_height_v0(transaction, platform_version)
            }
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_last_recorded_core_credit_pool_height".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_last_recorded_core_credit_pool_height".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
