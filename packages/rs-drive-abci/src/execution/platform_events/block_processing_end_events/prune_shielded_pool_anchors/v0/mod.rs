use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Prunes anchors older than `shielded_anchor_retention_blocks` from the current height.
    ///
    /// Checks interval and retention depth conditions, then delegates to
    /// `Drive::prune_shielded_pool_anchors` for the actual GroveDB operations.
    pub(super) fn prune_shielded_pool_anchors_v0(
        &self,
        block_height: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let event_constants = &platform_version
            .drive_abci
            .validation_and_processing
            .event_constants;
        let retention_blocks = event_constants.shielded_anchor_retention_blocks;
        let pruning_interval = event_constants.shielded_anchor_pruning_interval;

        // Only prune every N blocks to avoid unnecessary work
        if !block_height.is_multiple_of(pruning_interval) {
            return Ok(());
        }

        // Nothing to prune if we haven't reached the retention depth yet
        if block_height <= retention_blocks {
            return Ok(());
        }

        let cutoff_height = block_height - retention_blocks;

        self.drive
            .prune_shielded_pool_anchors(cutoff_height, transaction, platform_version)
            .map_err(Error::Drive)
    }
}
