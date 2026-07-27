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

#[cfg(test)]
mod tests {
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::version::PlatformVersion;

    /// On a fresh genesis state with no shielded pool activity, calling
    /// `record_shielded_pool_anchor_if_changed_v0` must be a no-op that succeeds —
    /// the drive helper must detect that the commitment tree did not change.
    #[test]
    fn v0_ok_on_fresh_state() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let transaction = platform.drive.grove.start_transaction();

        platform
            .record_shielded_pool_anchor_if_changed_v0(1, &transaction, platform_version)
            .expect("must succeed on fresh state");
    }

    /// The wrapper must be idempotent — calling it repeatedly with the same height
    /// (or varied heights) on an unchanging commitment tree must not fail.
    #[test]
    fn v0_idempotent_across_heights_when_tree_unchanged() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let transaction = platform.drive.grove.start_transaction();

        for h in [1u64, 2, 3, 100, 1_000_000] {
            platform
                .record_shielded_pool_anchor_if_changed_v0(h, &transaction, platform_version)
                .expect("must be idempotent when tree unchanged");
        }
    }

    /// Height = 0 (genesis) must be accepted without panic/error.
    #[test]
    fn v0_accepts_height_zero() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let transaction = platform.drive.grove.start_transaction();

        platform
            .record_shielded_pool_anchor_if_changed_v0(0, &transaction, platform_version)
            .expect("must accept block_height = 0");
    }

    /// `u64::MAX` as block height must not overflow inside the wrapper (which
    /// does no arithmetic of its own).
    #[test]
    fn v0_accepts_max_height() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let transaction = platform.drive.grove.start_transaction();

        platform
            .record_shielded_pool_anchor_if_changed_v0(u64::MAX, &transaction, platform_version)
            .expect("must accept u64::MAX without overflow");
    }
}
