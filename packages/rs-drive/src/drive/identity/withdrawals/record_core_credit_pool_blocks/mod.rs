mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::Credits;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Records Core's credit pool balance after each of the given Core blocks, under the block's
    /// height, for the Core-anchored withdrawal limit, in one batch.
    ///
    /// # Parameters
    ///
    /// * `balances`: Each Core block's height with Core's credit pool balance after it, in
    ///   credits.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once the balances are stored, or at once when there are none.
    /// * `Err(Error)` when the method version is unknown or not active, or the write fails.
    pub fn record_core_credit_pool_blocks(
        &self,
        balances: &[(u32, Credits)],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .record_core_credit_pool_blocks
        {
            Some(0) => {
                self.record_core_credit_pool_blocks_v0(balances, transaction, platform_version)
            }
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "record_core_credit_pool_blocks".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "record_core_credit_pool_blocks".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
