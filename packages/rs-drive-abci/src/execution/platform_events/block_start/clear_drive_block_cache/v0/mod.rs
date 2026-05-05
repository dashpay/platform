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

    /// After `clear_drive_block_cache_v0`, the `protocol_versions_counter`'s
    /// global cache must be unblocked (ready for reads). We prove this directly
    /// by first blocking the global cache, observing that a read returns the
    /// `GlobalCacheIsBlocked` error, then calling `clear_drive_block_cache_v0`
    /// and observing that the same read now succeeds.
    #[test]
    fn v0_unblocks_globally_blocked_protocol_versions_cache() {
        use drive::error::cache::CacheError;
        use drive::error::Error as DriveError;

        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        // Block the global cache so the read path would normally error.
        platform
            .drive
            .cache
            .protocol_versions_counter
            .write()
            .block_global_cache();

        // Sanity check: while blocked, `get` must return GlobalCacheIsBlocked.
        // Scope the read guard so it stays alive as long as the borrowed
        // `Option<&u64>` inside the Result.
        {
            let guard = platform.drive.cache.protocol_versions_counter.read();
            let blocked_result = guard.get(&1u32);
            assert!(
                matches!(
                    &blocked_result,
                    Err(DriveError::Cache(CacheError::GlobalCacheIsBlocked))
                ),
                "precondition: blocked cache must return GlobalCacheIsBlocked, got {:?}",
                blocked_result
            );
        }

        // Now clear the block cache — this must also unblock the global cache.
        platform.clear_drive_block_cache_v0();

        // Post-condition: `get` must now succeed (returning Ok regardless of
        // whether the key is present) — any Err would mean the cache is still
        // blocked.
        {
            let guard = platform.drive.cache.protocol_versions_counter.read();
            let unblocked_result = guard.get(&1u32);
            assert!(
                unblocked_result.is_ok(),
                "clear_drive_block_cache_v0 must leave the global cache unblocked, got {:?}",
                unblocked_result
            );
        }
    }

    /// After `clear_drive_block_cache_v0`, the `protocol_versions_counter`
    /// write lock must still be usable (not poisoned) and further reads must
    /// succeed. Complements the direct-unblock test above.
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
