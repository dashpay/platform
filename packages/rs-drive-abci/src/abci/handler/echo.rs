use crate::abci::app::PlatformApplication;
use crate::error::Error;
use crate::rpc::core::CoreRPCLike;
use tenderdash_abci::proto::abci as proto;

pub fn echo<A, C>(_app: &A, request: proto::RequestEcho) -> Result<proto::ResponseEcho, Error>
where
    A: PlatformApplication<C>,
    C: CoreRPCLike,
{
    Ok(proto::ResponseEcho {
        message: request.message,
    })
}
