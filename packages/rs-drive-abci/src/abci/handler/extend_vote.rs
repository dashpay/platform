use crate::abci::app::{BlockExecutionApplication, PlatformApplication, TransactionalApplication};
use crate::abci::AbciError;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
use crate::execution::types::block_state_info::v0::{
    BlockStateInfoV0Getters, BlockStateInfoV0Methods,
};
use crate::rpc::core::CoreRPCLike;
use tenderdash_abci::proto::abci as proto;

pub fn extend_vote<'a, A, C>(
    app: &A,
    request: proto::RequestExtendVote,
) -> Result<proto::ResponseExtendVote, Error>
where
    A: PlatformApplication<C> + TransactionalApplication<'a> + BlockExecutionApplication,
    C: CoreRPCLike,
{
    let _timer = crate::metrics::abci_request_duration("extend_vote");

    let proto::RequestExtendVote {
        hash: block_hash,
        height,
        round,
    } = request;
    let block_execution_context_guard = app.block_execution_context().read().unwrap();
    let block_execution_context =
        block_execution_context_guard
            .as_ref()
            .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "block execution context must be set in block begin handler for extend votes",
            )))?;

    // Verify Tenderdash that it called this handler correctly
    let block_state_info = &block_execution_context.block_state_info();

    if !block_state_info.matches_current_block(height as u64, round as u32, block_hash.clone())? {
        return Err(AbciError::RequestForWrongBlockReceived(format!(
            "received extend votes request for height: {} round: {}, block: {};  expected height: {} round: {}, block: {}",
            height, round, hex::encode(block_hash),
            block_state_info.height(), block_state_info.round(), block_state_info.block_hash().map(hex::encode).unwrap_or("None".to_string())
        )).into());
    }

    // Extend votes with unsigned withdrawal transactions
    // we only want to sign the hash of the transaction
    let vote_extensions = block_execution_context
        .unsigned_withdrawal_transactions()
        .into();

    Ok(proto::ResponseExtendVote { vote_extensions })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abci::app::FullAbciApplication;
    use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0;
    use crate::execution::types::block_execution_context::BlockExecutionContext;
    use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
    use crate::execution::types::block_state_info::BlockStateInfo;
    use crate::platform_types::epoch_info::v0::EpochInfoV0;
    use crate::platform_types::epoch_info::EpochInfo;
    use crate::platform_types::platform_state::PlatformState;
    use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn make_test_block_execution_context(
        height: u64,
        round: u32,
        block_hash: Option<[u8; 32]>,
        platform: &crate::platform_types::platform::Platform<MockCoreRPCLike>,
    ) -> BlockExecutionContext {
        let platform_version = PlatformVersion::latest();
        BlockExecutionContext::V0(BlockExecutionContextV0 {
            block_state_info: BlockStateInfo::V0(BlockStateInfoV0 {
                height,
                round,
                block_time_ms: 1_000_000,
                previous_block_time_ms: None,
                proposer_pro_tx_hash: [0u8; 32],
                core_chain_locked_height: 1,
                block_hash,
                app_hash: None,
            }),
            epoch_info: EpochInfo::V0(EpochInfoV0 {
                current_epoch_index: 0,
                previous_epoch_index: None,
                is_epoch_change: false,
            }),
            unsigned_withdrawal_transactions: UnsignedWithdrawalTxs::default(),
            block_address_balance_changes: BTreeMap::new(),
            block_platform_state: PlatformState::default_with_protocol_versions(
                platform_version.protocol_version,
                platform_version.protocol_version,
                &platform.config,
            )
            .expect("should create default platform state"),
            proposer_results: None,
        })
    }

    #[test]
    fn extend_vote_fails_when_no_block_execution_context() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        let request = proto::RequestExtendVote {
            hash: vec![0u8; 32],
            height: 10,
            round: 0,
        };

        let result = extend_vote::<_, MockCoreRPCLike>(&app, request);
        assert!(result.is_err());
        let err_string = result.unwrap_err().to_string();
        assert!(
            err_string.contains("block execution context must be set"),
            "Expected block execution context error, got: {}",
            err_string
        );
    }

    #[test]
    fn extend_vote_fails_when_block_hash_does_not_match() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        // Set block context at height 10, round 0 with specific hash
        let context =
            make_test_block_execution_context(10, 0, Some([0xAA; 32]), &platform.platform);
        app.block_execution_context
            .write()
            .unwrap()
            .replace(context);

        // Request with a different hash
        let request = proto::RequestExtendVote {
            hash: vec![0xBB; 32],
            height: 10,
            round: 0,
        };

        let result = extend_vote::<_, MockCoreRPCLike>(&app, request);
        assert!(result.is_err());
        let err_string = result.unwrap_err().to_string();
        assert!(
            err_string.contains("request does not match current block"),
            "Expected wrong block error, got: {}",
            err_string
        );
    }

    #[test]
    fn extend_vote_fails_when_height_does_not_match() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        // Set block context at height 10, round 0
        let context =
            make_test_block_execution_context(10, 0, Some([0xAA; 32]), &platform.platform);
        app.block_execution_context
            .write()
            .unwrap()
            .replace(context);

        // Request for a different height
        let request = proto::RequestExtendVote {
            hash: vec![0xAA; 32],
            height: 20,
            round: 0,
        };

        let result = extend_vote::<_, MockCoreRPCLike>(&app, request);
        assert!(result.is_err());
    }

    #[test]
    fn extend_vote_succeeds_with_matching_block_and_empty_withdrawals() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        // Set block context at height 10, round 0 with specific hash
        let context =
            make_test_block_execution_context(10, 0, Some([0xAA; 32]), &platform.platform);
        app.block_execution_context
            .write()
            .unwrap()
            .replace(context);

        // Request with matching hash, height, round
        let request = proto::RequestExtendVote {
            hash: vec![0xAA; 32],
            height: 10,
            round: 0,
        };

        let response =
            extend_vote::<_, MockCoreRPCLike>(&app, request).expect("extend_vote should succeed");

        // No withdrawal transactions, so no vote extensions
        assert!(response.vote_extensions.is_empty());
    }
}
