mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Records the credits an asset lock minted into Platform as a credit inflow dated by the
    /// Core block that mined it, the way Core counts it, rather than by the Platform block that
    /// consumed it. An asset lock Core mined so long ago that it left the window
    /// (`core_credit_pool_window_min_blocks`) records nothing: a lock published late adds no
    /// budget Core does not grant. One Core has not mined at or below the block's chain locked
    /// height is recorded as pending, and counts once a Core block holding it is read
    /// (`record_core_credit_pool_block`).
    ///
    /// # Parameters
    ///
    /// * `asset_lock_txid`: The id of the asset lock transaction.
    /// * `amount`: The credits it minted in this block.
    /// * `mined_at_core_height`: The height of the Core block that mined it, when that is at or
    ///   below the block's chain locked height; `None` otherwise.
    /// * `block_info`: The Platform block being executed.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once the inflow is recorded (dated or pending), or at once when `amount` is
    ///   zero or the inflow is already out of the window.
    /// * `Err(Error)` when the method version is unknown or not active, a stored entry is
    ///   corrupted, or the write fails.
    pub fn record_asset_lock_credit_inflow(
        &self,
        asset_lock_txid: [u8; 32],
        amount: Credits,
        mined_at_core_height: Option<u32>,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .record_asset_lock_credit_inflow
        {
            Some(0) => self.record_asset_lock_credit_inflow_v0(
                asset_lock_txid,
                amount,
                mined_at_core_height,
                block_info,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "record_asset_lock_credit_inflow".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "record_asset_lock_credit_inflow".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
