mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::Transaction;

impl Drive {
    /// Prunes shielded pool anchors older than the given cutoff height.
    ///
    /// Queries the anchors-by-height tree for all entries with
    /// `block_height < cutoff_height`, then deletes the corresponding entries
    /// from both the anchors-by-height tree (`block_height -> anchor_bytes`)
    /// and the primary anchors tree (`anchor_bytes -> block_height`).
    ///
    /// The caller is responsible for determining whether pruning should happen
    /// (interval checks, retention depth, etc.).
    ///
    /// # Parameters
    /// - `cutoff_height`: All anchors recorded at heights strictly below this are pruned
    /// - `transaction`: The GroveDB transaction
    /// - `platform_version`: The platform version for dispatch
    pub fn prune_shielded_pool_anchors(
        &self,
        cutoff_height: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version.drive.methods.shielded.prune_anchors {
            0 => self.prune_shielded_pool_anchors_v0(cutoff_height, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prune_shielded_pool_anchors".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
