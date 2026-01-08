mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Cleans up expired compacted address balance entries.
    ///
    /// This function queries the expiration time tree for entries with
    /// expiration time <= current_block_time_ms, then deletes:
    /// 1. The corresponding compacted address balance entries
    /// 2. The expiration time entries themselves
    ///
    /// # Arguments
    /// * `current_block_time_ms` - The current block time in milliseconds
    /// * `transaction` - Optional database transaction
    /// * `platform_version` - The platform version
    ///
    /// # Returns
    /// * `Ok(usize)` - The number of compacted entries that were cleaned up
    /// * `Err` - An error occurred
    pub fn cleanup_expired_address_balances(
        &self,
        current_block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<usize, Error> {
        match platform_version
            .drive
            .methods
            .saved_block_transactions
            .cleanup_expired_address_balances
        {
            0 => self.cleanup_expired_address_balances_v0(
                current_block_time_ms,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "cleanup_expired_address_balances".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
