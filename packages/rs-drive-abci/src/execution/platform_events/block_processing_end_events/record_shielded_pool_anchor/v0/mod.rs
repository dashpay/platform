use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Records the current shielded pool anchor if the commitment tree changed this block.
    ///
    /// Delegates to `Drive::record_shielded_pool_anchor_if_changed` which handles
    /// all GroveDB operations: reading the current and most recent anchors, and
    /// conditionally writing to the anchors tree, anchors-by-height tree, and
    /// most recent anchor item.
    pub(super) fn record_shielded_pool_anchor_if_changed_v0(
        &self,
        block_height: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.drive
            .record_shielded_pool_anchor_if_changed(block_height, transaction, platform_version)
            .map_err(Error::Drive)
    }
}
