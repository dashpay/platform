use crate::abci::app::{BlockExecutionApplication, PlatformApplication};
use crate::error::Error;
use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
use crate::execution::types::block_state_info::v0::BlockStateInfoV0Getters;
use crate::rpc::core::CoreRPCLike;
use tenderdash_abci::proto::abci as proto;
use tenderdash_abci::proto::abci::response_verify_vote_extension::VerifyStatus;
use tenderdash_abci::proto::abci::ExtendVoteExtension;

/// Todo: Verify votes extension not really needed because extend votes is deterministic
pub fn verify_vote_extension<A, C>(
    app: &A,
    request: proto::RequestVerifyVoteExtension,
) -> Result<proto::ResponseVerifyVoteExtension, Error>
where
    A: PlatformApplication<C> + BlockExecutionApplication,
    C: CoreRPCLike,
{
    let _timer = crate::metrics::abci_request_duration("verify_vote_extension");

    // Verify that this is a votes extension for our current executed block and our proposer
    let proto::RequestVerifyVoteExtension {
        height,
        round,
        vote_extensions,
        ..
    } = request;

    let height: u64 = height as u64;
    let round: u32 = round as u32;

    // Make sure we are in a block execution phase
    let block_execution_context_ref = app.block_execution_context().read().unwrap();
    let Some(block_execution_context) = block_execution_context_ref.as_ref() else {
        tracing::warn!(
                "votes extensions for height: {}, round: {} are rejected because we are not in a block execution phase",
                height,
                round,
            );

        return Ok(proto::ResponseVerifyVoteExtension {
            status: VerifyStatus::Reject.into(),
        });
    };

    // Make sure votes extension is for our currently executing block

    let block_state_info = block_execution_context.block_state_info();

    // We might get votes extension to verify for previous (in case if other node is behind)
    // or future round (in case if the current node is behind), so we make sure that only height
    // is matching. It's fine because withdrawal transactions to sign are the same for any round
    // of the same height
    if block_state_info.height() != height {
        tracing::warn!(
            "votes extensions for height: {}, round: {} are rejected because we are at height: {}",
            height,
            round,
            block_state_info.height(),
        );

        return Ok(proto::ResponseVerifyVoteExtension {
            status: VerifyStatus::Reject.into(),
        });
    }

    // Verify that a validator is requesting a signatures
    // for a correct set of withdrawal transactions

    let expected_withdrawals = block_execution_context.unsigned_withdrawal_transactions();

    if expected_withdrawals != vote_extensions.as_slice() {
        let expected_extensions: Vec<ExtendVoteExtension> = expected_withdrawals.into();

        tracing::error!(
            received_extensions = ?vote_extensions,
            ?expected_extensions,
            "votes extensions for height: {}, round: {} mismatch",
            height, round
        );

        return Ok(proto::ResponseVerifyVoteExtension {
            status: VerifyStatus::Reject.into(),
        });
    }

    tracing::trace!(
        "votes extensions for height: {}, round: {} are successfully verified",
        height,
        round,
    );

    Ok(proto::ResponseVerifyVoteExtension {
        status: VerifyStatus::Accept.into(),
    })
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
    fn verify_vote_extension_rejects_when_no_block_execution_context() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        // No block execution context is set
        let request = proto::RequestVerifyVoteExtension {
            hash: vec![0u8; 32],
            validator_pro_tx_hash: vec![0u8; 32],
            height: 10,
            round: 0,
            vote_extensions: vec![],
        };

        let response = verify_vote_extension::<_, MockCoreRPCLike>(&app, request)
            .expect("should return Ok with Reject status");

        assert_eq!(response.status, VerifyStatus::Reject as i32);
    }

    #[test]
    fn verify_vote_extension_rejects_when_height_mismatch() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        // Set block execution context at height 10
        let context =
            make_test_block_execution_context(10, 0, Some([0xAA; 32]), &platform.platform);
        app.block_execution_context
            .write()
            .unwrap()
            .replace(context);

        // Request verification for a different height (20)
        let request = proto::RequestVerifyVoteExtension {
            hash: vec![0u8; 32],
            validator_pro_tx_hash: vec![0u8; 32],
            height: 20,
            round: 0,
            vote_extensions: vec![],
        };

        let response = verify_vote_extension::<_, MockCoreRPCLike>(&app, request)
            .expect("should return Ok with Reject status");

        assert_eq!(response.status, VerifyStatus::Reject as i32);
    }

    #[test]
    fn verify_vote_extension_accepts_matching_empty_withdrawals() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        // Set block execution context at height 10, with empty withdrawal transactions
        let context =
            make_test_block_execution_context(10, 0, Some([0xAA; 32]), &platform.platform);
        app.block_execution_context
            .write()
            .unwrap()
            .replace(context);

        // Request with matching empty vote extensions
        let request = proto::RequestVerifyVoteExtension {
            hash: vec![0u8; 32],
            validator_pro_tx_hash: vec![0u8; 32],
            height: 10,
            round: 0,
            vote_extensions: vec![],
        };

        let response = verify_vote_extension::<_, MockCoreRPCLike>(&app, request)
            .expect("should return Ok with Accept status");

        assert_eq!(response.status, VerifyStatus::Accept as i32);
    }

    #[test]
    fn verify_vote_extension_rejects_mismatched_withdrawals() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        // Set block execution context at height 10 with empty withdrawal transactions
        let context =
            make_test_block_execution_context(10, 0, Some([0xAA; 32]), &platform.platform);
        app.block_execution_context
            .write()
            .unwrap()
            .replace(context);

        // Request with non-empty vote extensions (mismatch)
        let request = proto::RequestVerifyVoteExtension {
            hash: vec![0u8; 32],
            validator_pro_tx_hash: vec![0u8; 32],
            height: 10,
            round: 0,
            vote_extensions: vec![proto::ExtendVoteExtension {
                r#type: 0,
                extension: vec![1, 2, 3],
                sign_request_id: None,
            }],
        };

        let response = verify_vote_extension::<_, MockCoreRPCLike>(&app, request)
            .expect("should return Ok with Reject status");

        assert_eq!(response.status, VerifyStatus::Reject as i32);
    }

    #[test]
    fn verify_vote_extension_accepts_different_round_same_height() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        // Set block execution context at height 10, round 1
        let context =
            make_test_block_execution_context(10, 1, Some([0xAA; 32]), &platform.platform);
        app.block_execution_context
            .write()
            .unwrap()
            .replace(context);

        // Request for same height but different round (round 5)
        // This should still be accepted since only height needs to match
        let request = proto::RequestVerifyVoteExtension {
            hash: vec![0u8; 32],
            validator_pro_tx_hash: vec![0u8; 32],
            height: 10,
            round: 5,
            vote_extensions: vec![],
        };

        let response = verify_vote_extension::<_, MockCoreRPCLike>(&app, request)
            .expect("should return Ok with Accept status");

        assert_eq!(response.status, VerifyStatus::Accept as i32);
    }
}
