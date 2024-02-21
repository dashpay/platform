use tenderdash_abci::proto::abci as proto;

// TODO: Return error as a string
pub fn echo(request: proto::RequestEcho) -> Result<proto::ResponseEcho, proto::ResponseException> {
    Ok(proto::ResponseEcho {
        message: request.message,
    })
}
