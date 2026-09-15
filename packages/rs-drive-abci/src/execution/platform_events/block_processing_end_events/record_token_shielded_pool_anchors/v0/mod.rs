use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use std::collections::BTreeSet;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// For each touched pool: record the current anchor if the commitment tree changed this
    /// block, then prune anchors older than `shielded_anchor_retention_blocks`.
    ///
    /// Pruning is touch-driven rather than interval-driven like the credit pool's: there can
    /// be many token pools, so scanning all of them every N blocks is avoided; a pool that is
    /// never touched again keeps at most the anchors it accumulated while active, and a pool
    /// that is touched prunes as part of the same write. The same "always keep the newest"
    /// rule applies, so an idle pool always has one valid anchor to spend against.
    pub(super) fn record_token_shielded_pool_anchors_v0(
        &self,
        token_ids: &BTreeSet<[u8; 32]>,
        block_height: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let retention_blocks = platform_version
            .drive_abci
            .validation_and_processing
            .event_constants
            .shielded_anchor_retention_blocks;

        for token_id in token_ids {
            self.drive
                .record_token_shielded_pool_anchor_if_changed(
                    *token_id,
                    block_height,
                    transaction,
                    platform_version,
                )
                .map_err(Error::Drive)?;

            if block_height > retention_blocks {
                self.drive
                    .prune_token_shielded_pool_anchors(
                        *token_id,
                        block_height - retention_blocks,
                        transaction,
                        platform_version,
                    )
                    .map_err(Error::Drive)?;
            }
        }

        Ok(())
    }
}
