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

    // Verify Tenderdash that it called this handler correctly
    if let Some(block_execution_context) = block_execution_context_guard.as_ref() {
        if block_execution_context
            .block_state_info()
            .matches_current_block(height as u64, round as u32, block_hash.clone())?
        {
            // Extend votes with unsigned withdrawal transactions
            // we only want to sign the hash of the transaction
            let vote_extensions = block_execution_context
                .unsigned_withdrawal_transactions()
                .into();

            return Ok(proto::ResponseExtendVote { vote_extensions });
        }
    }

    // Tenderdash signs again a block it locked in an earlier round without processing it in
    // this round. That round's proposal has replaced the block execution context meanwhile, or
    // left none when it was rejected before execution. A block's withdrawal transactions do not
    // depend on the round, so sign the ones we built when we accepted it.
    if let Some(vote_extensions) = app
        .unsigned_withdrawal_txs_by_round()
        .read()
        .expect("poisoned only after a panic, which stops the node")
        .get(height as u64, round as u32, &block_hash)
    {
        return Ok(proto::ResponseExtendVote {
            vote_extensions: vote_extensions.to_vec(),
        });
    }

    let block_execution_context =
        block_execution_context_guard
            .as_ref()
            .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "block execution context must be set in block begin handler for extend votes",
            )))?;
    let block_state_info = &block_execution_context.block_state_info();

    Err(AbciError::RequestForWrongBlockReceived(format!(
        "received extend votes request for height: {} round: {}, block: {};  expected height: {} round: {}, block: {}",
        height, round, hex::encode(block_hash),
        block_state_info.height(), block_state_info.round(), block_state_info.block_hash().map(hex::encode).unwrap_or("None".to_string())
    )).into())
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
    use crate::test::helpers::withdrawals::unsigned_withdrawal_transactions;
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
        let err_string = result.unwrap_err().to_string();
        assert!(
            err_string.contains("request does not match current block"),
            "Expected wrong block error, got: {}",
            err_string
        );
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

    /// Tenderdash signs a block it locked in an earlier round again in a later round, without
    /// processing it there: the withdrawals kept for that block are signed.
    #[test]
    fn should_sign_a_block_accepted_in_an_earlier_round_with_its_kept_withdrawals() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        let context =
            make_test_block_execution_context(10, 0, Some([0xAA; 32]), &platform.platform);
        app.block_execution_context
            .write()
            .unwrap()
            .replace(context);

        let kept_extensions: Vec<proto::ExtendVoteExtension> =
            (&unsigned_withdrawal_transactions(1000)).into();
        app.unsigned_withdrawal_txs_by_round
            .write()
            .unwrap()
            .insert(10, 0, [0xAA; 32], kept_extensions.clone());

        let response = extend_vote::<_, MockCoreRPCLike>(
            &app,
            proto::RequestExtendVote {
                hash: vec![0xAA; 32],
                height: 10,
                round: 1,
            },
        )
        .expect("extend_vote should sign the kept withdrawals");
        assert_eq!(response.vote_extensions, kept_extensions);

        let result = extend_vote::<_, MockCoreRPCLike>(
            &app,
            proto::RequestExtendVote {
                hash: vec![0xBB; 32],
                height: 10,
                round: 1,
            },
        );
        assert!(
            result.is_err(),
            "a block this node has not accepted must not be signed"
        );
    }

    /// A later round's proposal rejected before execution leaves no block execution context, and
    /// Tenderdash can still sign the block it locked in an earlier round.
    #[test]
    fn should_sign_a_block_accepted_in_an_earlier_round_without_a_block_execution_context() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        let kept_extensions: Vec<proto::ExtendVoteExtension> =
            (&unsigned_withdrawal_transactions(1000)).into();
        app.unsigned_withdrawal_txs_by_round
            .write()
            .unwrap()
            .insert(10, 0, [0xAA; 32], kept_extensions.clone());

        let response = extend_vote::<_, MockCoreRPCLike>(
            &app,
            proto::RequestExtendVote {
                hash: vec![0xAA; 32],
                height: 10,
                round: 1,
            },
        )
        .expect("extend_vote should sign the kept withdrawals");
        assert_eq!(response.vote_extensions, kept_extensions);

        let result = extend_vote::<_, MockCoreRPCLike>(
            &app,
            proto::RequestExtendVote {
                hash: vec![0xBB; 32],
                height: 10,
                round: 1,
            },
        );
        assert!(
            result.is_err(),
            "a block this node has not accepted must not be signed"
        );
    }
}
