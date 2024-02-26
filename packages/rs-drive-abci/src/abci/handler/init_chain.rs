use crate::abci::app::{PlatformApplication, TransactionalApplication};
use crate::error::Error;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use tenderdash_abci::proto::abci as proto;

pub fn init_chain<'a, A, C>(
    app: &A,
    request: proto::RequestInitChain,
) -> Result<proto::ResponseInitChain, Error>
where
    A: PlatformApplication<C> + TransactionalApplication<'a>,
    C: CoreRPCLike,
{
    app.start_transaction();

    let chain_id = request.chain_id.to_string();

    // We need to drop the block execution context just in case init chain had already been called
    let mut block_execution_context = app.platform().block_execution_context.write().unwrap();
    let block_context = block_execution_context.take(); //drop the block execution context
    if block_context.is_some() {
        tracing::warn!("block context was present during init chain, restarting");
        let protocol_version_in_consensus = app.platform().config.initial_protocol_version;
        let mut platform_state_write_guard = app.platform().state.write().unwrap();
        *platform_state_write_guard = PlatformState::default_with_protocol_versions(
            protocol_version_in_consensus,
            protocol_version_in_consensus,
        );
        drop(platform_state_write_guard);
    }
    drop(block_execution_context);

    let transaction_guard = app.transaction().read().unwrap();
    let transaction = transaction_guard.as_ref().unwrap();
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
