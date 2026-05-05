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

#[cfg(test)]
mod tests {
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::version::PlatformVersion;

    /// Returns `(retention_blocks, pruning_interval)` from the version's
    /// event_constants so tests derive branch-entry/branch-skip heights from
    /// the live configuration rather than hardcoded literals.
    fn shielded_anchor_pruning_parameters(platform_version: &PlatformVersion) -> (u64, u64) {
        let event_constants = &platform_version
            .drive_abci
            .validation_and_processing
            .event_constants;
        (
            event_constants.shielded_anchor_retention_blocks,
            event_constants.shielded_anchor_pruning_interval,
        )
    }

    /// Smallest block height that satisfies both
    /// `block_height.is_multiple_of(pruning_interval)` AND
    /// `block_height > retention_blocks`, so that the delegated prune path is
    /// reached.
    fn block_height_on_pruning_boundary_after_retention(platform_version: &PlatformVersion) -> u64 {
        let (retention_blocks, pruning_interval) =
            shielded_anchor_pruning_parameters(platform_version);
        assert!(pruning_interval > 0, "pruning interval must be non-zero");
        ((retention_blocks / pruning_interval) + 1) * pruning_interval
    }

    /// When `block_height` is not a multiple of `pruning_interval`, the method
    /// must return `Ok(())` without touching storage.
    #[test]
    fn v0_noop_when_not_on_pruning_interval_boundary() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let transaction = platform.drive.grove.start_transaction();

        let (_, pruning_interval) = shielded_anchor_pruning_parameters(platform_version);
        assert!(
            pruning_interval > 1,
            "this test requires a pruning_interval > 1 so a non-boundary height exists"
        );
        // `pruning_interval - 1` is guaranteed not to be a multiple of
        // `pruning_interval`, so the early-return branch fires.
        let block_height = pruning_interval - 1;

        platform
            .prune_shielded_pool_anchors_v0(block_height, &transaction, platform_version)
            .expect("prune must succeed off the pruning interval boundary");
    }

    /// When `block_height <= retention_blocks`, the method returns early (no cutoff
    /// to compute). We pick `block_height = 0` which always satisfies this for
    /// any non-zero retention value.
    #[test]
    fn v0_noop_when_below_retention_depth() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let transaction = platform.drive.grove.start_transaction();

        platform
            .prune_shielded_pool_anchors_v0(0, &transaction, platform_version)
            .expect("prune must succeed at height 0");
    }

    /// Calling the pruner at the first boundary past the retention window on an
    /// empty state should succeed — there is simply nothing to remove, and the
    /// delegated drive method is expected to treat this as a no-op.
    #[test]
    fn v0_large_height_on_empty_state_is_ok() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let transaction = platform.drive.grove.start_transaction();

        // Derived from `event_constants` so the prune branch is actually
        // exercised regardless of how the version constants drift.
        let block_height = block_height_on_pruning_boundary_after_retention(platform_version);

        platform
            .prune_shielded_pool_anchors_v0(block_height, &transaction, platform_version)
            .expect("prune must succeed on empty state past retention");
    }

    /// Pruning is idempotent on an empty state: calling it twice must not panic
    /// or return an error.
    #[test]
    fn v0_idempotent_on_empty_state() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let transaction = platform.drive.grove.start_transaction();

        let block_height = block_height_on_pruning_boundary_after_retention(platform_version);

        for _ in 0..3 {
            platform
                .prune_shielded_pool_anchors_v0(block_height, &transaction, platform_version)
                .expect("prune must be idempotent");
        }
    }
}
