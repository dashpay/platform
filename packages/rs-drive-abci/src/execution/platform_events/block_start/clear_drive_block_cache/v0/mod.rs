use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Clears the drive cache at the start of block processing. This does a few things like clearing
    /// the block data contract cache and the block platform versions cache.
    ///
    #[inline(always)]
    pub(super) fn clear_drive_block_cache_v0(&self) {
        self.drive.cache.data_contracts.clear_block_cache();

        let mut protocol_versions_counter = self.drive.cache.protocol_versions_counter.write();

        protocol_versions_counter.clear_block_cache();
        // Getter is disabled in case of epoch change so we need to enable it back
        // For more information read comments in `upgrade_protocol_version_v0` function
        protocol_versions_counter.unblock_global_cache();
    }
}

#[cfg(test)]
mod tests {
    use crate::test::helpers::setup::TestPlatformBuilder;

    /// `clear_drive_block_cache_v0` on a fresh platform must succeed without panicking,
    /// even when the caches are empty and the global cache is already unblocked.
    #[test]
    fn v0_noop_on_fresh_platform() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        platform.clear_drive_block_cache_v0();
    }

    /// The operation is idempotent: calling it multiple times in a row must not
    /// panic or leave the cache in a poisoned state.
    #[test]
    fn v0_idempotent_across_repeated_calls() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        for _ in 0..5 {
            platform.clear_drive_block_cache_v0();
        }
    }

    /// After `clear_drive_block_cache_v0`, the protocol_versions_counter's
    /// global cache must be unblocked (ready for reads). We exercise this by
    /// calling the method and then confirming a subsequent call continues to
    /// succeed — which it can't do if the write lock became poisoned.
    #[test]
    fn v0_leaves_cache_usable_after_call() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        platform.clear_drive_block_cache_v0();
        // If the write lock were poisoned the second call would panic.
        platform.clear_drive_block_cache_v0();

        // Confirm we can still take a read lock on the counter afterwards.
        let _counter = platform.drive.cache.protocol_versions_counter.read();
    }
}
