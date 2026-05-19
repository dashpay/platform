use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Version 0 implementation of cleaning up expired compacted nullifier entries.
    pub(super) fn cleanup_recent_block_storage_nullifiers_v0(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.drive.cleanup_expired_nullifier_compactions(
            block_info.time_ms,
            Some(transaction),
            platform_version,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_cleanup_nullifiers_with_no_expired_entries() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 1,
            core_height: 1,
            epoch: Epoch::default(),
        };

        let result = platform.cleanup_recent_block_storage_nullifiers_v0(
            &block_info,
            &transaction,
            platform_version,
        );

        assert!(result.is_ok());
    }

    /// Nullifier cleanup must be idempotent on an empty state.
    #[test]
    fn test_cleanup_nullifiers_idempotent_on_empty_state() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 5_000,
            height: 10,
            core_height: 10,
            epoch: Epoch::default(),
        };

        for _ in 0..3 {
            platform
                .cleanup_recent_block_storage_nullifiers_v0(
                    &block_info,
                    &transaction,
                    platform_version,
                )
                .expect("nullifier cleanup must be idempotent on empty state");
        }
    }

    /// Zero `time_ms` must be accepted and yield no error: nothing can have
    /// expired "before time zero", so this is a safe no-op.
    #[test]
    fn test_cleanup_nullifiers_with_zero_time_ms() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 0,
            height: 1,
            core_height: 1,
            epoch: Epoch::default(),
        };

        platform
            .cleanup_recent_block_storage_nullifiers_v0(&block_info, &transaction, platform_version)
            .expect("zero time_ms must be accepted");
    }
}
