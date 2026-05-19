mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::Transaction;

impl Drive {
    /// Records the current shielded pool anchor if the commitment tree changed
    /// this block.
    ///
    /// Reads the current Sinsemilla anchor from the CommitmentTree, compares it
    /// to the most recent stored anchor, and if different (and non-zero) writes
    /// entries to the anchors tree, anchors-by-height tree, and updates the
    /// most recent anchor item.
    ///
    /// # Parameters
    /// - `block_height`: The current block height
    /// - `transaction`: The GroveDB transaction
    /// - `platform_version`: The platform version for dispatch
    pub fn record_shielded_pool_anchor_if_changed(
        &self,
        block_height: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .shielded
            .record_anchor_if_changed
        {
            0 => self.record_shielded_pool_anchor_if_changed_v0(
                block_height,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "record_shielded_pool_anchor_if_changed".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
