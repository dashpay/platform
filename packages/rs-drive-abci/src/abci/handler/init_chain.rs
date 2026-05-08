use crate::abci::app::{BlockExecutionApplication, PlatformApplication, TransactionalApplication};
use crate::error::Error;
use crate::rpc::core::CoreRPCLike;
use tenderdash_abci::proto::abci as proto;

pub fn init_chain<'a, A, C>(
    app: &A,
    request: proto::RequestInitChain,
) -> Result<proto::ResponseInitChain, Error>
where
    A: PlatformApplication<C> + TransactionalApplication<'a> + BlockExecutionApplication,
    C: CoreRPCLike,
{
    app.start_transaction();

    let transaction_guard = app.transaction().read().unwrap();
    let transaction = transaction_guard
        .as_ref()
        .expect("transaction must be started");

    // We need to drop the block execution context just in case init chain had already been called
    let block_context = app.block_execution_context().write().unwrap().take(); //drop the block execution context
    if block_context.is_some() {
        tracing::warn!("block context was present during init chain, dropping it");
    }

    let chain_id = request.chain_id.to_string();

    let response = app.platform().init_chain(request, transaction)?;

    transaction.set_savepoint();

    let app_hash = hex::encode(&response.app_hash);

    tracing::info!(
        app_hash,
        chain_id,
        "Platform chain initialized, initial state is created"
    );

    Ok(response)
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

    #[test]
    fn init_chain_drops_existing_block_execution_context() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        let platform_version = PlatformVersion::latest();

        // Place an existing block execution context to exercise the "drop" branch
        let existing_context = BlockExecutionContext::V0(BlockExecutionContextV0 {
            block_state_info: BlockStateInfo::V0(BlockStateInfoV0 {
                height: 5,
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
            block_platform_state: PlatformState::default_with_protocol_versions(
                platform_version.protocol_version,
                platform_version.protocol_version,
                &platform.config,
            )
            .expect("should create default platform state"),
            proposer_results: None,
        });

        app.block_execution_context
            .write()
            .unwrap()
            .replace(existing_context);

        // The block execution context is now present, init_chain should drop it
        // and then attempt to initialize the chain
        // We are not providing a valid genesis request, so this will likely fail
        // after dropping the context, but that still exercises the drop path

        let request = proto::RequestInitChain {
            time: Some(tenderdash_abci::proto::google::protobuf::Timestamp {
                seconds: 1700000000,
                nanos: 0,
            }),
            chain_id: "test-chain".to_string(),
            ..Default::default()
        };

        // The result will fail because init_chain requires valid genesis data,
        // but the important thing is that the drop path was exercised.
        // We just verify that block_execution_context was cleared.
        let _ = init_chain::<_, MockCoreRPCLike>(&app, request);

        assert!(
            app.block_execution_context.read().unwrap().is_none(),
            "expected init_chain to clear existing block execution context"
        );
    }
}
