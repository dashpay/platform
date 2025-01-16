use tenderdash_abci::proto::abci as proto;
use crate::abci::app::{PlatformApplication};
use crate::error::Error;
use crate::rpc::core::CoreRPCLike;

pub fn finalize_snapshot<A, C>(
    app: &A,
    request: proto::RequestFinalizeSnapshot,
) -> Result<proto::ResponseFinalizeSnapshot, Error>
where
    A: PlatformApplication<C>,
    C: CoreRPCLike,
{
    //let mut new_state = PlatformState::v
    //let x = app.platform().state.load();
    let mut new_state = (*app.platform().state.load()).clone();
    Ok(Default::default())
}
