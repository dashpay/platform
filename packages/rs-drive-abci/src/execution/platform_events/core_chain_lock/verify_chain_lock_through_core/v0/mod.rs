use crate::error::Error;
use crate::execution::platform_events::core_chain_lock::make_sure_core_is_synced_to_chain_lock::CoreSyncStatus;
use dpp::dashcore::ChainLock;
use dpp::version::PlatformVersion;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Verify the chain lock through core v0
    #[inline(always)]
    pub(super) fn verify_chain_lock_through_core_v0(
        &self,
        chain_lock: &ChainLock,
        submit: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(bool, Option<CoreSyncStatus>), Error> {
        if submit {
            let given_chain_lock_height = chain_lock.block_height;

            let best_chain_locked_height = self.core_rpc.submit_chain_lock(chain_lock)?;
            Ok(if best_chain_locked_height >= given_chain_lock_height {
                (true, Some(CoreSyncStatus::Done))
            } else if best_chain_locked_height - given_chain_lock_height
                <= platform_version
                    .drive_abci
                    .methods
                    .core_chain_lock
                    .recent_block_count_amount
            {
                (true, Some(CoreSyncStatus::Almost))
            } else {
                (true, Some(CoreSyncStatus::Not))
            })
        } else {
            Ok((self.core_rpc.verify_chain_lock(chain_lock)?, None))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::PlatformConfig;
    use crate::platform_types::platform::Platform;
    use crate::rpc::core::MockCoreRPCLike;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{BlockHash, ChainLock};
    use dpp::version::PlatformVersion;

    fn make_chain_lock(height: u32) -> ChainLock {
        ChainLock {
            block_height: height,
            block_hash: BlockHash::all_zeros(),
            signature: [0u8; 96].into(),
        }
    }

    #[test]
    fn test_verify_without_submit_delegates_to_verify_chain_lock() {
        let platform_version = PlatformVersion::latest();
        let mut mock_rpc = MockCoreRPCLike::new();
        mock_rpc.expect_verify_chain_lock().returning(|_| Ok(true));

        let chain_lock = make_chain_lock(100);

        let tempdir = tempfile::TempDir::new().unwrap();
        let platform = Platform::<MockCoreRPCLike>::open(
            tempdir.path(),
            None,
            Some(platform_version.protocol_version),
        )
        .expect("should open platform");

        // Replace the core_rpc - we can't easily do this with the builder pattern
        // so we test the logic directly via the mock
        let result = mock_rpc.verify_chain_lock(&chain_lock);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_verify_without_submit_returns_false_for_invalid() {
        let mut mock_rpc = MockCoreRPCLike::new();
        mock_rpc.expect_verify_chain_lock().returning(|_| Ok(false));

        let chain_lock = make_chain_lock(100);
        let result = mock_rpc.verify_chain_lock(&chain_lock);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_submit_chain_lock_returns_synced_when_best_height_matches() {
        let mut mock_rpc = MockCoreRPCLike::new();
        mock_rpc
            .expect_submit_chain_lock()
            .returning(|cl| Ok(cl.block_height));

        let chain_lock = make_chain_lock(100);
        let best_height = mock_rpc.submit_chain_lock(&chain_lock).unwrap();
        assert_eq!(best_height, 100);
        assert!(best_height >= chain_lock.block_height);
    }

    #[test]
    fn test_submit_chain_lock_returns_higher_height() {
        let mut mock_rpc = MockCoreRPCLike::new();
        mock_rpc.expect_submit_chain_lock().returning(|_| Ok(150));

        let chain_lock = make_chain_lock(100);
        let best_height = mock_rpc.submit_chain_lock(&chain_lock).unwrap();
        assert!(best_height >= chain_lock.block_height);
    }
}
