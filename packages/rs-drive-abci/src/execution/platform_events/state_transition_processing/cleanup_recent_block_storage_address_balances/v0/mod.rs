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
    /// Version 0 implementation of cleaning up expired compacted address balance entries.
    ///
    /// Calls the drive layer to remove compacted entries that have expired.
    pub(super) fn cleanup_recent_block_storage_address_balances_v0(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.drive.cleanup_expired_address_balances(
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
    fn test_cleanup_with_no_expired_entries() {
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

        // Should succeed even when there are no expired entries to clean up
        let result = platform.cleanup_recent_block_storage_address_balances_v0(
            &block_info,
            &transaction,
            platform_version,
        );

        assert!(result.is_ok());
    }
}
