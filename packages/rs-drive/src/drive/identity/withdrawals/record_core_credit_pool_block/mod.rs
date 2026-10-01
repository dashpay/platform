mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Records what a Core block tells the Core-anchored withdrawal limit: Core's credit pool
    /// balance after it, and which of the asset locks Platform consumed before Core mined them
    /// it holds. The balance is stored under the block's height; each such asset lock leaves the
    /// pending tree and, while the block is still inside the window
    /// (`core_credit_pool_window_min_blocks` past it), its credits are recorded as a credit
    /// inflow dated by this block.
    ///
    /// # Parameters
    ///
    /// * `core_height`: The height of the Core block.
    /// * `credit_pool_balance`: Core's credit pool balance after the block, in credits.
    /// * `asset_lock_txids`: The ids of the asset lock transactions the block holds.
    /// * `block_info`: The Platform block being executed; its Core height decides whether a
    ///   dated inflow still counts, and its time is not used (a resolved entry keeps the time
    ///   its credits were minted at).
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once the balance is stored and the pending asset locks of the block resolved.
    /// * `Err(Error)` when the method version is unknown or not active, a stored entry is
    ///   corrupted, or the write fails.
    pub fn record_core_credit_pool_block(
        &self,
        core_height: u32,
        credit_pool_balance: Credits,
        asset_lock_txids: &[[u8; 32]],
        block_info: &BlockInfo,
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
                asset_lock_txids,
                block_info,
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
