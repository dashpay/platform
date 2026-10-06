mod v0;

use crate::drive::shielded::paths::{
    token_shielded_pool_anchors_by_height_path, token_shielded_pool_anchors_path,
    token_shielded_pool_latest_recorded_anchor_path_query, token_shielded_pool_path,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::{Transaction, TransactionArg};

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

impl Drive {
    /// Records a TOKEN shielded pool's anchor if its commitment tree changed this block. Same
    /// versioning as [`Drive::record_shielded_pool_anchor_if_changed`].
    pub fn record_token_shielded_pool_anchor_if_changed(
        &self,
        token_id: [u8; 32],
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
            0 => {
                let pool_path = token_shielded_pool_path(&token_id);
                let anchors_path = token_shielded_pool_anchors_path(&token_id);
                let anchors_by_height_path = token_shielded_pool_anchors_by_height_path(&token_id);
                self.record_pool_anchor_if_changed_v0(
                    &pool_path,
                    &anchors_path,
                    &anchors_by_height_path,
                    &token_shielded_pool_latest_recorded_anchor_path_query(token_id),
                    block_height,
                    transaction,
                    platform_version,
                )
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "record_token_shielded_pool_anchor_if_changed".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Reads the latest anchor recorded for a TOKEN shielded pool (`None` before the first
    /// block-end record). Same versioning as [`Drive::record_shielded_pool_anchor_if_changed`].
    pub fn read_latest_recorded_token_shielded_pool_anchor(
        &self,
        token_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<[u8; 32]>, Error> {
        match platform_version
            .drive
            .methods
            .shielded
            .record_anchor_if_changed
        {
            0 => self.read_latest_recorded_pool_anchor_v0(
                &token_shielded_pool_latest_recorded_anchor_path_query(token_id),
                transaction,
                &platform_version.drive,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "read_latest_recorded_token_shielded_pool_anchor".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
