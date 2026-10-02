mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::Credits;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Records Core's credit pool balance after a Core block, under the block's height, for the
    /// Core-anchored withdrawal limit.
    ///
    /// # Parameters
    ///
    /// * `core_height`: The height of the Core block.
    /// * `credit_pool_balance`: Core's credit pool balance after the block, in credits.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once the balance is stored.
    /// * `Err(Error)` when the method version is unknown or not active, or the write fails.
    pub fn record_core_credit_pool_block(
        &self,
        core_height: u32,
        credit_pool_balance: Credits,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .record_core_credit_pool_block
        {
            Some(0) => self.record_core_credit_pool_block_v0(
                core_height,
                credit_pool_balance,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "record_core_credit_pool_block".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "record_core_credit_pool_block".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
