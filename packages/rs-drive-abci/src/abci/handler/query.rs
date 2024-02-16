use crate::abci::app::PlatformApplication;
use crate::abci::handler::error::HandlerError;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use dpp::version::PlatformVersionCurrentVersion;
use tenderdash_abci::proto::abci as proto;

pub fn query<A, C>(
    app: &A,
    request: proto::RequestQuery,
) -> Result<proto::ResponseQuery, proto::ResponseException>
where
    A: PlatformApplication<C>,
    C: CoreRPCLike,
{
    let _timer = crate::metrics::abci_request_duration("query");

    let proto::RequestQuery { data, path, .. } = &request;

    let last_committed_height = app.platform().state.read().unwrap().last_committed_height() as i64;

    // TODO: It must be proto::ResponseException
    let Some(platform_version) = PlatformVersion::get_maybe_current() else {
        let handler_error = HandlerError::Unavailable("platform is not initialized".to_string());

        let response = proto::ResponseQuery {
            code: handler_error.code(),
            log: "".to_string(),
            info: handler_error.response_info()?,
            index: 0,
            key: vec![],
            value: vec![],
            proof_ops: None,
            height: last_committed_height,
            codespace: "".to_string(),
        };

        tracing::error!(?response, "platform version not initialized");

        return Ok(response);
    };

    let result = app
        .platform()
        .query(path.as_str(), data.as_slice(), platform_version)?;

    let (code, data, info) = if result.is_valid() {
        (0, result.data.unwrap_or_default(), "success".to_string())
    } else {
        let error = result
            .errors
            .first()
            .expect("validation result should have at least one error");

        let handler_error = HandlerError::from(error);

        (handler_error.code(), vec![], handler_error.response_info()?)
    };

    let response = proto::ResponseQuery {
        //todo: right now just put GRPC error codes,
        //  later we will use own error codes
        code,
        log: "".to_string(),
        info,
        index: 0,
        key: vec![],
        value: data,
        proof_ops: None,
        height: last_committed_height,
        codespace: "".to_string(),
    };

    Ok(response)
}
