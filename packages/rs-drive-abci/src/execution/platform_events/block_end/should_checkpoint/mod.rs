mod v0;

pub use v0::CheckpointNeededInfo;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Determines whether a checkpoint should be created for the current block.
    ///
    /// Returns `Ok(Some(CheckpointNeededInfo))` if a checkpoint should be created,
    /// `Ok(None)` if no checkpoint is needed.
    pub fn should_checkpoint(
        &self,
        block_execution_context: &BlockExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<Option<CheckpointNeededInfo>, Error> {
        match platform_version
            .drive_abci
            .methods
            .block_end
            .should_checkpoint
        {
            None => Ok(None),
            Some(0) => self.should_checkpoint_v0(block_execution_context, platform_version),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "should_checkpoint".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::setup::TestPlatformBuilder;

    #[test]
    fn test_dispatcher_none_version_returns_none() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut modified_version = platform_version.clone();
        modified_version
            .drive_abci
            .methods
            .block_end
            .should_checkpoint = None;

        use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0;
        use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
        use crate::execution::types::block_state_info::BlockStateInfo;
        use crate::platform_types::epoch_info::v0::EpochInfoV0;
        use crate::platform_types::epoch_info::EpochInfo;
        use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
        use std::collections::BTreeMap;

        let platform_state = platform.state.load();
        let block_platform_state = platform_state.as_ref().clone();

        let block_execution_context = BlockExecutionContext::V0(BlockExecutionContextV0 {
            block_state_info: BlockStateInfo::V0(BlockStateInfoV0 {
                height: 1,
                round: 0,
                block_time_ms: 1_000_000,
                previous_block_time_ms: None,
                proposer_pro_tx_hash: [0u8; 32],
                core_chain_locked_height: 1,
                block_hash: None,
                app_hash: None,
            }),
            epoch_info: EpochInfo::V0(EpochInfoV0 {
                current_epoch_index: 0,
                previous_epoch_index: None,
                is_epoch_change: false,
            }),
            unsigned_withdrawal_transactions: UnsignedWithdrawalTxs::default(),
            block_address_balance_changes: BTreeMap::new(),
            block_platform_state,
            proposer_results: None,
        });

        // When should_checkpoint version is None, the dispatcher returns Ok(None) directly
        let result = platform
            .should_checkpoint(&block_execution_context, &modified_version)
            .expect("expected Ok");
        assert!(result.is_none(), "expected None when version is None");
    }

    #[test]
    fn test_dispatcher_unknown_version_returns_error() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut modified_version = platform_version.clone();
        modified_version
            .drive_abci
            .methods
            .block_end
            .should_checkpoint = Some(255);

        use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0;
        use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
        use crate::execution::types::block_state_info::BlockStateInfo;
        use crate::platform_types::epoch_info::v0::EpochInfoV0;
        use crate::platform_types::epoch_info::EpochInfo;
        use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
        use std::collections::BTreeMap;

        let platform_state = platform.state.load();
        let block_platform_state = platform_state.as_ref().clone();

        let block_execution_context = BlockExecutionContext::V0(BlockExecutionContextV0 {
            block_state_info: BlockStateInfo::V0(BlockStateInfoV0 {
                height: 1,
                round: 0,
                block_time_ms: 1_000_000,
                previous_block_time_ms: None,
                proposer_pro_tx_hash: [0u8; 32],
                core_chain_locked_height: 1,
                block_hash: None,
                app_hash: None,
            }),
            epoch_info: EpochInfo::V0(EpochInfoV0 {
                current_epoch_index: 0,
                previous_epoch_index: None,
                is_epoch_change: false,
            }),
            unsigned_withdrawal_transactions: UnsignedWithdrawalTxs::default(),
            block_address_balance_changes: BTreeMap::new(),
            block_platform_state,
            proposer_results: None,
        });

        let result = platform.should_checkpoint(&block_execution_context, &modified_version);

        assert!(result.is_err());
        match result {
            Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            })) => {
                assert_eq!(method, "should_checkpoint");
                assert_eq!(known_versions, vec![0]);
                assert_eq!(received, 255);
            }
            _ => panic!("expected UnknownVersionMismatch error"),
        }
    }
}
