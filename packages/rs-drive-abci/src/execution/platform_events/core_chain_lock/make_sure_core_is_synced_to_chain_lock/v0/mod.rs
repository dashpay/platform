use crate::error::Error;
use crate::execution::platform_events::core_chain_lock::make_sure_core_is_synced_to_chain_lock::CoreSyncStatus;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::ChainLock;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// The point of this call is to make sure core is synced.
    /// Before this call we had previously validated that the chain lock is valid.
    pub(super) fn make_sure_core_is_synced_to_chain_lock_v0(
        &self,
        chain_lock: &ChainLock,
        platform_version: &PlatformVersion,
    ) -> Result<CoreSyncStatus, Error> {
        let given_chain_lock_height = chain_lock.block_height;
        // We need to make sure core is synced to the core height we see as valid for the state transitions

        // TODO: submit_chain_lock responds with invalid signature. We should handle it properly and return CoreSyncStatus
        let best_chain_locked_height = self.core_rpc.submit_chain_lock(chain_lock)?;
        Ok(if best_chain_locked_height >= given_chain_lock_height {
            CoreSyncStatus::Done
        } else if given_chain_lock_height - best_chain_locked_height
            <= platform_version
                .drive_abci
                .methods
                .core_chain_lock
                .recent_block_count_amount
        {
            CoreSyncStatus::Almost
        } else {
            CoreSyncStatus::Not
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{BlockHash, ChainLock};

    fn chain_lock_at(height: u32) -> ChainLock {
        ChainLock {
            block_height: height,
            block_hash: BlockHash::from_byte_array([0u8; 32]),
            signature: [0u8; 96].into(),
        }
    }

    fn run(best: u32, given: u32) -> CoreSyncStatus {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();
        let mut mock_rpc = MockCoreRPCLike::new();
        mock_rpc
            .expect_submit_chain_lock()
            .returning(move |_| Ok(best));
        platform.core_rpc = mock_rpc;

        let platform_version = PlatformVersion::latest();
        platform
            .platform
            .make_sure_core_is_synced_to_chain_lock_v0(&chain_lock_at(given), platform_version)
            .expect("should succeed")
    }

    #[test]
    fn returns_done_when_core_caught_up() {
        assert!(matches!(run(100, 100), CoreSyncStatus::Done));
        assert!(matches!(run(101, 100), CoreSyncStatus::Done));
    }

    #[test]
    fn returns_almost_when_core_is_slightly_behind() {
        // recent_block_count_amount is 2 in the latest version.
        // With the pre-fix code, `best - given` would underflow here:
        // debug -> panic, release -> wraparound to near u32::MAX, falsely yielding `Not`.
        assert!(matches!(run(99, 100), CoreSyncStatus::Almost));
        assert!(matches!(run(98, 100), CoreSyncStatus::Almost));
    }

    #[test]
    fn returns_not_when_core_is_far_behind() {
        assert!(matches!(run(97, 100), CoreSyncStatus::Not));
        assert!(matches!(run(0, 1000), CoreSyncStatus::Not));
    }

    /// Verifies that when `submit_chain_lock` returns an RPC error the
    /// function propagates it as `Err` (not silently swallowed as `Ok`).
    /// This exercises the `?` propagation path in the v0 wrapper.
    #[test]
    fn propagates_rpc_error_from_submit_chain_lock() {
        use dpp::dashcore_rpc::Error as DashCoreRpcError;

        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();
        let mut mock_rpc = MockCoreRPCLike::new();
        mock_rpc.expect_submit_chain_lock().returning(|_| {
            Err(DashCoreRpcError::UnexpectedStructure(
                "simulated rpc failure".to_string(),
            ))
        });
        platform.core_rpc = mock_rpc;

        let platform_version = PlatformVersion::latest();
        let result = platform
            .platform
            .make_sure_core_is_synced_to_chain_lock_v0(&chain_lock_at(100), platform_version);

        assert!(
            result.is_err(),
            "RPC errors must propagate, not be swallowed"
        );
    }
}
