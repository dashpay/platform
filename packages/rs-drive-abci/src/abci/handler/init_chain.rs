use crate::abci::app::{BlockExecutionApplication, PlatformApplication, TransactionalApplication};
use crate::error::Error;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use std::sync::Arc;
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

    let transaction_ref = app.transaction().borrow();
    let transaction = transaction_ref
        .as_ref()
        .expect("transaction must be started");

    // We need to drop the block execution context just in case init chain had already been called
    let block_context = app.block_execution_context().take(); //drop the block execution context
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
