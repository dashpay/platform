use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::prelude::{CoreBlockHeight, TimestampMillis};
use std::time::{SystemTime, UNIX_EPOCH};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Determine initial core height.
    ///
    /// Use core height received from Tenderdash (from genesis.json) by default,
    /// otherwise we go with height of mn_rr fork.
    ///
    /// Core height is verified to ensure that it is both at or after mn_rr fork, and
    /// before or at last chain lock.
    ///
    /// ## Error handling
    ///
    /// This function will fail if:
    ///
    /// * mn_rr fork is not yet active
    /// * `requested` core height is before mn_rr fork
    /// * `requested` core height is after current best chain lock
    ///
    pub(in crate::execution::platform_events) fn initial_core_height_and_time_v0(
        &self,
        requested: Option<u32>,
    ) -> Result<(CoreBlockHeight, TimestampMillis), Error> {
        let fork_info = self.core_rpc.get_fork_info("mn_rr")?.ok_or(
            ExecutionError::InitializationForkNotActive("fork is not yet known".to_string()),
        )?;
        if !fork_info.active || fork_info.height.is_none() {
            // fork is not good yet
            return Err(ExecutionError::InitializationForkNotActive(format!(
                "fork is not yet known (currently {:?})",
                fork_info
            ))
            .into());
        } else {
            tracing::debug!(?fork_info, "core fork mn_rr is active");
        };
        // We expect height to present if the fork is active
        let mn_rr_fork_height = fork_info.height.unwrap();

        let initial_height = if let Some(requested) = requested {
            tracing::debug!(
                requested,
                mn_rr_fork_height,
                "initial core lock height is set in genesis"
            );

            requested
        } else {
            tracing::debug!(mn_rr_fork_height, "used fork height as initial core height");

            mn_rr_fork_height
        };

        // Make sure initial height is chain locked
        let chain_lock_height = self.core_rpc.get_best_chain_lock()?.block_height;

        if initial_height <= chain_lock_height {
            let block_time = self.core_rpc.get_block_time_from_height(initial_height)?;

            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards") // Copilot rocks :))
                .as_millis() as TimestampMillis;

            if block_time > current_time {
                return Err(ExecutionError::InitializationGenesisTimeInFuture {
                    initial_height,
                    genesis_time: block_time,
                    current_time,
                }
                .into());
            }

            Ok((initial_height, block_time))
        } else {
            Err(ExecutionError::InitializationHeightIsNotLocked {
                initial_height,
                chain_lock_height,
            }
            .into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{BlockHash, ChainLock};
    use dpp::dashcore_rpc::dashcore_rpc_json::{
        Bip9SoftforkInfo, Bip9SoftforkStatus, SoftforkInfo, SoftforkType,
    };

    fn active_fork(height: u32) -> SoftforkInfo {
        SoftforkInfo {
            softfork_type: SoftforkType::Bip9,
            active: true,
            height: Some(height),
            bip9: Some(Bip9SoftforkInfo {
                status: Bip9SoftforkStatus::Active,
                bit: None,
                start_time: 0,
                timeout: 0,
                since: height,
                statistics: None,
            }),
        }
    }

    fn inactive_fork() -> SoftforkInfo {
        SoftforkInfo {
            softfork_type: SoftforkType::Bip9,
            active: false,
            height: None,
            bip9: Some(Bip9SoftforkInfo {
                status: Bip9SoftforkStatus::Defined,
                bit: None,
                start_time: 0,
                timeout: 0,
                since: 0,
                statistics: None,
            }),
        }
    }

    fn chain_lock_at(height: u32) -> ChainLock {
        ChainLock {
            block_height: height,
            block_hash: BlockHash::from_byte_array([0u8; 32]),
            signature: [0u8; 96].into(),
        }
    }

    /// If `get_fork_info("mn_rr")` returns `Ok(None)`, the method must yield
    /// `InitializationForkNotActive`.
    #[test]
    fn v0_fork_info_none_returns_fork_not_active() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();
        let mut mock_rpc = MockCoreRPCLike::new();
        mock_rpc.expect_get_fork_info().returning(|_| Ok(None));
        platform.core_rpc = mock_rpc;

        let err = platform
            .platform
            .initial_core_height_and_time_v0(None)
            .expect_err("expected error");

        match err {
            Error::Execution(ExecutionError::InitializationForkNotActive(_)) => {}
            other => panic!("expected InitializationForkNotActive, got {:?}", other),
        }
    }

    /// If the fork exists but is not active, the method must yield
    /// `InitializationForkNotActive`.
    #[test]
    fn v0_fork_inactive_returns_fork_not_active() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();
        let mut mock_rpc = MockCoreRPCLike::new();
        mock_rpc
            .expect_get_fork_info()
            .returning(|_| Ok(Some(inactive_fork())));
        platform.core_rpc = mock_rpc;

        let err = platform
            .platform
            .initial_core_height_and_time_v0(None)
            .expect_err("expected error");

        match err {
            Error::Execution(ExecutionError::InitializationForkNotActive(_)) => {}
            other => panic!("expected InitializationForkNotActive, got {:?}", other),
        }
    }

    /// If `requested` height is AFTER the best chain lock, yield
    /// `InitializationHeightIsNotLocked`.
    #[test]
    fn v0_requested_above_chain_lock_returns_height_not_locked() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();
        let mut mock_rpc = MockCoreRPCLike::new();
        mock_rpc
            .expect_get_fork_info()
            .returning(|_| Ok(Some(active_fork(10))));
        mock_rpc
            .expect_get_best_chain_lock()
            .returning(|| Ok(chain_lock_at(50)));
        platform.core_rpc = mock_rpc;

        let err = platform
            .platform
            .initial_core_height_and_time_v0(Some(100))
            .expect_err("expected error");

        match err {
            Error::Execution(ExecutionError::InitializationHeightIsNotLocked {
                initial_height,
                chain_lock_height,
            }) => {
                assert_eq!(initial_height, 100);
                assert_eq!(chain_lock_height, 50);
            }
            other => panic!("expected InitializationHeightIsNotLocked, got {:?}", other),
        }
    }

    /// Happy path: active fork, no requested height, chain lock far above fork.
    /// Must return `(mn_rr_fork_height, block_time)`.
    #[test]
    fn v0_default_path_returns_fork_height_and_block_time() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();
        let mut mock_rpc = MockCoreRPCLike::new();
        mock_rpc
            .expect_get_fork_info()
            .returning(|_| Ok(Some(active_fork(10))));
        mock_rpc
            .expect_get_best_chain_lock()
            .returning(|| Ok(chain_lock_at(100)));
        mock_rpc
            .expect_get_block_time_from_height()
            .returning(|_| Ok(1_000_000));
        platform.core_rpc = mock_rpc;

        let (height, time) = platform
            .platform
            .initial_core_height_and_time_v0(None)
            .expect("happy path must succeed");

        assert_eq!(height, 10, "must fall back to mn_rr fork height");
        assert_eq!(time, 1_000_000);
    }

    /// When the requested height is EXACTLY equal to the chain lock height, we
    /// must NOT take the error path — the condition is `initial_height <= chain_lock_height`.
    #[test]
    fn v0_requested_equal_to_chain_lock_is_accepted() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();
        let mut mock_rpc = MockCoreRPCLike::new();
        mock_rpc
            .expect_get_fork_info()
            .returning(|_| Ok(Some(active_fork(1))));
        mock_rpc
            .expect_get_best_chain_lock()
            .returning(|| Ok(chain_lock_at(42)));
        mock_rpc
            .expect_get_block_time_from_height()
            .returning(|_| Ok(500));
        platform.core_rpc = mock_rpc;

        let (height, time) = platform
            .platform
            .initial_core_height_and_time_v0(Some(42))
            .expect("equality boundary must succeed");

        assert_eq!(height, 42);
        assert_eq!(time, 500);
    }

    /// Pins the currently-observable behavior when `requested <
    /// mn_rr_fork_height`. The function docstring (lines 24-26) says this
    /// should fail with an error, but the v0 implementation does not enforce
    /// that invariant — it only checks `initial_height <= chain_lock_height`.
    ///
    /// This test locks in the actual behavior as-is so that any future change
    /// (either tightening the implementation or relaxing the docstring) is a
    /// conscious, reviewed decision rather than an accidental regression.
    /// See discussion on PR #3528 for the docs-vs-impl gap.
    #[test]
    fn v0_requested_below_fork_height_is_currently_accepted() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();
        let mut mock_rpc = MockCoreRPCLike::new();
        mock_rpc
            .expect_get_fork_info()
            .returning(|_| Ok(Some(active_fork(10))));
        mock_rpc
            .expect_get_best_chain_lock()
            .returning(|| Ok(chain_lock_at(50)));
        mock_rpc
            .expect_get_block_time_from_height()
            .returning(|_| Ok(1_000));
        platform.core_rpc = mock_rpc;

        // requested = 9 is strictly below the mn_rr fork height (10).
        let result = platform.platform.initial_core_height_and_time_v0(Some(9));

        // Today the implementation accepts this; the docstring suggests it
        // should not. Assert the current observable behavior (acceptance)
        // rather than silently ignore the disagreement.
        assert!(
            result.is_ok(),
            "v0 currently accepts requested < mn_rr_fork_height (see docstring \
             vs. impl gap); update this test when production behavior changes, \
             got: {:?}",
            result
        );
        let (height, _time) = result.unwrap();
        assert_eq!(height, 9);
    }

    /// If the block time returned by core is in the future (> current system time),
    /// `InitializationGenesisTimeInFuture` must be returned.
    #[test]
    fn v0_genesis_time_in_future_returns_dedicated_error() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();
        let mut mock_rpc = MockCoreRPCLike::new();
        mock_rpc
            .expect_get_fork_info()
            .returning(|_| Ok(Some(active_fork(1))));
        mock_rpc
            .expect_get_best_chain_lock()
            .returning(|| Ok(chain_lock_at(50)));
        // Block time ~2100 AD in ms — well into the future.
        mock_rpc
            .expect_get_block_time_from_height()
            .returning(|_| Ok(4_102_444_800_000));
        platform.core_rpc = mock_rpc;

        let err = platform
            .platform
            .initial_core_height_and_time_v0(Some(10))
            .expect_err("expected genesis-time-in-future error");

        match err {
            Error::Execution(ExecutionError::InitializationGenesisTimeInFuture {
                initial_height,
                ..
            }) => {
                assert_eq!(initial_height, 10);
            }
            other => panic!(
                "expected InitializationGenesisTimeInFuture, got {:?}",
                other
            ),
        }
    }
}
